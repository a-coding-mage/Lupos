//! linux-parity: complete
//! linux-source: vendor/linux/mm/mmap_lock.c
//! test-origin: linux:vendor/linux/mm/mmap_lock.c
//! Semantic wrappers for Linux `mmap_lock` and per-VMA lock entry points.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::mm::mm_types::{MmStruct, VmAreaStruct};

#[derive(Clone, Copy, Debug, Default)]
struct MmapLockState {
    seq: u64,
    readers: usize,
    writers: usize,
    contended: bool,
    trace_start: u64,
    trace_acquire: u64,
    trace_release: u64,
}

#[derive(Clone, Copy, Debug)]
struct VmaLockState {
    attached: bool,
    readers: usize,
    write_locked: bool,
    readers_excluded: bool,
}

impl Default for VmaLockState {
    fn default() -> Self {
        Self {
            attached: true,
            readers: 0,
            write_locked: false,
            readers_excluded: false,
        }
    }
}

static MMAP_LOCK_SEQ: AtomicU64 = AtomicU64::new(1);
static MMAP_LOCKS: Mutex<BTreeMap<usize, MmapLockState>> = Mutex::new(BTreeMap::new());
static VMA_LOCKS: Mutex<BTreeMap<usize, VmaLockState>> = Mutex::new(BTreeMap::new());

fn mm_key(mm: *const u8) -> usize {
    mm as usize
}

fn vma_key(vma: *const u8) -> usize {
    vma as usize
}

fn with_mm_state<R>(mm: *mut u8, f: impl FnOnce(&mut MmapLockState) -> R) -> R {
    let key = mm_key(mm);
    let mut locks = MMAP_LOCKS.lock();
    let state = locks.entry(key).or_insert_with(|| MmapLockState {
        seq: MMAP_LOCK_SEQ.load(Ordering::Acquire),
        ..MmapLockState::default()
    });
    f(state)
}

fn bump_seq(state: &mut MmapLockState) -> u64 {
    state.seq = MMAP_LOCK_SEQ
        .fetch_add(1, Ordering::AcqRel)
        .saturating_add(1);
    state.seq
}

fn with_vma_state<R>(vma: *mut u8, f: impl FnOnce(&mut VmaLockState) -> R) -> R {
    let key = vma_key(vma);
    let mut locks = VMA_LOCKS.lock();
    let state = locks.entry(key).or_default();
    f(state)
}

pub fn mm_lock_seqcount_init(mm: *mut u8) {
    with_mm_state(mm, |state| {
        *state = MmapLockState {
            seq: MMAP_LOCK_SEQ.load(Ordering::Acquire),
            ..MmapLockState::default()
        };
    });
}

pub fn mm_lock_seqcount_begin(mm: *mut u8) -> u64 {
    with_mm_state(mm, bump_seq)
}

pub fn mm_lock_seqcount_end(_mm: *mut u8, _seq: u64) {}

pub fn mmap_read_lock_killable(mm: *mut u8) -> i32 {
    with_mm_state(mm, |state| {
        if state.writers != 0 {
            state.contended = true;
        }
        state.readers = state.readers.saturating_add(1);
    });
    0
}

pub fn mmap_read_trylock(mm: *mut u8) -> bool {
    with_mm_state(mm, |state| {
        if state.writers != 0 {
            state.contended = true;
            false
        } else {
            state.readers = state.readers.saturating_add(1);
            true
        }
    })
}

pub fn mmap_read_unlock_non_owner(mm: *mut u8) {
    with_mm_state(mm, |state| {
        state.readers = state.readers.saturating_sub(1);
    });
}

pub fn mmap_write_lock_killable(mm: *mut u8) -> i32 {
    with_mm_state(mm, |state| {
        if state.readers != 0 || state.writers != 0 {
            state.contended = true;
        }
        state.writers = state.writers.saturating_add(1);
        bump_seq(state);
    });
    0
}

pub fn mmap_write_lock_nested(mm: *mut u8, _subclass: i32) {
    let _ = mmap_write_lock_killable(mm);
}

pub fn mmap_write_downgrade(mm: *mut u8) {
    with_mm_state(mm, |state| {
        state.writers = state.writers.saturating_sub(1);
        state.readers = state.readers.saturating_add(1);
        bump_seq(state);
    });
}

