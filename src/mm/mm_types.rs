//! linux-parity: partial
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Core memory management types — `mm_struct` and `vm_area_struct`.
///
/// These structures form the backbone of Linux's per-process virtual memory
/// model.  Every user-space process has one `mm_struct` which owns the PGD
/// (page table root) and a Maple Tree of `vm_area_struct` entries describing
/// contiguous regions of its address space.
///
/// ## ABI parity
///
/// Field ordering within each struct follows the Linux layout from
/// `include/linux/mm_types.h` for the current LTS kernel.  Fields that are
/// not yet implemented (file pointers, anon_vma, etc.) are present as
/// `usize` placeholders so that future milestones can fill them in without
/// changing the struct layout.
///
/// ## References
///
/// - Linux `include/linux/mm_types.h` — `struct mm_struct`, `struct vm_area_struct`
/// - Linux `kernel/fork.c` — `mm_init()`, `dup_mm()`
/// - Linux `mm/init-mm.c` — `init_mm` (the kernel's own mm_struct)
extern crate alloc;
#[cfg(test)]
extern crate std;

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU64, Ordering};

use crate::kernel::locking::RawSpinLock;
use crate::mm::list::ListHead;
use crate::mm::maple_tree::MapleTree;
use crate::mm::page::Page;
use crate::mm::rmap::AnonVma;
use crate::mm::vm_flags::VmFlags;

// Re-export for convenience.
pub use crate::mm::maple_tree::MapleTree as MmMapleTree;

pub const MMF_HAS_MDWE: u64 = 1u64 << 28;
pub const MMF_HAS_MDWE_NO_INHERIT: u64 = 1u64 << 29;

/// Per-mm reader/writer semaphore.
///
/// The count word and intrusive, stack-allocated waiter ring follow Linux's
/// rwsem slow path. Uncontended acquisition touches only `count`. Contended
/// tasks enqueue under `wait_lock`, enter `TASK_UNINTERRUPTIBLE`, and remain
/// off the run queue until an unlocker grants/wakes them. When a reader heads
/// the queue, up to 256 queued readers are granted as one phase irrespective
/// of their position among writers; writers retain their relative FIFO order.
///
/// Ref: Linux `include/linux/mm_types.h` — `mm_struct::mmap_lock`;
/// `kernel/locking/rwsem.c`.
#[repr(C)]
pub struct MmapLock {
    count: AtomicI64,
    wait_lock: RawSpinLock,
    first_waiter: UnsafeCell<*mut MmapLockWaiter>,
    /// Count scheduler/yield entries so the host regression test can
    /// distinguish a parked waiter from the old runnable retry loop. This
    /// field is absent from kernel builds and therefore does not alter the
    /// production `mmap_lock` layout.
    #[cfg(test)]
    wait_calls: AtomicU64,
}

unsafe impl Send for MmapLock {}
unsafe impl Sync for MmapLock {}

const MMAP_LOCK_WRITER: i64 = 0x1;
const MMAP_LOCK_FLAG_WAITERS: i64 = 0x2;
const MMAP_LOCK_FLAG_HANDOFF: i64 = 0x4;
const MMAP_LOCK_FLAG_READFAIL: i64 = i64::MIN;
const MMAP_LOCK_READER_BIAS: i64 = 0x100;
const MMAP_LOCK_READER_MASK: i64 = !(MMAP_LOCK_READER_BIAS - 1);
const MMAP_LOCK_LOCK_MASK: i64 = MMAP_LOCK_WRITER | MMAP_LOCK_READER_MASK;
const MMAP_LOCK_READ_FAILED_MASK: i64 =
    MMAP_LOCK_WRITER | MMAP_LOCK_FLAG_WAITERS | MMAP_LOCK_FLAG_HANDOFF | MMAP_LOCK_FLAG_READFAIL;
const MMAP_LOCK_MAX_READERS_WAKEUP: usize = 0x100;

#[derive(Clone, Copy, Eq, PartialEq)]
enum MmapLockWaiterType {
    Write,
    Read,
}

/// Linux `struct rwsem_waiter`, allocated on the blocked task's kernel stack.
///
/// `next`/`prev` form the same circular FIFO rooted at `first_waiter` that
/// Linux builds with `list_add_tail(&waiter.list, &first->list)`.
struct MmapLockWaiter {
    next: *mut MmapLockWaiter,
    prev: *mut MmapLockWaiter,
    wake_next: *mut MmapLockWaiter,
    task: *mut crate::kernel::task::TaskStruct,
    waiter_type: MmapLockWaiterType,
    granted: AtomicBool,
    wake_started: AtomicBool,
    wake_complete: AtomicBool,
    #[cfg(test)]
    host_wait: std::sync::Mutex<bool>,
    #[cfg(test)]
    host_wake: std::sync::Condvar,
}

impl MmapLockWaiter {
    fn new(waiter_type: MmapLockWaiterType) -> Self {
        Self {
            next: core::ptr::null_mut(),
            prev: core::ptr::null_mut(),
            wake_next: core::ptr::null_mut(),
            #[cfg(not(test))]
            task: unsafe { crate::kernel::sched::get_current() },
            #[cfg(test)]
            task: core::ptr::null_mut(),
            waiter_type,
            granted: AtomicBool::new(false),
            wake_started: AtomicBool::new(false),
            wake_complete: AtomicBool::new(false),
            #[cfg(test)]
            host_wait: std::sync::Mutex::new(false),
            #[cfg(test)]
            host_wake: std::sync::Condvar::new(),
        }
    }
}

enum MmapLockWakeType {
    Any,
    Readers,
}

impl MmapLock {
    pub const fn new() -> Self {
        Self {
            count: AtomicI64::new(0),
            wait_lock: RawSpinLock::new(),
            first_waiter: UnsafeCell::new(core::ptr::null_mut()),
            #[cfg(test)]
            wait_calls: AtomicU64::new(0),
        }
    }

    #[inline]
    fn lock_waiters_irqsave(&self) -> crate::kernel::locking::IrqFlags {
        let flags = crate::kernel::locking::local_irq_save();
        crate::kernel::locking::preempt_disable();
        self.wait_lock.lock();
        flags
    }

    #[inline]
    fn unlock_waiters_irqrestore(&self, flags: crate::kernel::locking::IrqFlags) {
        self.wait_lock.unlock();
        crate::kernel::locking::preempt_enable();
        crate::kernel::locking::local_irq_restore(flags);
    }

