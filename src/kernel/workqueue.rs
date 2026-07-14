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
use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::ENOMEM;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::rcu::RcuHead;
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};

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

/// Linux `struct rcu_work` embeds `struct work_struct` at offset zero.
#[repr(C)]
pub struct RcuWork {
    pub work: WorkStruct,
    pub rcu: RcuHead,
    pub wq: *mut Workqueue,
}

const _: () = assert!(core::mem::size_of::<RcuWork>() == 56);

unsafe impl Send for RcuWork {}
unsafe impl Sync for RcuWork {}

impl RcuWork {
    pub const fn new() -> Self {
        Self {
            work: WorkStruct::new(),
            rcu: RcuHead::new(),
            wq: core::ptr::null_mut(),
        }
    }
}

pub const WORK_PENDING: u64 = 1;
static CURRENT_WORK: AtomicUsize = AtomicUsize::new(0);

const LINUX_KTHREAD_WORKER_SIZE: usize = 56;
const KTHREAD_WORK_FUNC_OFFSET: usize = 16;
const KTHREAD_WORK_WORKER_OFFSET: usize = 24;
const KTHREAD_WORK_CANCELING_OFFSET: usize = 32;

type KthreadWorkFn = unsafe extern "C" fn(*mut c_void);

lazy_static! {
    static ref KTHREAD_WORKERS: Mutex<BTreeMap<usize, Vec<usize>>> = Mutex::new(BTreeMap::new());
}

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
                        let previous = CURRENT_WORK.swap(p as usize, Ordering::AcqRel);
                        f(p);
                        CURRENT_WORK.store(previous, Ordering::Release);
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
pub static SYSTEM_FREEZABLE_WQ: LazyWq = LazyWq::new("events_freezable", WQ_FREEZABLE);

/// Linux exports `system_dfl_wq` as a data symbol whose storage contains a
/// `struct workqueue_struct *`. Keep a pointer slot for module relocations.
static LINUX_SYSTEM_DFL_WQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_SYSTEM_FREEZABLE_WQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_SYSTEM_POWER_EFFICIENT_WQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_SYSTEM_FREEZABLE_POWER_EFFICIENT_WQ: AtomicUsize = AtomicUsize::new(0);

fn init_system_wq_slot(slot: &AtomicUsize, queue: Arc<Workqueue>) {
    if slot.load(Ordering::Acquire) == 0 {
        let ptr = Arc::into_raw(queue) as usize;
        match slot.compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {}
            Err(_) => unsafe {
                let _ = Arc::from_raw(ptr as *const Workqueue);
            },
        }
    }
}

fn init_system_workqueue_exports() {
    init_system_wq_slot(&LINUX_SYSTEM_DFL_WQ, SYSTEM_UNBOUND_WQ.get());
    init_system_wq_slot(&LINUX_SYSTEM_FREEZABLE_WQ, SYSTEM_FREEZABLE_WQ.get());
    init_system_wq_slot(&LINUX_SYSTEM_POWER_EFFICIENT_WQ, SYSTEM_WQ.get());
    init_system_wq_slot(
        &LINUX_SYSTEM_FREEZABLE_POWER_EFFICIENT_WQ,
        SYSTEM_FREEZABLE_WQ.get(),
    );
}

