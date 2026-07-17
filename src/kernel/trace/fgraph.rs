//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/fgraph.c
//! test-origin: linux:vendor/linux/kernel/trace/fgraph.c
//! Function-graph tracer infrastructure.
//!
//! Records function entry / exit pairs with a per-cpu stack so callers can
//! reconstruct call trees.
//!
//! Ref: vendor/linux/kernel/trace/fgraph.c

extern crate alloc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

#[derive(Clone, Copy, Debug)]
pub struct FgraphEntry {
    pub func: u64,
    pub depth: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct FgraphReturn {
    pub func: u64,
    pub calltime: u64,
    pub rettime: u64,
    pub depth: u32,
    pub retval: u64,
}

static STACK: Mutex<Vec<FgraphEntry>> = Mutex::new(Vec::new());
static FGRAPH_UPDATE_LOCK: Mutex<()> = Mutex::new(());
static FGRAPH_OPS: Mutex<Vec<&'static FgraphOps>> = Mutex::new(Vec::new());
static RETURN_STACK: Mutex<Vec<ReturnFrame>> = Mutex::new(Vec::new());
// Covers both entry and return callbacks. Unregistration first unpublishes
// the ops and then waits here before allowing its module text to be freed.
static FGRAPH_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static FGRAPH_RECURSION: [AtomicUsize; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicUsize::new(0) }; crate::kernel::sched::MAX_CPUS];

pub const MAX_FGRAPH_OPS: usize = 8;
pub const MAX_FGRAPH_DEPTH: usize = 256;

pub type FgraphEntryFn = fn(entry: &FgraphEntry, data: usize) -> bool;
pub type FgraphReturnFn = fn(ret: &FgraphReturn, data: usize);

pub struct FgraphOps {
    pub entry: FgraphEntryFn,
    pub return_: FgraphReturnFn,
    pub data: usize,
}

impl FgraphOps {
    pub const fn new(entry: FgraphEntryFn, return_: FgraphReturnFn) -> Self {
        Self {
            entry,
            return_,
            data: 0,
        }
    }
}

unsafe impl Sync for FgraphOps {}

struct ReturnFrame {
    task: usize,
    original_return: u64,
    func: u64,
    calltime: u64,
    depth: u32,
    // Retain the exact callbacks which accepted entry.  Looking them up in
    // FGRAPH_OPS again at return time creates an unregister race: an op can
    // be temporarily unpublished while unregister determines that this frame
    // still owns it, causing the matching return callback to be skipped.
    consumers: Vec<&'static FgraphOps>,
}

fn now_nsec() -> u64 {
    crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000
}

fn current_task_key() -> usize {
    unsafe { crate::kernel::sched::get_current() as usize }
}

fn current_cpu() -> usize {
    (crate::kernel::sched::current_cpu() as usize).min(crate::kernel::sched::MAX_CPUS - 1)
}

pub fn register_ftrace_graph(ops: &'static FgraphOps) -> Result<(), i32> {
    let _update = FGRAPH_UPDATE_LOCK.lock();
    let mut registered = FGRAPH_OPS.lock();
    if registered.iter().any(|entry| core::ptr::eq(*entry, ops)) {
        return Err(-17); // EEXIST
    }
    if registered.len() >= MAX_FGRAPH_OPS {
        return Err(-12); // ENOMEM
    }
    crate::kernel::trace::ftrace::set_graph_tracing(true)?;
    registered.push(ops);
    Ok(())
}

pub fn unregister_ftrace_graph(ops: &'static FgraphOps) -> Result<(), i32> {
    let _update = FGRAPH_UPDATE_LOCK.lock();
    let mut registered = FGRAPH_OPS.lock();
    let Some(index) = registered
        .iter()
        .position(|entry| core::ptr::eq(*entry, ops))
    else {
        return Err(-2); // ENOENT
    };
    registered.remove(index);
    drop(registered);
    while FGRAPH_IN_FLIGHT.load(Ordering::Acquire) != 0 {
        core::hint::spin_loop();
    }
    if RETURN_STACK.lock().iter().any(|frame| {
        frame
            .consumers
            .iter()
            .any(|consumer| core::ptr::eq(*consumer, ops))
    }) {
        FGRAPH_OPS.lock().push(ops);
        return Err(-16); // EBUSY: a return hook still owns this callback
    }
    crate::kernel::trace::ftrace::set_graph_tracing(false)
}