    /// Put the current waiter to sleep once. The lock acquisition entry points
    /// retain one outer preempt-disable count, so mirror Linux's
    /// `schedule_preempt_disabled()` by dropping it only around `schedule()`.
    #[inline]
    unsafe fn sleep_waiter(&self, waiter: *mut MmapLockWaiter) {
        debug_assert!(!waiter.is_null());

        #[cfg(not(test))]
        {
            if !unsafe { (*waiter).wake_started.load(Ordering::Acquire) } {
                crate::kernel::locking::preempt_enable_no_resched();
                unsafe {
                    crate::kernel::sched::schedule_with_irqs_enabled();
                }
                crate::kernel::locking::preempt_disable();
            }

            // Linux pins a granted reader's task_struct with get_task_struct()
            // until wake_up_q() has consumed it. Lupos has no task reference
            // primitive yet, so the stack waiter supplies the equivalent
            // lifetime handshake: it cannot unwind until the waker has
            // completed its final task-pointer access.
            if unsafe { (*waiter).wake_started.load(Ordering::Acquire) } {
                while !unsafe { (*waiter).wake_complete.load(Ordering::Acquire) } {
                    core::hint::spin_loop();
                }
                unsafe {
                    (*waiter).wake_complete.store(false, Ordering::Relaxed);
                    (*waiter).wake_started.store(false, Ordering::Release);
                }
            }
        }

        #[cfg(test)]
        {
            self.wait_calls.fetch_add(1, Ordering::Relaxed);
            let mut notified = unsafe { (*waiter).host_wait.lock().unwrap() };
            while !*notified {
                notified = unsafe { (*waiter).host_wake.wait(notified).unwrap() };
            }
            *notified = false;
            unsafe {
                (*waiter).wake_started.store(false, Ordering::Release);
            }
        }
    }

    #[inline]
    unsafe fn set_waiter_sleeping(waiter: *mut MmapLockWaiter) {
        #[cfg(not(test))]
        if !unsafe { (*waiter).task.is_null() } {
            unsafe {
                (*(*waiter).task).__state.store(
                    crate::kernel::task::task_state::TASK_UNINTERRUPTIBLE,
                    Ordering::SeqCst,
                );
            }
        }
    }

    #[inline]
    unsafe fn set_waiter_running(waiter: *mut MmapLockWaiter) {
        #[cfg(not(test))]
        if !unsafe { (*waiter).task.is_null() } {
            unsafe {
                (*(*waiter).task).__state.store(
                    crate::kernel::task::task_state::TASK_RUNNING,
                    Ordering::Release,
                );
            }
        }
    }

    /// Append `waiter` to the circular FIFO. Caller holds `wait_lock`.
    unsafe fn add_waiter(&self, waiter: *mut MmapLockWaiter) {
        let first = unsafe { *self.first_waiter.get() };
        if first.is_null() {
            unsafe {
                (*waiter).next = waiter;
                (*waiter).prev = waiter;
                *self.first_waiter.get() = waiter;
            }
            return;
        }

        let last = unsafe { (*first).prev };
        unsafe {
            (*waiter).next = first;
            (*waiter).prev = last;
            (*last).next = waiter;
            (*first).prev = waiter;
        }
    }

    /// Remove `waiter` from the circular FIFO. Caller holds `wait_lock`.
    unsafe fn del_waiter(&self, waiter: *mut MmapLockWaiter) {
        let next = unsafe { (*waiter).next };
        let prev = unsafe { (*waiter).prev };
        if next == waiter {
            unsafe {
                *self.first_waiter.get() = core::ptr::null_mut();
            }
        } else {
            unsafe {
                (*prev).next = next;
                (*next).prev = prev;
                if *self.first_waiter.get() == waiter {
                    *self.first_waiter.get() = next;
                }
            }
        }
        unsafe {
            (*waiter).next = waiter;
            (*waiter).prev = waiter;
        }
    }

    /// Return the next queued waiter, or null after one full FIFO traversal.
    /// Caller holds `wait_lock`.
    unsafe fn next_waiter(&self, waiter: *mut MmapLockWaiter) -> *mut MmapLockWaiter {
        let next = unsafe { (*waiter).next };
        if next == unsafe { *self.first_waiter.get() } {
            core::ptr::null_mut()
        } else {
            next
        }
    }

    /// Grant or select waiters while holding `wait_lock`, returning an
    /// intrusive wake list to consume after the lock and IRQ state are
    /// restored. No allocation occurs in either pass.
    unsafe fn mark_wake_locked(&self, wake_type: MmapLockWakeType) -> *mut MmapLockWaiter {
        let first = unsafe { *self.first_waiter.get() };
        if first.is_null() {
            return core::ptr::null_mut();
        }

        if unsafe { (*first).waiter_type == MmapLockWaiterType::Write } {
            if matches!(wake_type, MmapLockWakeType::Any)
                && self.count.load(Ordering::Acquire) & MMAP_LOCK_LOCK_MASK == 0
                && !unsafe { (*first).wake_started.swap(true, Ordering::AcqRel) }
            {
                unsafe {
                    (*first).wake_next = core::ptr::null_mut();
                }
                return first;
            }
            return core::ptr::null_mut();
        }

        if self.count.load(Ordering::Acquire) & MMAP_LOCK_WRITER != 0 {
            return core::ptr::null_mut();
        }

        let mut wake_head: *mut MmapLockWaiter = core::ptr::null_mut();
        let mut wake_tail: *mut MmapLockWaiter = core::ptr::null_mut();
        let mut woken = 0usize;
        let mut waiter = first;
        loop {
            let next = unsafe { self.next_waiter(waiter) };
            if unsafe { (*waiter).waiter_type == MmapLockWaiterType::Read } {
                unsafe {
                    self.del_waiter(waiter);
                    (*waiter).wake_next = core::ptr::null_mut();
                    if wake_head.is_null() {
                        wake_head = waiter;
                    } else {
                        (*wake_tail).wake_next = waiter;
                    }
                    wake_tail = waiter;
                }
                woken += 1;

                if woken >= MMAP_LOCK_MAX_READERS_WAKEUP {
                    break;
                }
            }

            if next.is_null() {
                break;
            }
            waiter = next;
        }

        if woken == 0 {
            return core::ptr::null_mut();
        }

        // Account for the full reader phase before publishing any grant, so a
        // woken reader cannot finish and unlock before its count is present.
        let queue_empty = unsafe { (*self.first_waiter.get()).is_null() };
        let mut count = self.count.load(Ordering::Acquire);
        loop {
            let mut new = count + (woken as i64 * MMAP_LOCK_READER_BIAS);
            new &= !MMAP_LOCK_FLAG_HANDOFF;
            if queue_empty {
                new &= !MMAP_LOCK_FLAG_WAITERS;
            } else {
                new |= MMAP_LOCK_FLAG_WAITERS;
            }
            match self
                .count
                .compare_exchange_weak(count, new, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => break,
                Err(actual) => count = actual,
            }
        }

        let mut waiter = wake_head;
        while !waiter.is_null() {
            unsafe {
                (*waiter).granted.store(true, Ordering::Release);
                (*waiter).wake_started.store(true, Ordering::Release);
                waiter = (*waiter).wake_next;
            }
        }
        wake_head
    }

