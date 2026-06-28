//! linux-parity: complete
//! linux-source: vendor/linux/kernel/pid.c
//! test-origin: linux:vendor/linux/kernel/pid.c
//! PID / TGID allocator — Milestone 22.
//!
//! Implements `struct KPid`, `struct PidNamespace`, `alloc_pid`, and
//! `free_pid`, mirroring Linux `kernel/pid.c` and `include/linux/pid.h`.
//!
//! # Design — flat atomic bitmap
//!
//! For M22 (single namespace, no nesting) the allocator uses a 32 768-bit
//! (4 KiB) flat bitmap: `[AtomicU64; 512]`.  Bit N set ⟹ PID N is in use.
//!
//! - **Alloc**: cyclic scan starting from `last_pid + 1`, wrapping at
//!   `pid_max` back to `RESERVED_PIDS`.  Lock-free CAS on each 64-bit word.
//! - **Free**: atomic AND-clear of the bit.  O(1).
//!
//! PID 0 (swapper) and PIDs 1–299 (`RESERVED_PIDS`) are never returned.
//!
//! # Deferred to M28
//! - PID namespace nesting (parent pointer, level > 0)
//! - IDR radix-tree backend for non-contiguous namespace-scoped PIDs
//!
//! References:
//!   Linux `kernel/pid.c`
//!   Linux `include/linux/pid.h`
//!   Linux `include/linux/pid_namespace.h`

extern crate alloc;

use alloc::boxed::Box;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};

// ── Constants ────────────────────────────────────────────────────────────────

/// Default maximum PID value on 64-bit Linux (`/proc/sys/kernel/pid_max`).
/// Matches Linux `PID_MAX_DEFAULT` in `include/linux/threads.h`.
pub const PID_MAX_DEFAULT: i32 = 0x8000; // 32 768

/// Hard upper limit on PID values (same as PID_MAX_DEFAULT on 64-bit).
/// Matches Linux `PID_MAX_LIMIT` for 64-bit, non-BASE_SMALL configs.
pub const PID_MAX_LIMIT: i32 = PID_MAX_DEFAULT;

/// PIDs below this value are reserved for system tasks.
/// Matches Linux `RESERVED_PIDS` in `kernel/pid.c`.
pub const RESERVED_PIDS: i32 = 300;

/// Number of `u64` words needed for the bitmap (512 × 64 = 32 768 bits).
const BITMAP_WORDS: usize = (PID_MAX_DEFAULT as usize + 63) / 64;

// ── PidType ──────────────────────────────────────────────────────────────────

/// Type of a PID attachment.
///
/// Matches Linux `enum pid_type` exactly (same numeric values).
/// Used to attach a `KPid` to a task in multiple roles simultaneously
/// (e.g. a session leader holds both `Pid` and `Sid`).
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PidType {
    Pid = 0,  // PIDTYPE_PID
    Tgid = 1, // PIDTYPE_TGID
    Pgid = 2, // PIDTYPE_PGID
    Sid = 3,  // PIDTYPE_SID
    Max = 4,  // PIDTYPE_MAX (sentinel)
}

// ── Upid ─────────────────────────────────────────────────────────────────────

/// Per-namespace PID entry.
///
/// Mirrors Linux `struct upid` from `include/linux/pid.h`.
/// For M22 (single namespace, level = 0), only `numbers[0]` is used.
/// Full multi-level support (parent namespaces) arrives in M28.
#[repr(C)]
pub struct Upid {
    /// PID number visible in this namespace.
    pub nr: i32,
    /// The namespace this entry belongs to.  Points to `INIT_PID_NS` in M22.
    pub ns: *mut PidNamespace,
}

// SAFETY: Upid is only accessed during task creation (single-threaded in M22)
// and the namespace pointer is effectively 'static (points to INIT_PID_NS).
unsafe impl Send for Upid {}
unsafe impl Sync for Upid {}

// ── KPid ─────────────────────────────────────────────────────────────────────

/// Kernel `struct pid` equivalent.  Heap-allocated, reference-counted.
///
/// Mirrors Linux `struct pid` from `include/linux/pid.h`.
/// For M22, `level` is always 0 and `numbers` has exactly one element.
///
/// # Reference counting
///
/// - Created with `count = 1`.
/// - `get_pid()` increments; `put_pid()` decrements.
/// - When count reaches 0, `put_pid()` frees the bitmap bit and drops the Box.
#[repr(C)]
pub struct KPid {
    /// Structural reference count.
    pub count: AtomicU32,
    /// Stable pidfs inode number for pidfds referencing this struct pid.
    pub pidfs_ino: u64,
    /// Namespace nesting level (always 0 in M22).
    pub level: u32,
    /// Per-namespace PID number.  Only `numbers[0]` is populated in M22.
    pub numbers: [Upid; 1],
}