/// Architecture entry hook.  Save the real caller return address and replace
/// it with `return_to_handler`, exactly like x86 `prepare_ftrace_return()`.
pub fn function_graph_enter(func: u64, parent: *mut u64) -> bool {
    if parent.is_null() {
        return false;
    }
    let cpu = current_cpu();
    if FGRAPH_RECURSION[cpu]
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    FGRAPH_IN_FLIGHT.fetch_add(1, Ordering::AcqRel);
    let task = current_task_key();
    let depth = RETURN_STACK
        .lock()
        .iter()
        .filter(|frame| frame.task == task)
        .count();
    if depth >= MAX_FGRAPH_DEPTH {
        FGRAPH_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
        FGRAPH_RECURSION[cpu].store(0, Ordering::Release);
        return false;
    }
    let entry = FgraphEntry {
        func,
        depth: depth as u32,
    };
    let ops = FGRAPH_OPS.lock().clone();
    let mut consumers = Vec::new();
    for ops in ops {
        if (ops.entry)(&entry, ops.data) {
            consumers.push(ops);
        }
    }
    if consumers.is_empty() {
        FGRAPH_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
        FGRAPH_RECURSION[cpu].store(0, Ordering::Release);
        return false;
    }
    let original_return = unsafe { parent.read_volatile() };
    let handler = crate::arch::x86::kernel::ftrace::return_to_handler_addr() as u64;
    if original_return == handler {
        FGRAPH_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
        FGRAPH_RECURSION[cpu].store(0, Ordering::Release);
        return false;
    }
    RETURN_STACK.lock().push(ReturnFrame {
        task,
        original_return,
        func,
        calltime: now_nsec(),
        depth: depth as u32,
        consumers,
    });
    unsafe { parent.write_volatile(handler) };
    FGRAPH_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
    FGRAPH_RECURSION[cpu].store(0, Ordering::Release);
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn lupos_fgraph_return_dispatch(retval: u64) -> u64 {
    let cpu = current_cpu();
    assert!(
        FGRAPH_RECURSION[cpu]
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok(),
        "recursive function-graph return dispatch"
    );
    FGRAPH_IN_FLIGHT.fetch_add(1, Ordering::AcqRel);
    let task = current_task_key();
    let frame = {
        let mut stack = RETURN_STACK.lock();
        let index = stack
            .iter()
            .rposition(|frame| frame.task == task)
            .expect("function-graph return stack underflow");
        stack.remove(index)
    };
    let ret = FgraphReturn {
        func: frame.func,
        calltime: frame.calltime,
        rettime: now_nsec(),
        depth: frame.depth,
        retval,
    };
    // The frame, rather than the mutable registration list, owns the return
    // callbacks. unregister_ftrace_graph() refuses to complete while one of
    // these references is pending, so invoking them here is both race-free
    // and lifetime-safe.
    for ops in frame.consumers {
        (ops.return_)(&ret, ops.data);
    }
    FGRAPH_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
    FGRAPH_RECURSION[cpu].store(0, Ordering::Release);
    frame.original_return
}

/// `ftrace_graph_exit_task()` — discard return hooks owned by a task which
/// will never unwind its instrumented stack. The frame-held `FgraphOps`
/// references are lifetime pins; removing them at `do_exit()` is required so
/// callback unregistration cannot remain permanently busy after task death.
pub fn ftrace_graph_exit_task(task: *mut crate::kernel::task::TaskStruct) -> usize {
    let task = task as usize;
    let mut frames = RETURN_STACK.lock();
    let before = frames.len();
    frames.retain(|frame| frame.task != task);
    before - frames.len()
}

/// `ftrace_push_return_trace`.
pub fn push(entry: FgraphEntry) {
    STACK.lock().push(entry);
}

/// `ftrace_pop_return_trace`.
pub fn pop(now: u64, calltime: u64) -> Option<FgraphReturn> {
    let e = STACK.lock().pop()?;
    Some(FgraphReturn {
        func: e.func,
        calltime,
        rettime: now,
        depth: e.depth,
        retval: 0,
    })
}

pub fn depth() -> usize {
    STACK.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU64, Ordering};

    static GRAPH_TEST_LOCK: Mutex<()> = Mutex::new(());
    static ENTRY_HIT: AtomicU64 = AtomicU64::new(0);
    static RETURN_HIT: AtomicU64 = AtomicU64::new(0);
    static SECOND_ENTRY_HIT: AtomicU64 = AtomicU64::new(0);
    static SECOND_RETURN_HIT: AtomicU64 = AtomicU64::new(0);

    fn graph_entry(entry: &FgraphEntry, data: usize) -> bool {
        ENTRY_HIT.store(entry.func + data as u64, Ordering::Relaxed);
        true
    }

    fn graph_return(ret: &FgraphReturn, data: usize) {
        RETURN_HIT.store(ret.retval + data as u64, Ordering::Relaxed);
    }

    static GRAPH_OPS: FgraphOps = FgraphOps {
        entry: graph_entry,
        return_: graph_return,
        data: 5,
    };

    fn second_graph_entry(entry: &FgraphEntry, data: usize) -> bool {
        SECOND_ENTRY_HIT.store(entry.func + data as u64, Ordering::Relaxed);
        entry.func == 0x2000
    }

    fn second_graph_return(ret: &FgraphReturn, data: usize) {
        SECOND_RETURN_HIT.store(ret.retval + data as u64, Ordering::Relaxed);
    }

    static SECOND_GRAPH_OPS: FgraphOps = FgraphOps {
        entry: second_graph_entry,
        return_: second_graph_return,
        data: 9,
    };

    #[test]
    fn push_pop_round_trip() {
        let _guard = GRAPH_TEST_LOCK.lock();
        let d0 = depth();
        push(FgraphEntry {
            func: 0x1000,
            depth: 0,
        });
        push(FgraphEntry {
            func: 0x2000,
            depth: 1,
        });
        assert_eq!(depth(), d0 + 2);
        let r = pop(100, 50).unwrap();
        assert_eq!(r.func, 0x2000);
        assert_eq!(r.rettime, 100);
        assert_eq!(r.retval, 0);
        let _ = pop(101, 50);
        assert_eq!(depth(), d0);
    }

    #[test]
    fn graph_entry_rewrites_and_return_restores_the_caller() {
        let _guard = GRAPH_TEST_LOCK.lock();
        ENTRY_HIT.store(0, Ordering::Relaxed);
        RETURN_HIT.store(0, Ordering::Relaxed);
        register_ftrace_graph(&GRAPH_OPS).unwrap();
        let mut parent = 0xfeed_beefu64;
        assert!(function_graph_enter(0x1000, &mut parent));
        assert_eq!(
            parent,
            crate::arch::x86::kernel::ftrace::return_to_handler_addr() as u64
        );
        assert_eq!(ENTRY_HIT.load(Ordering::Relaxed), 0x1005);
        assert_eq!(lupos_fgraph_return_dispatch(37), 0xfeed_beef);
        assert_eq!(RETURN_HIT.load(Ordering::Relaxed), 42);
        unregister_ftrace_graph(&GRAPH_OPS).unwrap();
    }

    #[test]
    fn multiple_graph_ops_track_return_consumers_independently() {
        let _guard = GRAPH_TEST_LOCK.lock();
        ENTRY_HIT.store(0, Ordering::Relaxed);
        RETURN_HIT.store(0, Ordering::Relaxed);
        SECOND_ENTRY_HIT.store(0, Ordering::Relaxed);
        SECOND_RETURN_HIT.store(0, Ordering::Relaxed);
        register_ftrace_graph(&GRAPH_OPS).unwrap();
        register_ftrace_graph(&SECOND_GRAPH_OPS).unwrap();

        let mut first_parent = 0x1111u64;
        assert!(function_graph_enter(0x1000, &mut first_parent));
        assert_eq!(lupos_fgraph_return_dispatch(3), 0x1111);
        assert_eq!(RETURN_HIT.load(Ordering::Relaxed), 8);
        assert_eq!(SECOND_ENTRY_HIT.load(Ordering::Relaxed), 0x1009);
        assert_eq!(SECOND_RETURN_HIT.load(Ordering::Relaxed), 0);

        let mut second_parent = 0x2222u64;
        assert!(function_graph_enter(0x2000, &mut second_parent));
        assert_eq!(lupos_fgraph_return_dispatch(4), 0x2222);
        assert_eq!(RETURN_HIT.load(Ordering::Relaxed), 9);
        assert_eq!(SECOND_RETURN_HIT.load(Ordering::Relaxed), 13);

        unregister_ftrace_graph(&SECOND_GRAPH_OPS).unwrap();
        unregister_ftrace_graph(&GRAPH_OPS).unwrap();
    }

    #[test]
    fn graph_op_cannot_unregister_until_its_return_callback_finishes() {
        let _guard = GRAPH_TEST_LOCK.lock();
        RETURN_HIT.store(0, Ordering::Relaxed);
        register_ftrace_graph(&GRAPH_OPS).unwrap();

        let mut parent = 0x3333u64;
        assert!(function_graph_enter(0x3000, &mut parent));
        assert_eq!(unregister_ftrace_graph(&GRAPH_OPS), Err(-16));
        assert_eq!(lupos_fgraph_return_dispatch(12), 0x3333);
        assert_eq!(RETURN_HIT.load(Ordering::Relaxed), 17);
        unregister_ftrace_graph(&GRAPH_OPS).unwrap();
    }

    #[test]
    fn task_exit_releases_abandoned_return_callback_ownership() {
        let _guard = GRAPH_TEST_LOCK.lock();
        let task = 0x1234usize;
        RETURN_STACK.lock().push(ReturnFrame {
            task,
            original_return: 0x4444,
            func: 0x4000,
            calltime: 1,
            depth: 0,
            consumers: alloc::vec![&GRAPH_OPS],
        });

        assert_eq!(
            ftrace_graph_exit_task(task as *mut crate::kernel::task::TaskStruct),
            1
        );
        assert!(!RETURN_STACK.lock().iter().any(|frame| frame.task == task));
    }
}
