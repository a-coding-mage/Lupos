//! linux-parity: partial
//! linux-source: vendor/linux/kernel/workqueue.c
//! test-origin: linux:vendor/linux/kernel/workqueue.c
//! Workqueue — M35.
//!
//! Mirrors `vendor/linux/kernel/workqueue.c`.  Lupos M35 ships a cooperative
//! variant: `queue_work` enqueues a `WorkStruct` onto a `Workqueue`'s pending
//! list; the worker kthread drains the list on each schedule round.  Real
//! IPI-driven preemption arrives in M37.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use core::ffi::c_char;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};

/// Linux `struct work_struct` shape (40 B in defconfig).
#[repr(C)]
pub struct WorkStruct {
    /// `data` field — pending bit + workqueue pointer in upper bits.
    pub data: AtomicU64,
    /// Linux `entry` (list_head, 16 B) — we use Vec inside Workqueue, so
    /// keep two opaque pointer slots for layout parity.
    pub entry_next: *mut WorkStruct,
    pub entry_prev: *mut WorkStruct,
    /// Callback function.
    pub func: Option<unsafe extern "C" fn(*mut WorkStruct)>,
}

const _: () = assert!(core::mem::size_of::<WorkStruct>() == 32);

unsafe impl Send for WorkStruct {}
unsafe impl Sync for WorkStruct {}

pub const WORK_PENDING: u64 = 1;

impl WorkStruct {
    pub const fn new() -> Self {
        Self {
            data: AtomicU64::new(0),
            entry_next: core::ptr::null_mut(),
            entry_prev: core::ptr::null_mut(),
            func: None,
        }
    }

    /// `INIT_WORK(&work, func)`.
    pub fn init(&mut self, func: unsafe extern "C" fn(*mut WorkStruct)) {
        self.data.store(0, Ordering::Release);
        self.entry_next = core::ptr::null_mut();
        self.entry_prev = core::ptr::null_mut();
        self.func = Some(func);
    }

    pub fn is_pending(&self) -> bool {
        self.data.load(Ordering::Acquire) & WORK_PENDING != 0
    }
}

/// `struct workqueue_struct`.
pub struct Workqueue {
    pub name: String,
    pub flags: u32,
    pub max_active: u32,
    pending: Mutex<VecDeque<*mut WorkStruct>>,
    nr_active: AtomicU64,
}

unsafe impl Send for Workqueue {}
unsafe impl Sync for Workqueue {}

pub const WQ_UNBOUND: u32 = 1 << 1;
pub const WQ_FREEZABLE: u32 = 1 << 2;
pub const WQ_MEM_RECLAIM: u32 = 1 << 3;
pub const WQ_HIGHPRI: u32 = 1 << 4;
pub const WQ_CPU_INTENSIVE: u32 = 1 << 5;

impl Workqueue {
    pub fn new(name: &str, flags: u32, max_active: u32) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            flags,
            max_active: if max_active == 0 { 256 } else { max_active },
            pending: Mutex::new(VecDeque::new()),
            nr_active: AtomicU64::new(0),
        })
    }

    pub fn nr_pending(&self) -> usize {
        self.pending.lock().len()
    }
}

/// `alloc_workqueue(name, flags, max_active)`.
pub fn alloc_workqueue(name: &str, flags: u32, max_active: u32) -> Arc<Workqueue> {
    Workqueue::new(name, flags, max_active)
}

/// `queue_work(wq, work)` — returns true if the work was newly queued.
pub fn queue_work(wq: &Arc<Workqueue>, work: *mut WorkStruct) -> bool {
    if work.is_null() {
        return false;
    }
    let was_pending =
        unsafe { (*work).data.fetch_or(WORK_PENDING, Ordering::AcqRel) } & WORK_PENDING != 0;
    if was_pending {
        return false;
    }
    wq.pending.lock().push_back(work);
    wq.nr_active.fetch_add(1, Ordering::AcqRel);
    true
}

/// `flush_workqueue(wq)` — drain every pending work item synchronously.
///
/// Lupos M35 cooperative variant: caller runs the work items on this CPU.
/// Real Linux uses a barrier work item posted to every worker pool.
pub fn flush_workqueue(wq: &Arc<Workqueue>) {
    loop {
        let work = {
            let mut q = wq.pending.lock();
            q.pop_front()
        };
        match work {
            Some(p) if !p.is_null() => unsafe {
                let was_pending =
                    (*p).data.fetch_and(!WORK_PENDING, Ordering::AcqRel) & WORK_PENDING != 0;
                if was_pending {
                    if let Some(f) = (*p).func {
                        f(p);
                    }
                }
                wq.nr_active.fetch_sub(1, Ordering::AcqRel);
            },
            _ => return,
        }
    }
}