/// Run pending work from the Linux system workqueues in cooperative task
/// context.  Vendor drivers queue onto these through `schedule_work()` and
/// `queue_work_on()`; until worker kthreads are available, syscall wait and
/// module-load boundaries provide the same safe, lock-free execution point.
pub fn drain_system_workqueues() {
    for queue in [
        SYSTEM_WQ.get(),
        SYSTEM_LONG_WQ.get(),
        SYSTEM_UNBOUND_WQ.get(),
        SYSTEM_HIGHPRI_WQ.get(),
        SYSTEM_FREEZABLE_WQ.get(),
    ] {
        flush_workqueue(&queue);
    }
    drain_kthread_workers();
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    init_system_workqueue_exports();

    export_symbol_once(
        "alloc_workqueue_noprof",
        linux_alloc_workqueue_noprof as usize,
        true,
    );
    export_symbol_once(
        "system_dfl_wq",
        core::ptr::addr_of!(LINUX_SYSTEM_DFL_WQ) as usize,
        true,
    );
    export_symbol_once(
        "system_freezable_wq",
        core::ptr::addr_of!(LINUX_SYSTEM_FREEZABLE_WQ) as usize,
        true,
    );
    export_symbol_once(
        "system_power_efficient_wq",
        core::ptr::addr_of!(LINUX_SYSTEM_POWER_EFFICIENT_WQ) as usize,
        true,
    );
    export_symbol_once(
        "system_freezable_power_efficient_wq",
        core::ptr::addr_of!(LINUX_SYSTEM_FREEZABLE_POWER_EFFICIENT_WQ) as usize,
        true,
    );
    export_symbol_once("destroy_workqueue", linux_destroy_workqueue as usize, true);
    export_symbol_once("queue_work_on", linux_queue_work_on as usize, true);
    export_symbol_once("queue_work_node", linux_queue_work_node as usize, true);
    export_symbol_once("queue_rcu_work", linux_queue_rcu_work as usize, false);
    export_symbol_once(
        "mod_delayed_work_on",
        linux_mod_delayed_work_on as usize,
        true,
    );
    export_symbol_once("drain_workqueue", linux_drain_workqueue as usize, true);
    export_symbol_once("flush_work", linux_flush_work as usize, true);
    export_symbol_once(
        "flush_delayed_work",
        linux_flush_delayed_work as usize,
        false,
    );
    export_symbol_once("flush_rcu_work", linux_flush_rcu_work as usize, false);
    export_symbol_once(
        "cancel_delayed_work",
        linux_cancel_delayed_work as usize,
        false,
    );
    export_symbol_once(
        "cancel_delayed_work_sync",
        linux_cancel_delayed_work_sync as usize,
        false,
    );
    export_symbol_once("cancel_work_sync", linux_cancel_work_sync as usize, true);
    export_symbol_once("current_work", linux_current_work as usize, false);
    export_symbol_once("enable_work", linux_enable_work as usize, true);
    export_symbol_once("disable_work", linux_disable_work as usize, true);
    export_symbol_once("disable_work_sync", linux_disable_work_sync as usize, true);
    export_symbol_once(
        "kthread_create_worker_on_node",
        linux_kthread_create_worker_on_node as usize,
        false,
    );
    export_symbol_once(
        "kthread_queue_work",
        linux_kthread_queue_work as usize,
        true,
    );
    export_symbol_once(
        "kthread_flush_work",
        linux_kthread_flush_work as usize,
        true,
    );
    export_symbol_once(
        "kthread_flush_worker",
        linux_kthread_flush_worker as usize,
        true,
    );
    export_symbol_once(
        "kthread_cancel_work_sync",
        linux_kthread_cancel_work_sync as usize,
        true,
    );
    export_symbol_once(
        "kthread_destroy_worker",
        linux_kthread_destroy_worker as usize,
        false,
    );
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

/// `queue_work_node` - `vendor/linux/kernel/workqueue.c:2510`.
pub unsafe extern "C" fn linux_queue_work_node(
    _node: i32,
    wq: *mut Workqueue,
    work: *mut WorkStruct,
) -> bool {
    unsafe { linux_queue_work_on(0, wq, work) }
}

/// `queue_rcu_work` - `vendor/linux/kernel/workqueue.c:2683`.
///
/// Lupos' cooperative RCU/workqueue model runs work at explicit drain points;
/// queue the embedded work item immediately while preserving Linux's
/// already-pending return value.
pub unsafe extern "C" fn linux_queue_rcu_work(wq: *mut Workqueue, rwork: *mut RcuWork) -> bool {
    if wq.is_null() || rwork.is_null() {
        return false;
    }
    let queued = unsafe { linux_queue_work_on(0, wq, core::ptr::addr_of_mut!((*rwork).work)) };
    if queued {
        unsafe {
            (*rwork).wq = wq;
        }
    }
    queued
}

/// `mod_delayed_work_on` - `vendor/linux/kernel/workqueue.c:2647`.
///
/// Lupos' cooperative workqueue runtime does not yet own a module-facing timer
/// wheel for delayed work. Match the existing delayed-work ABI behavior used by
/// storage modules: make the work runnable now, preserving Linux's return value
/// distinction for an already-pending work item.
pub unsafe extern "C" fn linux_mod_delayed_work_on(
    cpu: i32,
    wq: *mut Workqueue,
    dwork: *mut c_void,
    _delay: u64,
) -> bool {
    if dwork.is_null() {
        return false;
    }

    let work = dwork.cast::<WorkStruct>();
    let was_pending = unsafe { (*work).is_pending() };
    if was_pending {
        unsafe {
            (*work).data.fetch_and(!WORK_PENDING, Ordering::AcqRel);
        }
    }
    let _ = unsafe { linux_queue_work_on(cpu, wq, work) };
    was_pending
}

/// `drain_workqueue` - `vendor/linux/kernel/workqueue.c:4228`.
pub unsafe extern "C" fn linux_drain_workqueue(wq: *mut Workqueue) {
    if wq.is_null() {
        return;
    }
    let queue = unsafe { Arc::from_raw(wq) };
    flush_workqueue(&queue);
    let _ = Arc::into_raw(queue);
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
            let previous = CURRENT_WORK.swap(work as usize, Ordering::AcqRel);
            unsafe { func(work) };
            CURRENT_WORK.store(previous, Ordering::Release);
            return true;
        }
    }
    false
}