    /// Wake an intrusive list after dropping `wait_lock`. `next` is loaded
    /// before the wake because the remote task may resume immediately.
    unsafe fn wake_waiters(&self, mut waiter: *mut MmapLockWaiter) {
        while !waiter.is_null() {
            let next = unsafe { (*waiter).wake_next };
            #[cfg(not(test))]
            unsafe {
                crate::kernel::sched::wake_task_normal((*waiter).task);
                (*waiter).wake_complete.store(true, Ordering::Release);
            }
            #[cfg(test)]
            {
                let mut notified = unsafe { (*waiter).host_wait.lock().unwrap() };
                *notified = true;
                unsafe {
                    (*waiter).host_wake.notify_one();
                }
            }
            waiter = next;
        }
    }

    fn wake(&self, wake_type: MmapLockWakeType) {
        let flags = self.lock_waiters_irqsave();
        let wake_list = unsafe { self.mark_wake_locked(wake_type) };
        self.unlock_waiters_irqrestore(flags);
        unsafe {
            self.wake_waiters(wake_list);
        }
    }

    pub(crate) fn read_lock(&self) {
        crate::kernel::locking::preempt_disable();
        let count = self
            .count
            .fetch_add(MMAP_LOCK_READER_BIAS, Ordering::Acquire)
            + MMAP_LOCK_READER_BIAS;
        if count & MMAP_LOCK_READ_FAILED_MASK != 0 {
            unsafe {
                self.read_lock_slow();
            }
        }
        crate::kernel::locking::preempt_enable();
    }

    unsafe fn read_lock_slow(&self) {
        let mut waiter = MmapLockWaiter::new(MmapLockWaiterType::Read);
        let waiter_ptr = &mut waiter as *mut MmapLockWaiter;

        let flags = self.lock_waiters_irqsave();
        let first = unsafe { *self.first_waiter.get() };
        if first.is_null() && self.count.load(Ordering::Acquire) & MMAP_LOCK_WRITER == 0 {
            // The writer released before this failed fast-path reader reached
            // wait_lock. Its reader bias is already present in count.
            core::sync::atomic::fence(Ordering::Acquire);
            self.unlock_waiters_irqrestore(flags);
            return;
        }

        unsafe {
            self.add_waiter(waiter_ptr);
        }
        let adjustment = if first.is_null() {
            -MMAP_LOCK_READER_BIAS + MMAP_LOCK_FLAG_WAITERS
        } else {
            -MMAP_LOCK_READER_BIAS
        };
        let count = self.count.fetch_add(adjustment, Ordering::AcqRel) + adjustment;
        unsafe {
            Self::set_waiter_sleeping(waiter_ptr);
        }

        // Close the failed-fastpath/enqueue race exactly where Linux calls
        // rwsem_cond_wake_waiter(): the previous owner may have dropped the
        // lock before WAITERS became visible and therefore skipped its wake.
        let wake_list = if count & MMAP_LOCK_WRITER != 0 {
            core::ptr::null_mut()
        } else if count & MMAP_LOCK_READER_MASK != 0 {
            unsafe { self.mark_wake_locked(MmapLockWakeType::Readers) }
        } else {
            unsafe { self.mark_wake_locked(MmapLockWakeType::Any) }
        };
        self.unlock_waiters_irqrestore(flags);
        unsafe {
            self.wake_waiters(wake_list);
        }

        loop {
            unsafe {
                self.sleep_waiter(waiter_ptr);
            }
            if waiter.granted.load(Ordering::Acquire) {
                unsafe {
                    Self::set_waiter_running(waiter_ptr);
                }
                return;
            }
            unsafe {
                Self::set_waiter_sleeping(waiter_ptr);
            }
        }
    }

    pub(crate) fn try_read_lock(&self) -> bool {
        crate::kernel::locking::preempt_disable();
        let mut count = self.count.load(Ordering::Relaxed);
        let acquired = loop {
            if count & MMAP_LOCK_READ_FAILED_MASK != 0 {
                break false;
            }
            match self.count.compare_exchange_weak(
                count,
                count + MMAP_LOCK_READER_BIAS,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => break true,
                Err(actual) => count = actual,
            }
        };
        crate::kernel::locking::preempt_enable();
        acquired
    }

    pub(crate) fn read_unlock(&self) {
        crate::kernel::locking::preempt_disable();
        let count = self
            .count
            .fetch_sub(MMAP_LOCK_READER_BIAS, Ordering::Release)
            - MMAP_LOCK_READER_BIAS;
        debug_assert!(count >= 0);
        debug_assert_eq!(count & MMAP_LOCK_WRITER, 0);
        if count & (MMAP_LOCK_LOCK_MASK | MMAP_LOCK_FLAG_WAITERS) == MMAP_LOCK_FLAG_WAITERS {
            self.wake(MmapLockWakeType::Any);
        }
        crate::kernel::locking::preempt_enable();
    }

    pub(crate) fn write_lock(&self) {
        crate::kernel::locking::preempt_disable();
        if self
            .count
            .compare_exchange(0, MMAP_LOCK_WRITER, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            unsafe {
                self.write_lock_slow();
            }
        }
        crate::kernel::locking::preempt_enable();
    }

    unsafe fn write_lock_slow(&self) {
        let mut waiter = MmapLockWaiter::new(MmapLockWaiterType::Write);
        let waiter_ptr = &mut waiter as *mut MmapLockWaiter;
        let mut queued = false;

        loop {
            let flags = self.lock_waiters_irqsave();

            if !queued {
                let first = unsafe { *self.first_waiter.get() };
                if first.is_null()
                    && self
                        .count
                        .compare_exchange(0, MMAP_LOCK_WRITER, Ordering::Acquire, Ordering::Relaxed)
                        .is_ok()
                {
                    self.unlock_waiters_irqrestore(flags);
                    return;
                }

                unsafe {
                    self.add_waiter(waiter_ptr);
                }
                self.count
                    .fetch_or(MMAP_LOCK_FLAG_WAITERS, Ordering::AcqRel);
                queued = true;
            }

            let first = unsafe { *self.first_waiter.get() };
            let count = self.count.load(Ordering::Acquire);
            if first == waiter_ptr && count & MMAP_LOCK_LOCK_MASK == 0 {
                let only_waiter = unsafe { (*waiter_ptr).next == waiter_ptr };
                let mut new = count | MMAP_LOCK_WRITER;
                new &= !MMAP_LOCK_FLAG_HANDOFF;
                if only_waiter {
                    new &= !MMAP_LOCK_FLAG_WAITERS;
                } else {
                    new |= MMAP_LOCK_FLAG_WAITERS;
                }
                if self
                    .count
                    .compare_exchange(count, new, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    unsafe {
                        self.del_waiter(waiter_ptr);
                        Self::set_waiter_running(waiter_ptr);
                    }
                    self.unlock_waiters_irqrestore(flags);
                    return;
                }
            }

            unsafe {
                Self::set_waiter_sleeping(waiter_ptr);
            }
            self.unlock_waiters_irqrestore(flags);
            unsafe {
                self.sleep_waiter(waiter_ptr);
            }
        }
    }