/// `destroy_workqueue(wq)` — drains pending work then drops the queue.
pub fn destroy_workqueue(wq: Arc<Workqueue>) {
    flush_workqueue(&wq);
    drop(wq);
}

/// System-wide workqueues lazily allocated.  The `LazyWq` wrapper avoids a
/// const-evaluation issue with `Arc::new` not being const yet.
pub struct LazyWq {
    inner: Mutex<Option<Arc<Workqueue>>>,
    name: &'static str,
    flags: u32,
}

impl LazyWq {
    pub const fn new(name: &'static str, flags: u32) -> Self {
        Self {
            inner: Mutex::new(None),
            name,
            flags,
        }
    }

    pub fn get(&self) -> Arc<Workqueue> {
        let mut g = self.inner.lock();
        if g.is_none() {
            *g = Some(Workqueue::new(self.name, self.flags, 0));
        }
        g.as_ref().unwrap().clone()
    }
}

pub static SYSTEM_WQ: LazyWq = LazyWq::new("events", 0);
pub static SYSTEM_LONG_WQ: LazyWq = LazyWq::new("events_long", 0);
pub static SYSTEM_UNBOUND_WQ: LazyWq = LazyWq::new("events_unbound", WQ_UNBOUND);
pub static SYSTEM_HIGHPRI_WQ: LazyWq = LazyWq::new("events_highpri", WQ_HIGHPRI);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "alloc_workqueue_noprof",
        linux_alloc_workqueue_noprof as usize,
        true,
    );
    export_symbol_once("destroy_workqueue", linux_destroy_workqueue as usize, true);
    export_symbol_once("queue_work_on", linux_queue_work_on as usize, true);
    export_symbol_once("flush_work", linux_flush_work as usize, true);
}

unsafe fn c_name(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::from("module-wq");
    }
    let mut len = 0usize;
    while len < 64 {
        if unsafe { *ptr.add(len) } == 0 {
            let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
            return core::str::from_utf8(bytes)
                .map(String::from)
                .unwrap_or_else(|_| String::from("module-wq"));
        }
        len += 1;
    }
    String::from("module-wq")
}

/// `alloc_workqueue`/`alloc_workqueue_noprof` - `vendor/linux/kernel/workqueue.c`.
#[unsafe(export_name = "alloc_workqueue_noprof")]
pub unsafe extern "C" fn linux_alloc_workqueue_noprof(
    fmt: *const c_char,
    flags: u32,
    max_active: i32,
) -> *mut Workqueue {
    let name = unsafe { c_name(fmt) };
    let ptr =
        Arc::into_raw(alloc_workqueue(&name, flags, max_active.max(0) as u32)) as *mut Workqueue;
    crate::log_info!(
        "workqueue",
        "alloc_workqueue_noprof: name={} flags={:#x} max_active={} ptr={:p}",
        name,
        flags,
        max_active,
        ptr
    );
    ptr
}

/// `destroy_workqueue` - `vendor/linux/kernel/workqueue.c`.
#[unsafe(export_name = "destroy_workqueue")]
pub unsafe extern "C" fn linux_destroy_workqueue(wq: *mut Workqueue) {
    if wq.is_null() {
        return;
    }
    let queue = unsafe { Arc::from_raw(wq) };
    destroy_workqueue(queue);
}

/// `queue_work_on` - `vendor/linux/kernel/workqueue.c`.
#[unsafe(export_name = "queue_work_on")]
pub unsafe extern "C" fn linux_queue_work_on(
    _cpu: i32,
    wq: *mut Workqueue,
    work: *mut WorkStruct,
) -> bool {
    if wq.is_null() {
        return false;
    }
    let queue = unsafe { Arc::from_raw(wq) };
    let queued = queue_work(&queue, work);
    let _ = Arc::into_raw(queue);
    queued
}