/// `flush_delayed_work` - `vendor/linux/kernel/workqueue.c:4411`.
///
/// Linux embeds `struct work_struct` at offset zero in `struct delayed_work`.
/// Delays are already collapsed to immediate cooperative execution in the
/// module ABI, so flushing delayed work is flushing that embedded work item.
pub unsafe extern "C" fn linux_flush_delayed_work(dwork: *mut c_void) -> bool {
    unsafe { linux_flush_work(dwork.cast::<WorkStruct>()) }
}

/// `flush_rcu_work` - `vendor/linux/kernel/workqueue.c:4429`.
pub unsafe extern "C" fn linux_flush_rcu_work(rwork: *mut RcuWork) -> bool {
    if rwork.is_null() {
        return false;
    }
    unsafe { linux_flush_work(core::ptr::addr_of_mut!((*rwork).work)) }
}

/// `current_work` - `vendor/linux/kernel/workqueue.c:6195`.
#[unsafe(export_name = "current_work")]
pub unsafe extern "C" fn linux_current_work() -> *mut WorkStruct {
    CURRENT_WORK.load(Ordering::Acquire) as *mut WorkStruct
}

/// `cancel_work_sync` - `vendor/linux/kernel/workqueue.c`.
pub unsafe extern "C" fn linux_cancel_work_sync(work: *mut WorkStruct) -> bool {
    if work.is_null() {
        return false;
    }
    unsafe { (*work).data.fetch_and(!WORK_PENDING, Ordering::AcqRel) & WORK_PENDING != 0 }
}

/// `cancel_delayed_work` - `vendor/linux/kernel/workqueue.c:4551`.
pub unsafe extern "C" fn linux_cancel_delayed_work(dwork: *mut c_void) -> bool {
    unsafe { linux_cancel_work_sync(dwork.cast::<WorkStruct>()) }
}

/// `cancel_delayed_work_sync` - `vendor/linux/kernel/workqueue.c:4566`.
pub unsafe extern "C" fn linux_cancel_delayed_work_sync(dwork: *mut c_void) -> bool {
    unsafe { linux_cancel_work_sync(dwork.cast::<WorkStruct>()) }
}

/// `enable_work` - `vendor/linux/kernel/workqueue.c:4619`.
pub unsafe extern "C" fn linux_enable_work(work: *mut WorkStruct) -> bool {
    !work.is_null()
}

/// `disable_work` - `vendor/linux/kernel/workqueue.c:4584`.
#[unsafe(export_name = "disable_work")]
pub unsafe extern "C" fn linux_disable_work(work: *mut WorkStruct) -> bool {
    unsafe { linux_cancel_work_sync(work) }
}

/// `disable_work_sync` - `vendor/linux/kernel/workqueue.c`.
#[unsafe(export_name = "disable_work_sync")]
pub unsafe extern "C" fn linux_disable_work_sync(work: *mut WorkStruct) -> bool {
    unsafe { linux_cancel_work_sync(work) }
}

#[cfg(not(test))]
fn kthread_kzalloc(size: usize) -> *mut u8 {
    unsafe { crate::mm::slab::linux___kmalloc_noprof(size, GFP_KERNEL | __GFP_ZERO) }
}

#[cfg(not(test))]
fn kthread_kfree(ptr: *mut u8) {
    unsafe { crate::mm::slab::linux_kfree(ptr) };
}