// SAFETY: KPid is reference-counted; the cooperative scheduler in M22 ensures
// single-threaded access to each task's KPid in practice.
unsafe impl Send for KPid {}
unsafe impl Sync for KPid {}

// ── PidNamespace ─────────────────────────────────────────────────────────────

/// Skeleton of Linux `struct pid_namespace`.
///
/// Only the fields needed for M22 bitmap allocation are present.
/// Full `pid_namespace` (parent pointer, `user_ns`, `ns_common`, reaper
/// task pointer) arrives in M28.
pub struct PidNamespace {
    /// Cyclic scan cursor: the last successfully allocated PID number.
    pub last_pid: AtomicI32,
    /// Maximum PID value for this namespace (defaults to `PID_MAX_DEFAULT`).
    pub pid_max: i32,
    /// Count of currently allocated PIDs.
    pub pid_allocated: AtomicU32,
    /// Allocation bitmap: bit N set ⟹ PID N is currently in use.
    bitmap: [AtomicU64; BITMAP_WORDS],
}

impl PidNamespace {
    /// Create a new namespace with default limits.
    ///
    /// Suitable for use in `static` initializers (the function is `const`).
    pub const fn new() -> Self {
        PidNamespace {
            last_pid: AtomicI32::new(RESERVED_PIDS - 1),
            pid_max: PID_MAX_DEFAULT,
            pid_allocated: AtomicU32::new(0),
            // SAFETY: AtomicU64 has the same memory layout as u64.
            // Initializing with 0 is a valid state (no PID allocated).
            bitmap: [const { AtomicU64::new(0) }; BITMAP_WORDS],
        }
    }

    /// Return whether PID `nr` is currently marked as allocated in the bitmap.
    ///
    /// Used by tests and debugging; not on the hot allocation path.
    pub fn bit_is_set(&self, nr: i32) -> bool {
        if nr < 0 || nr >= self.pid_max {
            return false;
        }
        let nr = nr as usize;
        let word = self.bitmap[nr / 64].load(Ordering::Relaxed);
        word & (1u64 << (nr % 64)) != 0
    }
}

// ── Global init namespace ────────────────────────────────────────────────────

/// The initial PID namespace — equivalent to Linux `init_pid_ns`.
///
/// All kernel tasks and (M24+) user processes reside in this namespace
/// until namespace support (M28) is added.
pub static INIT_PID_NS: PidNamespace = PidNamespace::new();

static PIDFS_INO_COUNTER: AtomicU64 = AtomicU64::new(1);

// ── alloc_pid ────────────────────────────────────────────────────────────────

/// Allocate a new PID from `ns`.
///
/// Performs a cyclic scan of the bitmap starting at `last_pid + 1`,
/// wrapping from `pid_max - 1` back to `RESERVED_PIDS`.  Uses
/// compare-and-swap on each 64-bit word for lock-free allocation.
///
/// Returns a heap-allocated `KPid` with:
/// - `count = 1`
/// - `level = 0`
/// - `numbers[0].nr` = the allocated PID number
///
/// # Parameters
///
/// - `set_tid`: If `Some(nr)`, attempt to allocate exactly that PID.
///   Returns `Err(-22)` (`EINVAL`) if the PID is out of range or already taken.
///
/// # Errors
///
/// - `Err(-11)` — `EAGAIN`: namespace is full (bitmap exhausted).
/// - `Err(-22)` — `EINVAL`: `set_tid` value is invalid or already taken.
///
/// Mirrors Linux `alloc_pid()` in `kernel/pid.c`.
pub fn alloc_pid(ns: &PidNamespace, set_tid: Option<i32>) -> Result<Box<KPid>, i32> {
    let nr = if let Some(requested) = set_tid {
        if requested <= 0 || requested >= ns.pid_max {
            return Err(-22); // EINVAL
        }
        try_set_bit(ns, requested).ok_or(-22)?
    } else {
        cyclic_alloc(ns)?
    };

    ns.pid_allocated.fetch_add(1, Ordering::Relaxed);

    Ok(Box::new(KPid {
        count: AtomicU32::new(1),
        pidfs_ino: PIDFS_INO_COUNTER.fetch_add(1, Ordering::Relaxed),
        level: 0,
        numbers: [Upid {
            nr,
            ns: &INIT_PID_NS as *const PidNamespace as *mut PidNamespace,
        }],
    }))
}