    pub(crate) fn try_write_lock(&self) -> bool {
        crate::kernel::locking::preempt_disable();
        let acquired = self
            .count
            .compare_exchange(0, MMAP_LOCK_WRITER, Ordering::Acquire, Ordering::Relaxed)
            .is_ok();
        crate::kernel::locking::preempt_enable();
        acquired
    }

    pub(crate) fn write_unlock(&self) {
        crate::kernel::locking::preempt_disable();
        let previous = self.count.fetch_sub(MMAP_LOCK_WRITER, Ordering::Release);
        debug_assert_ne!(previous & MMAP_LOCK_WRITER, 0);
        if previous & MMAP_LOCK_FLAG_WAITERS != 0 {
            self.wake(MmapLockWakeType::Any);
        }
        crate::kernel::locking::preempt_enable();
    }

    pub(crate) fn write_downgrade(&self) {
        crate::kernel::locking::preempt_disable();
        let previous = self
            .count
            .fetch_add(MMAP_LOCK_READER_BIAS - MMAP_LOCK_WRITER, Ordering::Release);
        debug_assert_ne!(previous & MMAP_LOCK_WRITER, 0);
        if previous & MMAP_LOCK_FLAG_WAITERS != 0 {
            self.wake(MmapLockWakeType::Readers);
        }
        crate::kernel::locking::preempt_enable();
    }

    pub(crate) fn is_locked(&self) -> bool {
        self.count.load(Ordering::Acquire) != 0
    }

    pub(crate) fn is_write_locked(&self) -> bool {
        self.count.load(Ordering::Acquire) & MMAP_LOCK_WRITER != 0
    }
}

// ---------------------------------------------------------------------------
// vm_area_struct
// ---------------------------------------------------------------------------

/// Virtual memory area descriptor.
///
/// Each VMA describes a contiguous region `[vm_start, vm_end)` of a
/// process's virtual address space with uniform protection and backing.
/// VMAs are stored in the owning `mm_struct`'s Maple Tree, keyed by their
/// address range.
///
/// ## Linux field mapping
///
/// | Lupos field         | Linux field         | Notes                      |
/// |---------------------|---------------------|----------------------------|
/// | `vm_start`          | `vm_start`          | Inclusive start             |
/// | `vm_end`            | `vm_end`            | Exclusive end               |
/// | `vm_mm`             | `vm_mm`             | Owning mm_struct            |
/// | `vm_page_prot`      | `vm_page_prot`      | PTE protection bits         |
/// | `vm_flags`          | `vm_flags`          | VM_READ, VM_WRITE, etc.    |
/// | `anon_vma`          | `anon_vma`          | Reverse-mapping anchor (M14)|
/// | `anon_vma_chain`    | `anon_vma_chain`    | AnonVmaChain list (M14)     |
/// | `vm_file`           | `vm_file`           | Placeholder (M15)           |
/// | `vm_pgoff`          | `vm_pgoff`          | File offset in pages        |
/// | `vm_ops`            | `vm_ops`            | Placeholder                 |
/// | `vm_private_data`   | `vm_private_data`   | Placeholder                 |
///
/// Ref: Linux `include/linux/mm_types.h` — `struct vm_area_struct`
#[repr(C)]
pub struct VmAreaStruct {
    // -- Address range --
    /// Start address (inclusive), page-aligned.
    pub vm_start: u64,
    /// End address (exclusive), page-aligned.
    pub vm_end: u64,

    // -- Owning mm --
    /// Pointer to the owning `mm_struct`.
    pub vm_mm: *mut MmStruct,

    // -- Protection --
    /// Page-level protection for PTEs in this VMA (pgprot_t).
    pub vm_page_prot: u64,

    /// VMA flags (VM_READ | VM_WRITE | ...).
    pub vm_flags: VmFlags,

    // -- Reverse mapping (M14) --
    /// Pointer to the `AnonVma` that owns new anonymous pages in this VMA.
    /// Null until the first anonymous page fault calls `anon_vma_prepare()`.
    pub anon_vma: *mut AnonVma,
    /// Intrusive list of `AnonVmaChain` nodes linking this VMA into one or
    /// more `AnonVma`s (one per ancestor after fork).
    /// Must be initialized with `ListHead::init` after the VMA is heap-allocated.
    pub anon_vma_chain: ListHead,

    // -- File mapping (M15+) --
    /// Backing file.  Placeholder until Milestone 15.
    pub vm_file: usize,

    /// Offset within the file, in units of pages.
    pub vm_pgoff: u64,

    // -- Operations --
    /// VMA operations table (`struct vm_operations_struct *`).
    pub vm_ops: usize,

    /// Private data for vm_ops.
    pub vm_private_data: usize,
}

// Safety: VmAreaStruct contains raw pointers but is only accessed under
// mmap_lock, so Send/Sync is safe for our single-threaded M11 use.
unsafe impl Send for VmAreaStruct {}
unsafe impl Sync for VmAreaStruct {}

impl VmAreaStruct {
    /// Create a new VMA with the given range and flags.
    pub fn new(start: u64, end: u64, flags: VmFlags) -> Self {
        VmAreaStruct {
            vm_start: start,
            vm_end: end,
            vm_mm: core::ptr::null_mut(),
            vm_page_prot: 0,
            vm_flags: flags,
            anon_vma: core::ptr::null_mut(),
            // anon_vma_chain is uninitialized (null next/prev).
            // Callers that put the VMA on the heap MUST call
            // `ListHead::init(&mut (*vma).anon_vma_chain)` before any
            // rmap function is invoked on this VMA.
            anon_vma_chain: ListHead::uninit(),
            vm_file: 0,
            vm_pgoff: 0,
            vm_ops: 0,
            vm_private_data: 0,
        }
    }

    /// Size of this VMA in bytes.
    pub fn size(&self) -> u64 {
        self.vm_end - self.vm_start
    }

    /// Check if this VMA contains the given address.
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.vm_start && addr < self.vm_end
    }
}

// ---------------------------------------------------------------------------
// mm_struct
// ---------------------------------------------------------------------------

/// Per-process memory descriptor.
///
/// Owns the page table root (PGD/PML4) and the Maple Tree of VMAs.
/// Reference-counted via `mm_users` (number of threads sharing this mm)
/// and `mm_count` (structural reference — drops to 0 triggers destruction).
///
/// ## Linux field mapping
///
/// | Lupos field     | Linux field     | Notes                          |
/// |-----------------|-----------------|--------------------------------|
/// | `mm_mt`         | `mm_mt`         | Maple tree of VMAs             |
/// | `pgd`           | `pgd`           | PML4 root pointer              |
/// | `mm_users`      | `mm_users`      | Thread count                   |
/// | `mm_count`      | `mm_count`      | Structural refcount            |
/// | `map_count`     | `map_count`     | Number of VMAs                 |
/// | `mmap_lock`     | `mmap_lock`     | Per-mm VMA reader/writer lock  |
/// | `total_vm`      | `total_vm`      | Total mapped pages             |
/// | `hiwater_rss`   | `hiwater_rss`   | High-water RSS                 |
/// | `hiwater_vm`    | `hiwater_vm`    | High-water virtual size        |
///
/// Ref: Linux `include/linux/mm_types.h` — `struct mm_struct`
#[repr(C)]
pub struct MmStruct {
    // -- VMA storage --
    /// Maple tree of `vm_area_struct` entries.
    pub mm_mt: MapleTree,