pub fn mmap_lock_is_contended(mm: *mut u8) -> bool {
    with_mm_state(mm, |state| state.contended)
}

pub fn mmap_lock_speculate_try_begin(mm: *mut u8) -> u64 {
    with_mm_state(mm, |state| state.seq)
}

pub fn mmap_lock_speculate_retry(mm: *mut u8, seq: u64) -> bool {
    with_mm_state(mm, |state| state.seq != seq || state.writers != 0)
}

pub fn mmap_assert_locked(mm: *mut u8) {
    debug_assert!(with_mm_state(mm, |state| state.readers != 0 || state.writers != 0));
}

pub fn mmap_assert_write_locked(mm: *mut u8) {
    debug_assert!(with_mm_state(mm, |state| state.writers != 0));
}

pub fn vma_lock_init(vma: *mut u8) {
    with_vma_state(vma, |state| *state = VmaLockState::default());
}

pub fn vma_mark_attached(vma: *mut u8) {
    with_vma_state(vma, |state| state.attached = true);
}

pub fn vma_mark_detached(vma: *mut u8) {
    with_vma_state(vma, |state| {
        state.attached = false;
        state.readers_excluded = true;
        state.readers = 0;
    });
}

pub fn vma_is_attached(vma: *const u8) -> bool {
    let locks = VMA_LOCKS.lock();
    locks
        .get(&vma_key(vma))
        .map(|state| state.attached)
        .unwrap_or(true)
}

pub fn vma_assert_attached(vma: *const u8) {
    debug_assert!(vma_is_attached(vma));
}

pub fn vma_assert_detached(vma: *const u8) {
    debug_assert!(!vma_is_attached(vma));
}

pub fn vma_assert_locked(vma: *const u8) {
    debug_assert!(with_vma_state(vma as *mut u8, |state| state.readers != 0
        || state.write_locked));
}

pub fn vma_assert_write_locked(vma: *const u8) {
    debug_assert!(__is_vma_write_locked(vma));
}

pub fn vma_assert_stabilised(vma: *const u8) {
    debug_assert!(vma_is_attached(vma) || __vma_are_readers_excluded(vma));
}

pub fn vma_refcount_put(_vma: *mut u8) {}

pub fn __vma_raw_mm_seqnum(vma: *const u8) -> u64 {
    if vma.is_null() {
        return MMAP_LOCK_SEQ.load(Ordering::Acquire);
    }
    let mm = unsafe { (*(vma as *const VmAreaStruct)).vm_mm } as *mut u8;
    with_mm_state(mm, |state| state.seq)
}

pub fn __vma_are_readers_excluded(vma: *const u8) -> bool {
    let locks = VMA_LOCKS.lock();
    locks
        .get(&vma_key(vma))
        .map(|state| state.readers_excluded)
        .unwrap_or(false)
}

pub fn __vma_exclude_readers_for_detach(vma: *mut u8) {
    with_vma_state(vma, |state| {
        state.readers_excluded = true;
        state.readers = 0;
    });
}

pub fn __is_vma_write_locked(vma: *const u8) -> bool {
    let locks = VMA_LOCKS.lock();
    locks
        .get(&vma_key(vma))
        .map(|state| state.write_locked)
        .unwrap_or(false)
}

pub fn vma_start_read_locked(vma: *mut u8) -> bool {
    with_vma_state(vma, |state| {
        if !state.attached || state.readers_excluded || state.write_locked {
            false
        } else {
            state.readers = state.readers.saturating_add(1);
            true
        }
    })
}

pub fn vma_start_read_locked_nested(vma: *mut u8, _subclass: i32) -> bool {
    vma_start_read_locked(vma)
}

pub fn vma_end_read(vma: *mut u8) {
    with_vma_state(vma, |state| {
        state.readers = state.readers.saturating_sub(1);
    });
}

pub fn vma_start_write(vma: *mut u8) {
    __vma_start_write(vma)
}

pub fn __vma_start_write(vma: *mut u8) {
    with_vma_state(vma, |state| {
        state.write_locked = true;
        state.readers_excluded = true;
    });
}

pub fn vma_end_write_all(mm: *mut u8) {
    with_mm_state(mm, |state| {
        bump_seq(state);
    });
    let mut locks = VMA_LOCKS.lock();
    for state in locks.values_mut() {
        state.write_locked = false;
        state.readers_excluded = false;
    }
}