#[cfg(test)]
fn kthread_kzalloc(size: usize) -> *mut u8 {
    let layout = core::alloc::Layout::from_size_align(size + 16, 16).unwrap();
    unsafe {
        let block = alloc::alloc::alloc_zeroed(layout);
        if block.is_null() {
            return block;
        }
        *(block as *mut usize) = size;
        block.add(16)
    }
}

#[cfg(test)]
fn kthread_kfree(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let block = ptr.sub(16);
        let size = *(block as *const usize);
        let layout = core::alloc::Layout::from_size_align(size + 16, 16).unwrap();
        alloc::alloc::dealloc(block, layout);
    }
}

fn err_ptr<T>(errno: i32) -> *mut T {
    (-(errno as isize)) as *mut T
}

unsafe fn kthread_work_func(work: *mut c_void) -> Option<KthreadWorkFn> {
    let func = unsafe {
        work.cast::<u8>()
            .add(KTHREAD_WORK_FUNC_OFFSET)
            .cast::<usize>()
            .read()
    };
    (func != 0).then(|| unsafe { core::mem::transmute::<usize, KthreadWorkFn>(func) })
}

unsafe fn kthread_work_set_worker(work: *mut c_void, worker: *mut c_void) {
    unsafe {
        work.cast::<u8>()
            .add(KTHREAD_WORK_WORKER_OFFSET)
            .cast::<*mut c_void>()
            .write(worker);
    }
}

unsafe fn kthread_work_canceling(work: *mut c_void) -> i32 {
    unsafe {
        work.cast::<u8>()
            .add(KTHREAD_WORK_CANCELING_OFFSET)
            .cast::<i32>()
            .read()
    }
}

fn kthread_run_queued(worker: usize) -> usize {
    let queued = {
        let mut workers = KTHREAD_WORKERS.lock();
        workers
            .get_mut(&worker)
            .map(core::mem::take)
            .unwrap_or_default()
    };

    let mut handled = 0usize;
    for work in queued {
        unsafe {
            kthread_work_set_worker(work as *mut c_void, worker as *mut c_void);
            if let Some(func) = kthread_work_func(work as *mut c_void) {
                func(work as *mut c_void);
                handled += 1;
            }
        }
    }
    handled
}

pub fn drain_kthread_workers() -> usize {
    let workers = {
        let workers = KTHREAD_WORKERS.lock();
        workers.keys().copied().collect::<Vec<_>>()
    };
    workers.into_iter().map(kthread_run_queued).sum()
}

/// `kthread_create_worker_on_node` - `vendor/linux/kernel/kthread.c:1087`.
pub unsafe extern "C" fn linux_kthread_create_worker_on_node(
    _flags: u32,
    _node: i32,
    _namefmt: *const c_char,
    _a0: usize,
) -> *mut c_void {
    let worker = kthread_kzalloc(LINUX_KTHREAD_WORKER_SIZE);
    if worker.is_null() {
        return err_ptr(ENOMEM);
    }
    KTHREAD_WORKERS.lock().insert(worker as usize, Vec::new());
    worker.cast()
}

/// `kthread_queue_work` - `vendor/linux/kernel/kthread.c:1199`.
pub unsafe extern "C" fn linux_kthread_queue_work(worker: *mut c_void, work: *mut c_void) -> bool {
    if worker.is_null() || work.is_null() || unsafe { kthread_work_canceling(work) } != 0 {
        return false;
    }
    let mut workers = KTHREAD_WORKERS.lock();
    let Some(queue) = workers.get_mut(&(worker as usize)) else {
        return false;
    };
    if queue.contains(&(work as usize)) {
        return false;
    }
    unsafe { kthread_work_set_worker(work, worker) };
    queue.push(work as usize);
    true
}

/// `kthread_flush_work` - `vendor/linux/kernel/kthread.c:1334`.
pub unsafe extern "C" fn linux_kthread_flush_work(work: *mut c_void) {
    if work.is_null() {
        return;
    }
    let workers = {
        let workers = KTHREAD_WORKERS.lock();
        workers
            .iter()
            .filter(|(_, queue)| queue.contains(&(work as usize)))
            .map(|(&worker, _)| worker)
            .collect::<Vec<_>>()
    };
    for worker in workers {
        kthread_run_queued(worker);
    }
}