    // -- Page table root --
    /// Pointer to the PGD (PML4) page table root.
    pub pgd: usize,

    // -- Reference counting --
    /// Number of threads (users) sharing this mm.
    pub mm_users: AtomicI32,
    /// Structural reference count (0 → destroy).
    pub mm_count: AtomicI32,

    // -- VMA accounting --
    /// Number of VMAs currently in the Maple Tree.
    pub map_count: i32,

    /// Per-address-space reader/writer exclusion for VMA mutations.
    pub mmap_lock: MmapLock,

    // -- RSS and VM accounting --
    /// High-water mark for RSS (resident set size) in pages.
    pub hiwater_rss: u64,
    /// High-water mark for virtual memory size in pages.
    pub hiwater_vm: u64,
    /// Total pages mapped.
    pub total_vm: u64,
    /// Locked pages (mlock).
    pub locked_vm: u64,
    /// Pinned pages.
    pub pinned_vm: AtomicU64,
    /// Data segment pages.
    pub data_vm: u64,
    /// Executable segment pages.
    pub exec_vm: u64,
    /// Stack segment pages.
    pub stack_vm: u64,

    // -- Address range boundaries --
    /// Start of code segment.
    pub start_code: u64,
    /// End of code segment.
    pub end_code: u64,
    /// Start of data segment.
    pub start_data: u64,
    /// End of data segment.
    pub end_data: u64,
    /// Start of brk (heap base).
    pub start_brk: u64,
    /// Current brk (heap end).
    pub brk: u64,
    /// Start of stack.
    pub start_stack: u64,
    /// Start of argv.
    pub arg_start: u64,
    /// End of argv.
    pub arg_end: u64,
    /// Start of envp.
    pub env_start: u64,
    /// End of envp.
    pub env_end: u64,

    // -- Executable file (M15+) --
    /// Pointer to the executable `struct file`.  Placeholder.
    pub exe_file: usize,

    // -- MM flags --
    /// Miscellaneous flags (MMF_DUMP_*, etc.).
    pub flags: u64,
    /// Default VMA flags applied to future mappings (Linux `mm->def_flags`).
    pub def_flags: VmFlags,
}

// Safety: same as VmAreaStruct — single-threaded or mmap_lock-protected.
unsafe impl Send for MmStruct {}
// Implement Sync for MmStruct
unsafe impl Sync for MmStruct {}

impl MmStruct {
    /// Create a new, empty mm_struct.
    ///
    /// Ref: Linux `kernel/fork.c` — `mm_init()`
    pub fn new(pgd: usize) -> Self {
        MmStruct {
            mm_mt: MapleTree::new(),
            pgd,
            mm_users: AtomicI32::new(1),
            mm_count: AtomicI32::new(1),
            map_count: 0,
            mmap_lock: MmapLock::new(),
            hiwater_rss: 0,
            hiwater_vm: 0,
            total_vm: 0,
            locked_vm: 0,
            pinned_vm: AtomicU64::new(0),
            data_vm: 0,
            exec_vm: 0,
            stack_vm: 0,
            start_code: 0,
            end_code: 0,
            start_data: 0,
            end_data: 0,
            start_brk: 0,
            brk: 0,
            start_stack: 0,
            arg_start: 0,
            arg_end: 0,
            env_start: 0,
            env_end: 0,
            exe_file: 0,
            flags: 0,
            def_flags: 0,
        }
    }

    /// Increment `mm_users` (another thread starts using this mm).
    ///
    /// Ref: Linux `include/linux/sched/mm.h` — `mmget()`
    pub fn mmget(&self) {
        self.mm_users.fetch_add(1, Ordering::AcqRel);
    }

    /// Decrement `mm_users`.  Returns true if it reached 0 (last user).
    ///
    /// Ref: Linux `include/linux/sched/mm.h` — `mmput()`
    pub fn mmput(&self) -> bool {
        self.mm_users.fetch_sub(1, Ordering::AcqRel) == 1
    }