/// `flush_work` - `vendor/linux/kernel/workqueue.c`.
#[unsafe(export_name = "flush_work")]
pub unsafe extern "C" fn linux_flush_work(work: *mut WorkStruct) -> bool {
    if work.is_null() {
        return false;
    }
    let was_pending =
        unsafe { (*work).data.fetch_and(!WORK_PENDING, Ordering::AcqRel) } & WORK_PENDING != 0;
    if was_pending {
        if let Some(func) = unsafe { (*work).func } {
            unsafe { func(work) };
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_struct_is_32_bytes() {
        // Linux's struct work_struct is 32 bytes after recent simplifications;
        // older kernels report 40 with the `wq_barrier` slot.  Either is OK.
        assert_eq!(core::mem::size_of::<WorkStruct>(), 32);
    }

    #[test]
    fn workqueue_pending_starts_empty() {
        let wq = alloc_workqueue("test", 0, 0);
        assert_eq!(wq.nr_pending(), 0);
    }

    #[test]
    fn queue_work_fires_callback_via_flush() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_w: *mut WorkStruct) {
            FIRED.fetch_add(1, O::AcqRel);
        }

        FIRED.store(0, O::Release);
        let wq = alloc_workqueue("t", 0, 0);
        let mut w = WorkStruct::new();
        w.init(cb);
        assert!(queue_work(&wq, &mut w as *mut WorkStruct));
        // Re-queue same work returns false (already pending).
        assert!(!queue_work(&wq, &mut w as *mut WorkStruct));
        assert_eq!(wq.nr_pending(), 1);
        flush_workqueue(&wq);
        assert_eq!(FIRED.load(O::Acquire), 1);
        assert_eq!(wq.nr_pending(), 0);
    }

    #[test]
    fn linux_flush_work_skips_unqueued_work() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_w: *mut WorkStruct) {
            FIRED.fetch_add(1, O::AcqRel);
        }

        unsafe {
            FIRED.store(0, O::Release);
            let mut work = WorkStruct::new();
            work.init(cb);
            assert!(!linux_flush_work(&mut work));
            assert_eq!(FIRED.load(O::Acquire), 0);
        }
    }

    #[test]
    fn flush_workqueue_skips_canceled_work() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_w: *mut WorkStruct) {
            FIRED.fetch_add(1, O::AcqRel);
        }

        FIRED.store(0, O::Release);
        let wq = alloc_workqueue("cancel-test", 0, 0);
        let mut work = WorkStruct::new();
        work.init(cb);
        assert!(queue_work(&wq, &mut work));
        work.data.fetch_and(!WORK_PENDING, Ordering::AcqRel);
        flush_workqueue(&wq);
        assert_eq!(FIRED.load(O::Acquire), 0);
        assert_eq!(wq.nr_pending(), 0);
    }

    #[test]
    fn system_wq_is_lazy_initialised() {
        let wq1 = SYSTEM_WQ.get();
        let wq2 = SYSTEM_WQ.get();
        assert!(Arc::ptr_eq(&wq1, &wq2));
        assert_eq!(wq1.name, "events");
    }

    #[test]
    fn flag_constants_match_linux() {
        assert_eq!(WQ_UNBOUND, 2);
        assert_eq!(WQ_FREEZABLE, 4);
        assert_eq!(WQ_MEM_RECLAIM, 8);
    }

    #[test]
    fn linux_workqueue_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("alloc_workqueue_noprof"),
            Some(linux_alloc_workqueue_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("destroy_workqueue"),
            Some(linux_destroy_workqueue as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("queue_work_on"),
            Some(linux_queue_work_on as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("flush_work"),
            Some(linux_flush_work as usize)
        );
    }

    #[test]
    fn linux_workqueue_c_entrypoints_queue_and_flush_work() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_w: *mut WorkStruct) {
            FIRED.fetch_add(1, O::AcqRel);
        }

        unsafe {
            FIRED.store(0, O::Release);
            let name = b"virtblk-wq\0";
            let wq = linux_alloc_workqueue_noprof(name.as_ptr().cast(), 0, 0);
            assert!(!wq.is_null());
            let mut work = WorkStruct::new();
            work.init(cb);
            assert!(linux_queue_work_on(0, wq, &mut work));
            assert!(!linux_queue_work_on(0, wq, &mut work));
            assert!(linux_flush_work(&mut work));
            assert_eq!(FIRED.load(O::Acquire), 1);
            linux_destroy_workqueue(wq);
        }
    }
}
