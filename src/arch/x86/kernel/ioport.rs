//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/ioport.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/ioport.c
//! `ioperm()` / `iopl()` syscalls and the I/O permission bitmap.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/ioport.c
//!
//! Linux maintains a 65 536-bit I/O permission bitmap per-thread. Each
//! bit corresponds to one I/O port; a *cleared* bit means "permitted",
//! a *set* bit means "denied" (matches the TSS hardware semantics).
//! `ksys_ioperm` toggles bits; `sys_iopl` switches the per-thread emul
//! level. Both bump a global sequence number that the context-switch
//! path uses to lazily re-program the TSS.

#![allow(dead_code)]

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::include::uapi::errno::{EINVAL, ENOMEM, EPERM};

// === Constants — mirror vendor/linux/arch/x86/include/asm/processor.h ===

pub const IO_BITMAP_BITS: usize = 65_536;
pub const IO_BITMAP_BYTES: usize = IO_BITMAP_BITS / 8;
pub const LONG_BITS: usize = 64;
pub const IO_BITMAP_LONGS: usize = IO_BITMAP_BYTES / (LONG_BITS / 8);

/// Per-task I/O bitmap. `bitmap` is initialised to all-ones (every port
/// denied). `max` tracks the highest *used* long-word (in bytes), and
/// `sequence` is bumped on every successful `ksys_ioperm`.
#[derive(Debug, Clone)]
pub struct IoBitmap {
    pub bitmap: alloc::vec::Vec<u64>,
    pub max: u32,
    pub sequence: u64,
}

impl IoBitmap {
    pub fn new() -> Self {
        Self {
            bitmap: alloc::vec![u64::MAX; IO_BITMAP_LONGS],
            max: 0,
            sequence: 0,
        }
    }
}