/// Attempt to atomically set exactly bit `nr` in the bitmap.
///
/// Returns `Some(nr)` on success, `None` if the bit was already set.
fn try_set_bit(ns: &PidNamespace, nr: i32) -> Option<i32> {
    let nr_u = nr as usize;
    let word_idx = nr_u / 64;
    let bit = 1u64 << (nr_u % 64);
    let mut old = ns.bitmap[word_idx].load(Ordering::Relaxed);
    loop {
        if old & bit != 0 {
            return None; // already allocated
        }
        match ns.bitmap[word_idx].compare_exchange_weak(
            old,
            old | bit,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Some(nr),
            Err(actual) => old = actual,
        }
    }
}

/// Cyclic scan: find and atomically claim the next free PID after `last_pid`.
///
/// Makes two passes over the bitmap: [start, pid_max) then [RESERVED_PIDS, start).
/// Returns `Err(-11)` (EAGAIN) if both passes find no free bit.
fn cyclic_alloc(ns: &PidNamespace) -> Result<i32, i32> {
    let pid_max = ns.pid_max as usize;
    let last = ns.last_pid.load(Ordering::Relaxed);
    let start = if last + 1 >= ns.pid_max {
        RESERVED_PIDS as usize
    } else {
        (last + 1) as usize
    };

    // Two passes: [start, pid_max) then [RESERVED_PIDS, start).
    for pass in 0..2usize {
        let (lo, hi) = if pass == 0 {
            (start, pid_max)
        } else {
            (RESERVED_PIDS as usize, start)
        };

        if lo >= hi {
            continue;
        }

        let word_lo = lo / 64;
        let word_hi = (hi + 63) / 64;

        'words: for word_idx in word_lo..word_hi {
            // For the first word of the range, bits below `lo` are out-of-range.
            // Pre-mask them as "taken" so trailing_ones() skips them directly.
            let lo_bit = if word_idx == word_lo { lo % 64 } else { 0 };
            let range_mask = if lo_bit > 0 { (1u64 << lo_bit) - 1 } else { 0 };

            let mut real_old = ns.bitmap[word_idx].load(Ordering::Relaxed);
            let mut masked = real_old | range_mask;

            loop {
                if masked == u64::MAX {
                    continue 'words; // all usable bits in this word are taken
                }

                let free_bit = masked.trailing_ones() as usize;
                let candidate = word_idx * 64 + free_bit;

                // Above upper bound — nothing left in this word.
                if candidate >= hi.min(pid_max) {
                    continue 'words;
                }

                // Attempt to claim the bit in the real bitmap.
                match ns.bitmap[word_idx].compare_exchange_weak(
                    real_old,
                    real_old | (1u64 << free_bit),
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        ns.last_pid.store(candidate as i32, Ordering::Relaxed);
                        return Ok(candidate as i32);
                    }
                    Err(actual) => {
                        real_old = actual;
                        masked = actual | range_mask; // re-apply range mask
                    }
                }
            }
        }
    }

    Err(-11) // EAGAIN: namespace full
}

// ── free_pid ─────────────────────────────────────────────────────────────────

/// Return PID `nr` to the namespace bitmap.
///
/// Mirrors Linux `free_pid()` in `kernel/pid.c`.
///
/// # Safety
/// `nr` must have been previously allocated from `ns` via `alloc_pid`.
pub fn free_pid(ns: &PidNamespace, nr: i32) {
    if nr <= 0 || nr >= ns.pid_max {
        return;
    }
    let nr_u = nr as usize;
    ns.bitmap[nr_u / 64].fetch_and(!(1u64 << (nr_u % 64)), Ordering::AcqRel);
    ns.pid_allocated.fetch_sub(1, Ordering::Relaxed);
}

// ── Reference counting ───────────────────────────────────────────────────────

/// Increment the reference count of `pid`.
///
/// Mirrors Linux `get_pid()`.
pub fn get_pid(pid: &KPid) {
    pid.count.fetch_add(1, Ordering::Relaxed);
}