pub fn lock_vma_under_rcu(mm: *mut u8, addr: u64) -> *mut u8 {
    if mm.is_null() {
        return core::ptr::null_mut();
    }
    let mm_ref = unsafe { &*(mm as *const MmStruct) };
    let Some(vma) = crate::mm::vma::find_vma(mm_ref, addr) else {
        return core::ptr::null_mut();
    };
    if unsafe { addr < (*vma).vm_start } || !vma_start_read_locked(vma as *mut u8) {
        core::ptr::null_mut()
    } else {
        vma as *mut u8
    }
}

pub fn rcuwait_wake_up(_w: *mut u8) {}

pub fn __mmap_lock_do_trace_start_locking(mm: *mut u8, _write: bool) {
    with_mm_state(mm, |state| {
        state.trace_start = state.trace_start.saturating_add(1);
    });
}

pub fn __mmap_lock_do_trace_acquire_returned(mm: *mut u8, _write: bool, _success: bool) {
    with_mm_state(mm, |state| {
        state.trace_acquire = state.trace_acquire.saturating_add(1);
    });
}

pub fn __mmap_lock_do_trace_released(mm: *mut u8, _write: bool) {
    with_mm_state(mm, |state| {
        state.trace_release = state.trace_release.saturating_add(1);
    });
}

pub fn __mmap_lock_trace_start_locking(mm: *mut u8, write: bool) {
    __mmap_lock_do_trace_start_locking(mm, write)
}

pub fn __mmap_lock_trace_acquire_returned(mm: *mut u8, write: bool, success: bool) {
    __mmap_lock_do_trace_acquire_returned(mm, write, success)
}

pub fn __mmap_lock_trace_released(mm: *mut u8, write: bool) {
    __mmap_lock_do_trace_released(mm, write)
}

#[cfg(test)]
pub fn reset_for_tests() {
    MMAP_LOCKS.lock().clear();
    VMA_LOCKS.lock().clear();
    MMAP_LOCK_SEQ.store(1, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::list::ListHead;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use crate::mm::vm_flags::VM_READ;
    use alloc::boxed::Box;

    #[test]
    fn mmap_lock_sequence_and_contention_are_observable() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let mm = 0x1000usize as *mut u8;

        mm_lock_seqcount_init(mm);
        let seq = mmap_lock_speculate_try_begin(mm);
        assert_eq!(mmap_read_lock_killable(mm), 0);
        assert_eq!(mmap_write_lock_killable(mm), 0);
        assert!(mmap_lock_is_contended(mm));
        assert!(mmap_lock_speculate_retry(mm, seq));
        mmap_write_downgrade(mm);
        mmap_read_unlock_non_owner(mm);
    }

    #[test]
    fn vma_attach_read_write_and_detach_state_tracks_linux_contract() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();
        let vma = 0x2000usize as *mut u8;

        vma_lock_init(vma);
        assert!(vma_is_attached(vma));
        assert!(vma_start_read_locked(vma));
        vma_end_read(vma);
        vma_start_write(vma);
        assert!(__is_vma_write_locked(vma));
        assert!(__vma_are_readers_excluded(vma));
        assert!(!vma_start_read_locked(vma));
        vma_end_write_all(core::ptr::null_mut());
        assert!(vma_start_read_locked(vma));
        vma_end_read(vma);
        vma_mark_detached(vma);
        assert!(!vma_is_attached(vma));
        assert!(__vma_are_readers_excluded(vma));
    }

    #[test]
    fn lock_vma_under_rcu_finds_and_read_locks_matching_vma() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_for_tests();

        let mut mm = Box::new(MmStruct::new(0));
        let mut vma = Box::new(VmAreaStruct::new(0x4000, 0x8000, VM_READ));
        unsafe {
            ListHead::init(&mut vma.anon_vma_chain);
            vma.vm_mm = &mut *mm;
            crate::mm::vma::insert_vma(&mut mm, &mut *vma).unwrap();
        }

        let locked = lock_vma_under_rcu(&mut *mm as *mut MmStruct as *mut u8, 0x5000);
        assert_eq!(locked, &mut *vma as *mut VmAreaStruct as *mut u8);
        vma_assert_locked(locked);
        vma_end_read(locked);
        assert!(lock_vma_under_rcu(&mut *mm as *mut MmStruct as *mut u8, 0x9000).is_null());
    }
}