impl Default for IoBitmap {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-thread state mirroring the relevant subset of `struct thread_struct`.
#[derive(Debug, Default, Clone)]
pub struct IoportThread {
    pub io_bitmap: Option<Arc<IoBitmap>>,
    pub iopl_emul: u8,
    pub io_bitmap_active: bool,
}

/// Global `io_bitmap_sequence` — bumped on every `ksys_ioperm` so the TSS
/// reprogramming path can detect a stale per-CPU cached pointer.
pub static IO_BITMAP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Linux's `bitmap_clear(bm, from, num)` — clear `num` consecutive bits
/// starting at bit position `from`.
pub fn bitmap_clear(bm: &mut [u64], from: usize, num: usize) {
    for i in from..(from + num) {
        if i >= IO_BITMAP_BITS {
            break;
        }
        let word = i / LONG_BITS;
        let bit = i % LONG_BITS;
        bm[word] &= !(1u64 << bit);
    }
}

/// Linux's `bitmap_set(bm, from, num)` — set `num` consecutive bits.
pub fn bitmap_set(bm: &mut [u64], from: usize, num: usize) {
    for i in from..(from + num) {
        if i >= IO_BITMAP_BITS {
            break;
        }
        let word = i / LONG_BITS;
        let bit = i % LONG_BITS;
        bm[word] |= 1u64 << bit;
    }
}

/// Helper to test a single bit (1 = denied, 0 = permitted).
pub fn bitmap_test(bm: &[u64], bit: usize) -> bool {
    let word = bit / LONG_BITS;
    let off = bit % LONG_BITS;
    (bm[word] >> off) & 1 == 1
}

/// Trait seam for capability + lockdown checks (`capable(CAP_SYS_RAWIO)`,
/// `security_locked_down(LOCKDOWN_IOPORT)`).
pub trait IoPermPolicy {
    fn allow_raw_io(&self) -> bool;
}

/// Always-deny policy — useful default for tests and for unprivileged
/// task contexts.
pub struct DenyAllPolicy;
impl IoPermPolicy for DenyAllPolicy {
    fn allow_raw_io(&self) -> bool {
        false
    }
}

/// Always-allow policy — for "root with no LSM" contexts in tests.
pub struct AllowAllPolicy;
impl IoPermPolicy for AllowAllPolicy {
    fn allow_raw_io(&self) -> bool {
        true
    }
}

/// Linux's `ksys_ioperm` — toggle a contiguous I/O-port range's
/// permission. Mirrors all the edge cases:
/// - Overflow check on `from + num`.
/// - `turn_on` requires `CAP_SYS_RAWIO`.
/// - Lazy bitmap allocation: first call only allocates if turning on.
/// - "All permissions dropped" detection clears the bitmap.
/// - Updates `max` and `sequence` on success.
pub fn ksys_ioperm<P: IoPermPolicy>(
    thread: &mut IoportThread,
    policy: &P,
    from: u64,
    num: u64,
    turn_on: bool,
) -> Result<i64, i32> {
    let end = match from.checked_add(num) {
        Some(e) => e,
        None => return Err(EINVAL),
    };
    if end <= from || end > IO_BITMAP_BITS as u64 {
        return Err(EINVAL);
    }
    if turn_on && !policy.allow_raw_io() {
        return Err(EPERM);
    }

    // First-time allocation: skip if turning *off* on an empty thread.
    if thread.io_bitmap.is_none() {
        if !turn_on {
            return Ok(0);
        }
        thread.io_bitmap = Some(Arc::new(IoBitmap::new()));
    }

    // Copy-on-write if shared.
    let bitmap = thread.io_bitmap.as_mut().unwrap();
    if Arc::strong_count(bitmap) > 1 {
        let owned: IoBitmap = (**bitmap).clone();
        *bitmap = Arc::new(owned);
    }
    let iobm = Arc::get_mut(bitmap).expect("just unshared above");

    if turn_on {
        bitmap_clear(&mut iobm.bitmap, from as usize, num as usize);
    } else {
        bitmap_set(&mut iobm.bitmap, from as usize, num as usize);
    }

    // Find the highest *non-all-ones* long-word.
    let mut max_long = u32::MAX;
    for (i, w) in iobm.bitmap.iter().enumerate() {
        if *w != u64::MAX {
            max_long = i as u32;
        }
    }
    if max_long == u32::MAX {
        // All permissions dropped — release the bitmap.
        thread.io_bitmap = None;
        thread.io_bitmap_active = false;
        return Ok(0);
    }

    iobm.max = (max_long + 1) * (LONG_BITS / 8) as u32;
    iobm.sequence = IO_BITMAP_SEQUENCE.fetch_add(1, Ordering::AcqRel) + 1;
    thread.io_bitmap_active = true;
    Ok(0)
}

/// Linux's `sys_iopl(level)` — switch the per-thread emul level. Valid
/// range is 0..=3; raising the level requires `CAP_SYS_RAWIO`.
pub fn sys_iopl<P: IoPermPolicy>(
    thread: &mut IoportThread,
    policy: &P,
    level: u32,
) -> Result<i64, i32> {
    if level > 3 {
        return Err(EINVAL);
    }
    let old = thread.iopl_emul as u32;
    if level == old {
        return Ok(0);
    }
    if level > old && !policy.allow_raw_io() {
        return Err(EPERM);
    }
    thread.iopl_emul = level as u8;
    thread.io_bitmap_active = thread.iopl_emul == 3 || thread.io_bitmap.is_some();
    Ok(0)
}

/// `io_bitmap_share` — share the parent's bitmap with the child on fork.
pub fn io_bitmap_share(parent: &IoportThread, child: &mut IoportThread) {
    if let Some(bm) = parent.io_bitmap.as_ref() {
        child.io_bitmap = Some(bm.clone());
    }
    child.io_bitmap_active = true;
}

/// `io_bitmap_exit` — release on thread exit.
pub fn io_bitmap_exit(thread: &mut IoportThread) {
    thread.io_bitmap = None;
    thread.io_bitmap_active = thread.iopl_emul == 3;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_sequence() {
        IO_BITMAP_SEQUENCE.store(0, Ordering::Relaxed);
    }

    #[test]
    fn bitmap_size_constants() {
        assert_eq!(IO_BITMAP_BITS, 65_536);
        assert_eq!(IO_BITMAP_BYTES, 8192);
        assert_eq!(IO_BITMAP_LONGS, 1024);
    }

    #[test]
    fn new_bitmap_denies_all_ports() {
        let bm = IoBitmap::new();
        assert!(bitmap_test(&bm.bitmap, 0));
        assert!(bitmap_test(&bm.bitmap, IO_BITMAP_BITS - 1));
    }

    #[test]
    fn bitmap_clear_then_test_returns_false() {
        let mut bm = alloc::vec![u64::MAX; IO_BITMAP_LONGS];
        bitmap_clear(&mut bm, 0x60, 8);
        for i in 0x60..0x68 {
            assert!(!bitmap_test(&bm, i));
        }
        // Adjacent bits remain denied.
        assert!(bitmap_test(&bm, 0x5f));
        assert!(bitmap_test(&bm, 0x68));
    }

    #[test]
    fn ksys_ioperm_rejects_overflow_range() {
        reset_sequence();
        let mut t = IoportThread::default();
        let r = ksys_ioperm(&mut t, &AllowAllPolicy, u64::MAX, 1, true);
        assert_eq!(r, Err(EINVAL));
    }

    #[test]
    fn ksys_ioperm_rejects_unprivileged_turn_on() {
        reset_sequence();
        let mut t = IoportThread::default();
        let r = ksys_ioperm(&mut t, &DenyAllPolicy, 0x60, 8, true);
        assert_eq!(r, Err(EPERM));
    }

    #[test]
    fn ksys_ioperm_first_call_off_without_bitmap_is_noop() {
        reset_sequence();
        let mut t = IoportThread::default();
        let r = ksys_ioperm(&mut t, &AllowAllPolicy, 0x60, 8, false);
        assert_eq!(r, Ok(0));
        assert!(t.io_bitmap.is_none());
    }

    #[test]
    fn ksys_ioperm_grant_then_release_clears_bitmap() {
        reset_sequence();
        let mut t = IoportThread::default();
        ksys_ioperm(&mut t, &AllowAllPolicy, 0x60, 8, true).unwrap();
        assert!(t.io_bitmap.is_some());
        let seq1 = t.io_bitmap.as_ref().unwrap().sequence;

        ksys_ioperm(&mut t, &AllowAllPolicy, 0x60, 8, false).unwrap();
        // All permissions dropped — bitmap released.
        assert!(t.io_bitmap.is_none());
        assert!(!t.io_bitmap_active);
        // Sequence number bumped exactly once (on the grant).
        assert_eq!(seq1, 1);
    }

    #[test]
    fn ksys_ioperm_bumps_max_to_correct_byte_offset() {
        reset_sequence();
        let mut t = IoportThread::default();
        // Open port 0x60 (bit 96 → in word 1).
        ksys_ioperm(&mut t, &AllowAllPolicy, 0x60, 8, true).unwrap();
        let max = t.io_bitmap.as_ref().unwrap().max;
        // Highest non-full word is word 1. (max_long + 1) * 8 = 16.
        assert_eq!(max, 16);
    }

    #[test]
    fn sys_iopl_rejects_level_above_3() {
        let mut t = IoportThread::default();
        assert_eq!(sys_iopl(&mut t, &AllowAllPolicy, 4), Err(EINVAL));
    }

    #[test]
    fn sys_iopl_unprivileged_cannot_raise_level() {
        let mut t = IoportThread::default();
        assert_eq!(sys_iopl(&mut t, &DenyAllPolicy, 3), Err(EPERM));
        assert_eq!(t.iopl_emul, 0);
    }

    #[test]
    fn sys_iopl_allows_lowering_without_capability() {
        let mut t = IoportThread {
            iopl_emul: 3,
            ..Default::default()
        };
        assert_eq!(sys_iopl(&mut t, &DenyAllPolicy, 0), Ok(0));
        assert_eq!(t.iopl_emul, 0);
    }

    #[test]
    fn io_bitmap_share_clones_arc() {
        let mut parent = IoportThread::default();
        ksys_ioperm(&mut parent, &AllowAllPolicy, 0x60, 8, true).unwrap();
        let mut child = IoportThread::default();
        io_bitmap_share(&parent, &mut child);
        assert!(child.io_bitmap.is_some());
        assert!(Arc::ptr_eq(
            parent.io_bitmap.as_ref().unwrap(),
            child.io_bitmap.as_ref().unwrap()
        ));
    }

    #[test]
    fn ksys_ioperm_cow_unshares_when_shared() {
        reset_sequence();
        let mut parent = IoportThread::default();
        ksys_ioperm(&mut parent, &AllowAllPolicy, 0x60, 8, true).unwrap();
        let mut child = IoportThread::default();
        io_bitmap_share(&parent, &mut child);
        assert!(Arc::ptr_eq(
            parent.io_bitmap.as_ref().unwrap(),
            child.io_bitmap.as_ref().unwrap()
        ));
        // Child mutates: should CoW.
        ksys_ioperm(&mut child, &AllowAllPolicy, 0x300, 4, true).unwrap();
        assert!(!Arc::ptr_eq(
            parent.io_bitmap.as_ref().unwrap(),
            child.io_bitmap.as_ref().unwrap()
        ));
    }

    #[test]
    fn io_bitmap_exit_keeps_iopl_3_active() {
        let mut t = IoportThread::default();
        ksys_ioperm(&mut t, &AllowAllPolicy, 0x60, 8, true).unwrap();
        t.iopl_emul = 3;
        io_bitmap_exit(&mut t);
        assert!(t.io_bitmap.is_none());
        assert!(t.io_bitmap_active);
    }
}