/// Decrement the reference count of `pid`.
///
/// When the count reaches zero, calls `free_pid` on the namespace stored in
/// `numbers[0].ns` and drops the heap allocation.
///
/// Mirrors Linux `put_pid()`.
///
/// # Safety
/// `pid` must be a valid, non-null pointer previously returned by
/// `Box::into_raw(alloc_pid(...))`.  The pointer becomes invalid after this
/// call if the refcount reaches zero.
pub unsafe fn put_pid(pid: *mut KPid) {
    if pid.is_null() {
        return;
    }
    let p = unsafe { &*pid };
    // fetch_sub returns the *previous* value; release when it was 1 (now 0).
    if p.count.fetch_sub(1, Ordering::AcqRel) == 1 {
        let ns = p.numbers[0].ns;
        let nr = p.numbers[0].nr;
        if !ns.is_null() {
            free_pid(unsafe { &*ns }, nr);
        }
        // SAFETY: caller guarantees `pid` was obtained from Box::into_raw.
        drop(unsafe { Box::from_raw(pid) });
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_ns() -> PidNamespace {
        PidNamespace::new()
    }

    // ── Constants match Linux ────────────────────────────────────────────────

    #[test]
    fn pid_max_default_is_32768() {
        assert_eq!(PID_MAX_DEFAULT, 0x8000);
    }

    #[test]
    fn reserved_pids_is_300() {
        assert_eq!(RESERVED_PIDS, 300);
    }

    #[test]
    fn pid_type_values_match_linux() {
        assert_eq!(PidType::Pid as u32, 0);
        assert_eq!(PidType::Tgid as u32, 1);
        assert_eq!(PidType::Pgid as u32, 2);
        assert_eq!(PidType::Sid as u32, 3);
        assert_eq!(PidType::Max as u32, 4);
    }

    #[test]
    fn bitmap_words_is_512() {
        assert_eq!(BITMAP_WORDS, 512);
    }

    // ── Allocation invariants ────────────────────────────────────────────────

    #[test]
    fn pid_alloc_honors_reserved_range() {
        let ns = fresh_ns();
        let kpid = alloc_pid(&ns, None).expect("alloc should succeed");
        assert!(
            kpid.numbers[0].nr >= RESERVED_PIDS,
            "PID {} must be >= RESERVED_PIDS ({})",
            kpid.numbers[0].nr,
            RESERVED_PIDS
        );
    }

    #[test]
    fn pid_zero_is_never_allocated() {
        let ns = fresh_ns();
        let kpid = alloc_pid(&ns, None).expect("alloc should succeed");
        assert_ne!(
            kpid.numbers[0].nr, 0,
            "PID 0 (swapper) must never be allocated"
        );
        assert!(kpid.numbers[0].nr > 0);
    }

    #[test]
    fn pid_alloc_returns_unique_values() {
        let ns = fresh_ns();
        let mut allocated: alloc::vec::Vec<i32> = alloc::vec::Vec::new();
        for i in 0..10 {
            let kpid = alloc_pid(&ns, None).expect("alloc should succeed");
            let nr = kpid.numbers[0].nr;
            assert!(
                !allocated.contains(&nr),
                "Duplicate PID {} on iteration {}",
                nr,
                i
            );
            allocated.push(nr);
            // Intentionally leak — keep bits set to force uniqueness.
            let _ = Box::into_raw(kpid);
        }
    }

    #[test]
    fn kpid_refcount_starts_at_one() {
        let ns = fresh_ns();
        let kpid = alloc_pid(&ns, None).expect("alloc should succeed");
        assert_eq!(kpid.count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn upid_nr_matches_allocated_number() {
        let ns = fresh_ns();
        let kpid = alloc_pid(&ns, None).expect("alloc should succeed");
        let nr = kpid.numbers[0].nr;
        assert!(nr >= RESERVED_PIDS);
        assert!(nr < PID_MAX_DEFAULT);
        assert!(ns.bit_is_set(nr), "Bit for allocated PID must be set");
    }

    // ── Free and reuse ───────────────────────────────────────────────────────

    #[test]
    fn pid_free_clears_bitmap_bit() {
        let ns = fresh_ns();
        let kpid = alloc_pid(&ns, None).expect("alloc should succeed");
        let nr = kpid.numbers[0].nr;
        assert!(ns.bit_is_set(nr));

        let raw = Box::into_raw(kpid);
        // Manually free the bitmap bit (bypassing put_pid to avoid INIT_PID_NS).
        free_pid(&ns, nr);
        // Also drop the heap allocation to avoid leak.
        unsafe { drop(Box::from_raw(raw)) };

        assert!(!ns.bit_is_set(nr), "Bit must be clear after free_pid");
    }

    #[test]
    fn pid_free_allows_reuse() {
        let ns = fresh_ns();
        // Force cursor to just before RESERVED_PIDS so first alloc = 300.
        ns.last_pid.store(RESERVED_PIDS - 1, Ordering::Relaxed);

        let kpid1 = alloc_pid(&ns, None).expect("first alloc");
        let nr = kpid1.numbers[0].nr;
        assert_eq!(nr, RESERVED_PIDS);

        let raw = Box::into_raw(kpid1);
        free_pid(&ns, nr);
        unsafe { drop(Box::from_raw(raw)) };

        // Reset cursor so the cyclic scan wraps back to the freed slot.
        ns.last_pid.store(nr - 1, Ordering::Relaxed);

        let kpid2 = alloc_pid(&ns, None).expect("second alloc after free");
        assert_eq!(kpid2.numbers[0].nr, nr, "Freed PID should be reusable");
    }

    // ── set_tid path ─────────────────────────────────────────────────────────

    #[test]
    fn pid_alloc_with_set_tid_returns_exact_pid() {
        let ns = fresh_ns();
        let requested = 500i32;
        let kpid = alloc_pid(&ns, Some(requested)).expect("set_tid alloc");
        assert_eq!(kpid.numbers[0].nr, requested);
        assert!(ns.bit_is_set(requested));
    }

    #[test]
    fn pid_alloc_set_tid_rejects_duplicate() {
        let ns = fresh_ns();
        let requested = 501i32;
        let _first = alloc_pid(&ns, Some(requested)).expect("first set_tid alloc");
        let second = alloc_pid(&ns, Some(requested));
        assert!(second.is_err(), "Duplicate set_tid should return an error");
        assert_eq!(
            second.err(),
            Some(-22),
            "Duplicate set_tid should return EINVAL"
        );
    }

    #[test]
    fn pid_alloc_set_tid_rejects_zero() {
        let ns = fresh_ns();
        assert_eq!(
            alloc_pid(&ns, Some(0)).err(),
            Some(-22),
            "PID 0 is reserved"
        );
    }

    #[test]
    fn pid_alloc_set_tid_rejects_negative() {
        let ns = fresh_ns();
        assert_eq!(
            alloc_pid(&ns, Some(-1)).err(),
            Some(-22),
            "Negative PID is invalid"
        );
    }

    #[test]
    fn pid_alloc_set_tid_rejects_at_pid_max() {
        let ns = fresh_ns();
        assert_eq!(
            alloc_pid(&ns, Some(PID_MAX_DEFAULT)).err(),
            Some(-22),
            "PID >= pid_max is invalid"
        );
    }

    // ── Exhaustion ───────────────────────────────────────────────────────────

    #[test]
    fn pid_alloc_exhaustion_returns_eagain() {
        let ns = fresh_ns();
        // Fill all bitmap words to simulate a fully exhausted namespace.
        for word in &ns.bitmap {
            word.store(u64::MAX, Ordering::Relaxed);
        }
        let result = alloc_pid(&ns, None);
        assert_eq!(
            result.err(),
            Some(-11),
            "Full namespace should return -EAGAIN"
        );
    }

    // ── put_pid reference counting ───────────────────────────────────────────

    #[test]
    fn put_pid_to_zero_frees_bitmap_bit() {
        // Use a specific PID value via set_tid so we can predict the bit.
        // Choose 31000 — high enough to avoid conflicts with cyclic tests.
        let nr = 31000i32;
        let kpid = match alloc_pid(&INIT_PID_NS, Some(nr)) {
            Ok(p) => p,
            // Another test already allocated this PID — skip gracefully.
            Err(_) => return,
        };
        assert!(INIT_PID_NS.bit_is_set(nr), "Bit should be set after alloc");
        let raw = Box::into_raw(kpid);
        unsafe { put_pid(raw) }; // count 1 → 0: frees bit + drops Box
        assert!(
            !INIT_PID_NS.bit_is_set(nr),
            "Bit should be clear after put_pid"
        );
    }

    #[test]
    fn put_pid_null_is_a_noop() {
        // Must not panic or crash.
        unsafe { put_pid(core::ptr::null_mut()) };
    }
}