    /// Increment `mm_count` (structural reference).
    pub fn mmdrop_get(&self) {
        self.mm_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Decrement `mm_count`.  Returns true if it reached 0.
    pub fn mmdrop(&self) -> bool {
        self.mm_count.fetch_sub(1, Ordering::AcqRel) == 1
    }

    // pub fn insert_vma(&mut self, vma: VmAreaStruct) {
    //     // Placeholder implementation
    //     self.vmas.push(vma);
    // }
}

// ---------------------------------------------------------------------------
// Linux-visible mm_types.h helpers
// ---------------------------------------------------------------------------

pub fn __mk_vma_flags(flags: u64) -> VmFlags {
    flags
}

pub fn vma_flags_empty(flags: VmFlags) -> bool {
    flags == 0
}

pub unsafe fn vma_flags_set_word(vma: *mut VmAreaStruct, flags: VmFlags) {
    if !vma.is_null() {
        unsafe {
            (*vma).vm_flags |= flags;
        }
    }
}

pub unsafe fn vma_flags_clear_word(vma: *mut VmAreaStruct, flags: VmFlags) {
    if !vma.is_null() {
        unsafe {
            (*vma).vm_flags &= !flags;
        }
    }
}

pub unsafe fn vma_flags_overwrite_word(vma: *mut VmAreaStruct, flags: VmFlags) {
    if !vma.is_null() {
        unsafe {
            (*vma).vm_flags = flags;
        }
    }
}

pub unsafe fn vma_flags_overwrite_word_once(vma: *mut VmAreaStruct, flags: VmFlags) {
    unsafe { vma_flags_overwrite_word(vma, flags) };
}

pub unsafe fn vma_flags_clear_all(vma: *mut VmAreaStruct) {
    unsafe { vma_flags_overwrite_word(vma, 0) };
}

pub fn vma_flags_to_legacy(flags: VmFlags) -> u64 {
    flags
}

pub fn legacy_to_vma_flags(flags: u64) -> VmFlags {
    flags
}

pub unsafe fn __vm_flags_mod(vma: *mut VmAreaStruct, set: VmFlags, clear: VmFlags) -> VmFlags {
    if vma.is_null() {
        return 0;
    }
    unsafe {
        let old = (*vma).vm_flags;
        (*vma).vm_flags = (old | set) & !clear;
        old
    }
}

pub fn __vma_atomic_valid_flag(_flag: VmFlags) -> bool {
    true
}

pub unsafe fn __mm_flags_get_word(mm: *const MmStruct) -> u64 {
    if mm.is_null() {
        0
    } else {
        unsafe { (*mm).flags }
    }
}

pub unsafe fn __mm_flags_get_bitmap(mm: *const MmStruct) -> u64 {
    unsafe { __mm_flags_get_word(mm) }
}

pub unsafe fn __mm_flags_overwrite_word(mm: *mut MmStruct, flags: u64) {
    if !mm.is_null() {
        unsafe {
            (*mm).flags = flags;
        }
    }
}

pub unsafe fn __mm_flags_set_mask_bits_word(mm: *mut MmStruct, mask: u64, bits: u64) -> u64 {
    if mm.is_null() {
        return 0;
    }
    unsafe {
        let old = (*mm).flags;
        (*mm).flags = (old & !mask) | (bits & mask);
        old
    }
}

pub unsafe fn mm_flags_set(mm: *mut MmStruct, flags: u64) {
    if !mm.is_null() {
        unsafe {
            (*mm).flags |= flags;
        }
    }
}

pub unsafe fn mm_flags_clear(mm: *mut MmStruct, flags: u64) {
    if !mm.is_null() {
        unsafe {
            (*mm).flags &= !flags;
        }
    }
}

pub unsafe fn mm_flags_clear_all(mm: *mut MmStruct) {
    if !mm.is_null() {
        unsafe {
            (*mm).flags = 0;
        }
    }
}

pub unsafe fn mm_flags_test(mm: *const MmStruct, flags: u64) -> bool {
    !mm.is_null() && unsafe { (*mm).flags & flags != 0 }
}

pub unsafe fn mm_flags_test_and_set(mm: *mut MmStruct, flags: u64) -> bool {
    let old = unsafe { mm_flags_test(mm, flags) };
    unsafe { mm_flags_set(mm, flags) };
    old
}

pub unsafe fn mm_flags_test_and_clear(mm: *mut MmStruct, flags: u64) -> bool {
    let old = unsafe { mm_flags_test(mm, flags) };
    unsafe { mm_flags_clear(mm, flags) };
    old
}

pub fn mmf_init_legacy_flags(flags: u64) -> u64 {
    if flags & MMF_HAS_MDWE_NO_INHERIT != 0 {
        return flags & !(MMF_HAS_MDWE | MMF_HAS_MDWE_NO_INHERIT);
    }
    flags
}

pub unsafe fn anon_vma_name(_vma: *const VmAreaStruct) -> *const u8 {
    core::ptr::null()
}

pub unsafe fn anon_vma_name_alloc(_name: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub unsafe fn anon_vma_name_free(_name: *mut u8) {}

pub fn encode_page(page: *mut Page, flags: usize) -> usize {
    (page as usize) | (flags & 0x3)
}

pub fn encoded_page_ptr(encoded: usize) -> *mut Page {
    (encoded & !0x3) as *mut Page
}

pub fn encoded_page_flags(encoded: usize) -> usize {
    encoded & 0x3
}

pub fn encode_nr_pages(nr: usize) -> usize {
    nr << 2
}

pub fn encoded_nr_pages(encoded: usize) -> usize {
    encoded >> 2
}

pub unsafe fn lru_gen_init_mm(_mm: *mut MmStruct) {}

pub unsafe fn lru_gen_add_mm(_mm: *mut MmStruct) {}

pub unsafe fn lru_gen_del_mm(_mm: *mut MmStruct) {}

pub unsafe fn lru_gen_migrate_mm(_old: *mut MmStruct, _new: *mut MmStruct) {}

pub unsafe fn lru_gen_use_mm(_mm: *mut MmStruct) {}

pub unsafe fn mm_init_cid(_mm: *mut MmStruct) {}

pub unsafe fn mm_destroy_cid(_mm: *mut MmStruct) {}

pub unsafe fn mm_alloc_cid(_mm: *mut MmStruct) -> i32 {
    0
}

pub unsafe fn mm_alloc_cid_noprof(mm: *mut MmStruct) -> i32 {
    unsafe { mm_alloc_cid(mm) }
}

pub unsafe fn mm_cid_size(_mm: *const MmStruct) -> usize {
    0
}

pub unsafe fn mm_cidmask(_mm: *const MmStruct) -> *const u64 {
    core::ptr::null()
}

pub unsafe fn mm_cpumask(_mm: *const MmStruct) -> *const u64 {
    core::ptr::null()
}

pub unsafe fn mm_cpus_allowed(_mm: *const MmStruct) -> *const u64 {
    core::ptr::null()
}

pub unsafe fn mm_init_cpumask(_mm: *mut MmStruct) {}

pub unsafe fn ptdesc_pmd_pts_init(_ptdesc: *mut u8) {}

pub unsafe fn ptdesc_pmd_pts_inc(_ptdesc: *mut u8) {}

pub unsafe fn ptdesc_pmd_pts_dec(_ptdesc: *mut u8) {}

pub unsafe fn ptdesc_pmd_pts_count(_ptdesc: *const u8) -> i32 {
    0
}

pub unsafe fn ptdesc_pmd_is_shared(_ptdesc: *const u8) -> bool {
    false
}

pub unsafe fn tlb_gather_mmu_fullmm(_tlb: *mut u8, _mm: *mut MmStruct) {}

pub unsafe fn tlb_gather_mmu_vma(_tlb: *mut u8, _vma: *mut VmAreaStruct) {}

pub unsafe fn vma_iter_init(_vmi: *mut u8, _mm: *mut MmStruct, _addr: u64) {}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    // -- VmAreaStruct tests --

    #[test]
    fn vma_new_sets_fields() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, 0x3);
        assert_eq!(vma.vm_start, 0x1000);
        assert_eq!(vma.vm_end, 0x2000);
        assert_eq!(vma.vm_flags, 0x3);
        assert!(vma.vm_mm.is_null());
        assert_eq!(vma.vm_file, 0);
    }

    #[test]
    fn vma_size() {
        let vma = VmAreaStruct::new(0x1000, 0x3000, 0);
        assert_eq!(vma.size(), 0x2000);
    }

    #[test]
    fn vma_contains() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, 0);
        assert!(!vma.contains(0x0FFF));
        assert!(vma.contains(0x1000));
        assert!(vma.contains(0x1FFF));
        assert!(!vma.contains(0x2000)); // exclusive end
    }

    #[test]
    fn vma_is_repr_c() {
        // Ensure the struct has a predictable layout.
        assert!(mem::size_of::<VmAreaStruct>() > 0);
        // vm_start should be at offset 0.
        assert_eq!(mem::offset_of!(VmAreaStruct, vm_start), 0);
        // vm_end should be at offset 8.
        assert_eq!(mem::offset_of!(VmAreaStruct, vm_end), 8);
    }

    // -- MmStruct tests --

    #[test]
    fn mm_new_defaults() {
        let mm = MmStruct::new(0xDEAD_BEEF);
        assert_eq!(mm.pgd, 0xDEAD_BEEF);
        assert_eq!(mm.mm_users.load(Ordering::Relaxed), 1);
        assert_eq!(mm.mm_count.load(Ordering::Relaxed), 1);
        assert_eq!(mm.map_count, 0);
        assert!(!mm.mmap_lock.is_locked());
        assert_eq!(mm.total_vm, 0);
        assert_eq!(mm.hiwater_rss, 0);
    }

    /// A queued writer must exclude readers which arrive later. This is the
    /// ordering exercised by Linux's rwsem wake-queue tests and prevents a
    /// page-fault stream from indefinitely starving fork/mmap writers.
    ///
    /// test-origin: linux:vendor/linux/kernel/locking/rwsem.c
    #[test]
    fn mmap_lock_queued_writer_precedes_late_reader() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
        use std::thread;

        let lock = Arc::new(MmapLock::new());
        lock.read_lock();

        let writer_acquired = Arc::new(AtomicBool::new(false));
        let release_writer = Arc::new(AtomicBool::new(false));
        let writer = {
            let lock = Arc::clone(&lock);
            let writer_acquired = Arc::clone(&writer_acquired);
            let release_writer = Arc::clone(&release_writer);
            thread::spawn(move || {
                lock.write_lock();
                writer_acquired.store(true, AtomicOrdering::Release);
                while !release_writer.load(AtomicOrdering::Acquire) {
                    thread::yield_now();
                }
                lock.write_unlock();
            })
        };

        while lock.count.load(Ordering::Acquire) & MMAP_LOCK_FLAG_WAITERS == 0 {
            thread::yield_now();
        }

        let late_reader_acquired = Arc::new(AtomicBool::new(false));
        let late_reader = {
            let lock = Arc::clone(&lock);
            let late_reader_acquired = Arc::clone(&late_reader_acquired);
            thread::spawn(move || {
                lock.read_lock();
                late_reader_acquired.store(true, AtomicOrdering::Release);
                lock.read_unlock();
            })
        };

        lock.read_unlock();
        while !writer_acquired.load(AtomicOrdering::Acquire) {
            thread::yield_now();
        }
        assert!(
            !late_reader_acquired.load(AtomicOrdering::Acquire),
            "a reader which arrived after the writer bypassed it"
        );

        release_writer.store(true, AtomicOrdering::Release);
        writer.join().unwrap();
        late_reader.join().unwrap();
        assert!(late_reader_acquired.load(AtomicOrdering::Acquire));
    }

    /// Linux queues a contended rwsem waiter, changes it to a sleeping task
    /// state, and calls `schedule_preempt_disabled()` once; it cannot retry
    /// until an unlocker explicitly wakes it. A waiter which remains runnable
    /// instead repeatedly yields and retries while the lock is still held.
    ///
    /// test-origin: linux:vendor/linux/kernel/locking/rwsem.c
    #[test]
    fn mmap_lock_contended_writer_parks_until_unlock() {
        use std::sync::Arc;
        use std::thread;
        use std::time::{Duration, Instant};

        let lock = Arc::new(MmapLock::new());
        lock.read_lock();

        let writer = {
            let lock = Arc::clone(&lock);
            thread::spawn(move || {
                lock.write_lock();
                lock.write_unlock();
            })
        };

        let first_wait_deadline = Instant::now() + Duration::from_secs(1);
        while lock.wait_calls.load(Ordering::Acquire) == 0 && Instant::now() < first_wait_deadline {
            thread::yield_now();
        }
        assert_ne!(
            lock.wait_calls.load(Ordering::Acquire),
            0,
            "contended writer did not reach the mmap_lock slow path"
        );

        // A Linux waiter cannot return from schedule until the unlocker makes
        // it runnable. Give the current yield/retry implementation enough
        // time to expose a second slow-path iteration, then always release and
        // join the writer before asserting so a red run leaves no live thread.
        let retry_deadline = Instant::now() + Duration::from_millis(100);
        while lock.wait_calls.load(Ordering::Acquire) < 2 && Instant::now() < retry_deadline {
            thread::yield_now();
        }
        let waits_before_unlock = lock.wait_calls.load(Ordering::Acquire);

        lock.read_unlock();
        writer.join().unwrap();

        assert_eq!(
            waits_before_unlock, 1,
            "contended mmap_lock writer remained runnable and retried {waits_before_unlock} times"
        );
    }

    /// Linux's phase-fair wake pass admits readers irrespective of their
    /// position behind queued writers, while retaining writer order, and
    /// limits each pass to `MAX_READERS_WAKEUP`.
    ///
    /// The upstream locking selftest only covers lock ordering/lockdep, so
    /// this is a focused adaptation of `rwsem_mark_wake()`'s queue invariant.
    ///
    /// test-origin: linux:vendor/linux/kernel/locking/rwsem.c
    #[test]
    fn mmap_lock_reader_phase_skips_writers_and_honors_wake_cap() {
        use std::boxed::Box;
        use std::vec::Vec;

        let lock = MmapLock::new();
        let mut readers = (0..=MMAP_LOCK_MAX_READERS_WAKEUP)
            .map(|_| Box::new(MmapLockWaiter::new(MmapLockWaiterType::Read)))
            .collect::<Vec<_>>();
        let mut writer = Box::new(MmapLockWaiter::new(MmapLockWaiterType::Write));

        let flags = lock.lock_waiters_irqsave();
        unsafe {
            lock.add_waiter(&mut *readers[0]);
            lock.add_waiter(&mut *writer);
            for reader in readers.iter_mut().skip(1) {
                lock.add_waiter(&mut **reader);
            }
        }
        lock.count.store(MMAP_LOCK_FLAG_WAITERS, Ordering::Relaxed);

        let wake_head = unsafe { lock.mark_wake_locked(MmapLockWakeType::Readers) };
        let remaining_first = unsafe { *lock.first_waiter.get() };
        let remaining_second = unsafe { (*remaining_first).next };
        let remaining_third = unsafe { (*remaining_second).next };
        lock.unlock_waiters_irqrestore(flags);

        let granted_readers = readers
            .iter()
            .filter(|reader| reader.granted.load(Ordering::Acquire))
            .count();
        assert_eq!(
            granted_readers, MMAP_LOCK_MAX_READERS_WAKEUP,
            "reader phase did not grant the Linux MAX_READERS_WAKEUP batch"
        );
        assert_eq!(wake_head, &mut *readers[0] as *mut MmapLockWaiter);
        assert!(!writer.granted.load(Ordering::Acquire));
        assert!(
            !readers[MMAP_LOCK_MAX_READERS_WAKEUP]
                .granted
                .load(Ordering::Acquire),
            "reader wake pass exceeded Linux MAX_READERS_WAKEUP"
        );
        assert_eq!(remaining_first, &mut *writer as *mut MmapLockWaiter);
        assert_eq!(
            remaining_second,
            &mut *readers[MMAP_LOCK_MAX_READERS_WAKEUP] as *mut MmapLockWaiter
        );
        assert_eq!(remaining_third, remaining_first);
        assert_eq!(
            lock.count.load(Ordering::Acquire),
            MMAP_LOCK_FLAG_WAITERS + MMAP_LOCK_MAX_READERS_WAKEUP as i64 * MMAP_LOCK_READER_BIAS
        );
    }

    #[test]
    fn mm_get_put_refcount() {
        let mm = MmStruct::new(0);
        assert_eq!(mm.mm_users.load(Ordering::Relaxed), 1);

        mm.mmget();
        assert_eq!(mm.mm_users.load(Ordering::Relaxed), 2);

        mm.mmget();
        assert_eq!(mm.mm_users.load(Ordering::Relaxed), 3);

        assert!(!mm.mmput()); // 3 -> 2
        assert!(!mm.mmput()); // 2 -> 1
        assert!(mm.mmput()); // 1 -> 0 (last user)
    }

    #[test]
    fn mm_count_refcount() {
        let mm = MmStruct::new(0);
        mm.mmdrop_get();
        assert_eq!(mm.mm_count.load(Ordering::Relaxed), 2);
        assert!(!mm.mmdrop()); // 2 -> 1
        assert!(mm.mmdrop()); // 1 -> 0
    }

    #[test]
    fn mm_maple_tree_starts_empty() {
        let mm = MmStruct::new(0);
        assert!(mm.mm_mt.is_empty());
        assert_eq!(mm.mm_mt.count(), 0);
    }

    #[test]
    fn mm_struct_is_repr_c() {
        // mm_mt should be at offset 0.
        assert_eq!(mem::offset_of!(MmStruct, mm_mt), 0);
        // pgd follows mm_mt.
        assert!(mem::offset_of!(MmStruct, pgd) > 0);
    }

    #[test]
    fn vma_and_mm_flag_helpers_mutate_exact_words() {
        let mut vma = VmAreaStruct::new(0x1000, 0x2000, 0);
        assert!(vma_flags_empty(0));
        assert_eq!(__mk_vma_flags(0x12), 0x12);
        unsafe {
            vma_flags_set_word(&mut vma, 0b0011);
            assert_eq!(vma.vm_flags, 0b0011);
            vma_flags_clear_word(&mut vma, 0b0001);
            assert_eq!(vma.vm_flags, 0b0010);
            assert_eq!(__vm_flags_mod(&mut vma, 0b1000, 0b0010), 0b0010);
            assert_eq!(vma.vm_flags, 0b1000);
            vma_flags_overwrite_word_once(&mut vma, 0x55);
            assert_eq!(vma_flags_to_legacy(vma.vm_flags), 0x55);
            assert_eq!(legacy_to_vma_flags(0xaa), 0xaa);
            vma_flags_clear_all(&mut vma);
            assert_eq!(vma.vm_flags, 0);
        }

        let mut mm = MmStruct::new(0);
        unsafe {
            assert_eq!(__mm_flags_get_word(&mm), 0);
            mm_flags_set(&mut mm, 0b0101);
            assert!(mm_flags_test(&mm, 0b0001));
            assert!(mm_flags_test_and_set(&mut mm, 0b0001));
            assert!(!mm_flags_test_and_set(&mut mm, 0b1000));
            assert_eq!(
                __mm_flags_set_mask_bits_word(&mut mm, 0b0110, 0b0010),
                0b1101
            );
            assert_eq!(__mm_flags_get_bitmap(&mm), 0b1011);
            assert!(mm_flags_test_and_clear(&mut mm, 0b0010));
            mm_flags_clear_all(&mut mm);
            assert_eq!(mm.flags, 0);
            __mm_flags_overwrite_word(&mut mm, 0xbeef);
            assert_eq!(mmf_init_legacy_flags(mm.flags), 0xbeef);
        }
    }

    #[test]
    fn configured_disabled_inline_helpers_match_linux_shape() {
        let mut mm = MmStruct::new(0);
        let mut new_mm = MmStruct::new(0);
        let vma = VmAreaStruct::new(0x1000, 0x2000, 0);
        unsafe {
            assert!(anon_vma_name(&vma).is_null());
            assert!(anon_vma_name_alloc(b"name\0".as_ptr()).is_null());
            anon_vma_name_free(core::ptr::null_mut());
            lru_gen_init_mm(&mut mm);
            lru_gen_add_mm(&mut mm);
            lru_gen_use_mm(&mut mm);
            lru_gen_migrate_mm(&mut mm, &mut new_mm);
            lru_gen_del_mm(&mut mm);
            mm_init_cid(&mut mm);
            assert_eq!(mm_alloc_cid(&mut mm), 0);
            assert_eq!(mm_alloc_cid_noprof(&mut mm), 0);
            assert_eq!(mm_cid_size(&mm), 0);
            assert!(mm_cidmask(&mm).is_null());
            assert!(mm_cpumask(&mm).is_null());
            assert!(mm_cpus_allowed(&mm).is_null());
            mm_init_cpumask(&mut mm);
            mm_destroy_cid(&mut mm);
            tlb_gather_mmu_fullmm(core::ptr::null_mut(), &mut mm);
            tlb_gather_mmu_vma(core::ptr::null_mut(), core::ptr::null_mut());
            vma_iter_init(core::ptr::null_mut(), &mut mm, 0);
        }
    }

    #[test]
    fn encoded_page_and_ptdesc_helpers_follow_inline_contracts() {
        let mut page = Page::new();
        let encoded = encode_page(&mut page, 0b11);
        assert_eq!(encoded_page_ptr(encoded), &mut page as *mut Page);
        assert_eq!(encoded_page_flags(encoded), 0b11);
        assert_eq!(encoded_nr_pages(encode_nr_pages(7)), 7);
        assert!(__vma_atomic_valid_flag(0x3));

        let mut ptdesc_count = AtomicI32::new(99);
        unsafe {
            ptdesc_pmd_pts_init(&mut ptdesc_count as *mut AtomicI32 as *mut u8);
            assert_eq!(
                ptdesc_pmd_pts_count(&ptdesc_count as *const AtomicI32 as *const u8),
                0
            );
            assert!(!ptdesc_pmd_is_shared(
                &ptdesc_count as *const AtomicI32 as *const u8
            ));
            ptdesc_pmd_pts_inc(&mut ptdesc_count as *mut AtomicI32 as *mut u8);
            assert_eq!(
                ptdesc_pmd_pts_count(&ptdesc_count as *const AtomicI32 as *const u8),
                0
            );
            ptdesc_pmd_pts_dec(&mut ptdesc_count as *mut AtomicI32 as *mut u8);
            assert_eq!(
                ptdesc_pmd_pts_count(&ptdesc_count as *const AtomicI32 as *const u8),
                0
            );
        }
    }
}

pub static mut CURRENT_TEST_MM: *mut MmStruct = core::ptr::null_mut();