/// `kthread_flush_worker` - `vendor/linux/kernel/kthread.c:1571`.
pub unsafe extern "C" fn linux_kthread_flush_worker(worker: *mut c_void) {
    if !worker.is_null() {
        kthread_run_queued(worker as usize);
    }
}

/// `kthread_cancel_work_sync` - `vendor/linux/kernel/kthread.c:1543`.
pub unsafe extern "C" fn linux_kthread_cancel_work_sync(work: *mut c_void) -> bool {
    if work.is_null() {
        return false;
    }
    let mut cancelled = false;
    for queue in KTHREAD_WORKERS.lock().values_mut() {
        let before = queue.len();
        queue.retain(|&queued| queued != work as usize);
        cancelled |= queue.len() != before;
    }
    cancelled
}

/// `kthread_destroy_worker` - `vendor/linux/kernel/kthread.c:1595`.
pub unsafe extern "C" fn linux_kthread_destroy_worker(worker: *mut c_void) {
    if worker.is_null() {
        return;
    }
    kthread_run_queued(worker as usize);
    KTHREAD_WORKERS.lock().remove(&(worker as usize));
    kthread_kfree(worker.cast());
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
        static SEEN_CURRENT: AtomicUsize = AtomicUsize::new(0);
        unsafe extern "C" fn cb(w: *mut WorkStruct) {
            FIRED.fetch_add(1, O::AcqRel);
            SEEN_CURRENT.store(unsafe { linux_current_work() } as usize, O::Release);
            assert_eq!(unsafe { linux_current_work() }, w);
        }

        FIRED.store(0, O::Release);
        SEEN_CURRENT.store(0, O::Release);
        let wq = alloc_workqueue("t", 0, 0);
        let mut w = WorkStruct::new();
        w.init(cb);
        assert!(queue_work(&wq, &mut w as *mut WorkStruct));
        // Re-queue same work returns false (already pending).
        assert!(!queue_work(&wq, &mut w as *mut WorkStruct));
        assert_eq!(wq.nr_pending(), 1);
        flush_workqueue(&wq);
        assert_eq!(FIRED.load(O::Acquire), 1);
        assert_eq!(SEEN_CURRENT.load(O::Acquire), &mut w as *mut _ as usize);
        assert!(unsafe { linux_current_work() }.is_null());
        assert_eq!(wq.nr_pending(), 0);
    }

    #[test]
    fn queue_rcu_work_queues_embedded_work_struct() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_w: *mut WorkStruct) {
            FIRED.fetch_add(1, O::AcqRel);
        }

        FIRED.store(0, O::Release);
        let wq = alloc_workqueue("rcu-work-test", 0, 0);
        let raw_wq = Arc::into_raw(wq.clone()) as *mut Workqueue;
        let mut rwork = RcuWork::new();
        rwork.work.init(cb);

        unsafe {
            assert!(linux_queue_rcu_work(raw_wq, &mut rwork));
            assert!(!linux_queue_rcu_work(raw_wq, &mut rwork));
            assert_eq!(rwork.wq, raw_wq);
            assert_eq!(wq.nr_pending(), 1);
            assert!(linux_flush_rcu_work(&mut rwork));
            assert_eq!(FIRED.load(O::Acquire), 1);
            assert!(!linux_flush_rcu_work(&mut rwork));
            drop(Arc::from_raw(raw_wq));
        }
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
        let source = include_str!("../../vendor/linux/kernel/workqueue.c");
        assert!(source.contains("EXPORT_SYMBOL_GPL(disable_work);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(enable_work);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("alloc_workqueue_noprof"),
            Some(linux_alloc_workqueue_noprof as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("system_dfl_wq"),
            Some(core::ptr::addr_of!(LINUX_SYSTEM_DFL_WQ) as usize)
        );
        assert_ne!(LINUX_SYSTEM_DFL_WQ.load(Ordering::Acquire), 0);
        assert_eq!(
            crate::kernel::module::find_symbol("system_power_efficient_wq"),
            Some(core::ptr::addr_of!(LINUX_SYSTEM_POWER_EFFICIENT_WQ) as usize)
        );
        assert_ne!(LINUX_SYSTEM_POWER_EFFICIENT_WQ.load(Ordering::Acquire), 0);
        assert_eq!(
            crate::kernel::module::find_symbol("system_freezable_power_efficient_wq"),
            Some(core::ptr::addr_of!(LINUX_SYSTEM_FREEZABLE_POWER_EFFICIENT_WQ) as usize)
        );
        assert_ne!(
            LINUX_SYSTEM_FREEZABLE_POWER_EFFICIENT_WQ.load(Ordering::Acquire),
            0
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
            crate::kernel::module::find_symbol("queue_work_node"),
            Some(linux_queue_work_node as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("queue_rcu_work"),
            Some(linux_queue_rcu_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("mod_delayed_work_on"),
            Some(linux_mod_delayed_work_on as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("drain_workqueue"),
            Some(linux_drain_workqueue as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("flush_work"),
            Some(linux_flush_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("flush_delayed_work"),
            Some(linux_flush_delayed_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("flush_rcu_work"),
            Some(linux_flush_rcu_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("cancel_delayed_work"),
            Some(linux_cancel_delayed_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("cancel_delayed_work_sync"),
            Some(linux_cancel_delayed_work_sync as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("current_work"),
            Some(linux_current_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("enable_work"),
            Some(linux_enable_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("disable_work"),
            Some(linux_disable_work as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kthread_cancel_work_sync"),
            Some(linux_kthread_cancel_work_sync as usize)
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

    #[test]
    fn linux_queue_work_node_and_drain_workqueue_run_pending_work() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_w: *mut WorkStruct) {
            FIRED.fetch_add(1, O::AcqRel);
        }

        unsafe {
            FIRED.store(0, O::Release);
            let name = b"node-wq\0";
            let wq = linux_alloc_workqueue_noprof(name.as_ptr().cast(), WQ_UNBOUND, 0);
            assert!(!wq.is_null());
            let mut work = WorkStruct::new();
            work.init(cb);
            assert!(linux_queue_work_node(-1, wq, &mut work));
            linux_drain_workqueue(wq);
            assert_eq!(FIRED.load(O::Acquire), 1);
            linux_destroy_workqueue(wq);
        }
    }

    #[test]
    fn delayed_work_cancel_exports_and_clears_pending_bit() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/workqueue.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL(cancel_delayed_work);"));
        assert!(source.contains("EXPORT_SYMBOL(cancel_delayed_work_sync);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(system_power_efficient_wq);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(system_freezable_power_efficient_wq);"));

        let mut work = WorkStruct::new();
        work.data.store(WORK_PENDING, Ordering::Release);

        assert!(unsafe { linux_cancel_delayed_work((&mut work as *mut WorkStruct).cast()) });
        assert!(!work.is_pending());
        assert!(!unsafe { linux_cancel_delayed_work((&mut work as *mut WorkStruct).cast()) });

        work.data.store(WORK_PENDING, Ordering::Release);
        assert!(unsafe { linux_cancel_delayed_work_sync((&mut work as *mut WorkStruct).cast()) });
        assert!(!work.is_pending());
    }

    #[test]
    fn linux_kthread_worker_queues_flushes_and_cancels() {
        use core::sync::atomic::{AtomicU32, Ordering as O};
        static FIRED: AtomicU32 = AtomicU32::new(0);
        unsafe extern "C" fn cb(_work: *mut c_void) {
            FIRED.fetch_add(1, O::AcqRel);
        }

        unsafe {
            FIRED.store(0, O::Release);
            let worker = linux_kthread_create_worker_on_node(0, -1, core::ptr::null(), 0);
            assert!(!worker.is_null());

            let mut work = [0u8; 40];
            work.as_mut_ptr()
                .add(KTHREAD_WORK_FUNC_OFFSET)
                .cast::<usize>()
                .write(cb as usize);
            assert!(linux_kthread_queue_work(worker, work.as_mut_ptr().cast()));
            assert!(!linux_kthread_queue_work(worker, work.as_mut_ptr().cast()));
            linux_kthread_flush_worker(worker);
            assert_eq!(FIRED.load(O::Acquire), 1);

            assert!(linux_kthread_queue_work(worker, work.as_mut_ptr().cast()));
            assert!(linux_kthread_cancel_work_sync(work.as_mut_ptr().cast()));
            linux_kthread_flush_worker(worker);
            assert_eq!(FIRED.load(O::Acquire), 1);
            linux_kthread_destroy_worker(worker);
        }
    }
}
