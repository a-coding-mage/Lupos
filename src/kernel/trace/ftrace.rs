//! linux-parity: partial
//! linux-source: vendor/linux/kernel/trace/ftrace.c
//! test-origin: linux:vendor/linux/kernel/trace/ftrace.c
//! Dynamic function tracer and module callsite lifecycle.
//!
//! Mirrors `vendor/linux/kernel/trace/ftrace.c::function_trace_call`.
//! Refs:
//! - `vendor/linux/include/linux/ftrace.h::register_ftrace_function`
//! - `vendor/linux/kernel/trace/ftrace.c::ftrace_caller`

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use super::ring_buffer::{TRACE_FN, TRACE_RB, TraceEvent};

static FTRACE_UPDATE_LOCK: Mutex<()> = Mutex::new(());
static GRAPH_USERS: AtomicUsize = AtomicUsize::new(0);
// Readers increment this before loading any published callback. Removal
// clears the callback first and then waits for all pre-existing readers, so a
// module may free callback text as soon as unregister returns.
static FTRACE_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

pub const MAX_FTRACE_OPS: usize = 16;
const FTRACE_KIND_SIMPLE: usize = 1;
const FTRACE_KIND_REGS: usize = 2;
const FTRACE_KIND_OPS: usize = 3;

struct FtraceSlot {
    key: AtomicUsize,
    callback: AtomicUsize,
    data: AtomicUsize,
    filter: AtomicUsize,
    kind: AtomicUsize,
}

impl FtraceSlot {
    const fn new() -> Self {
        Self {
            key: AtomicUsize::new(0),
            callback: AtomicUsize::new(0),
            data: AtomicUsize::new(0),
            filter: AtomicUsize::new(0),
            kind: AtomicUsize::new(0),
        }
    }
}

static FTRACE_SLOTS: [FtraceSlot; MAX_FTRACE_OPS] = [const { FtraceSlot::new() }; MAX_FTRACE_OPS];
static FTRACE_RECURSION: [AtomicUsize; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicUsize::new(0) }; crate::kernel::sched::MAX_CPUS];

/// One relocated `__mcount_loc`/`__patchable_function_entries` callsite.
///
/// Vendor Linux creates `struct dyn_ftrace` records in
/// `ftrace_module_init()`, keeps them disabled until
/// `ftrace_module_enable()`, and removes every record owned by the module in
/// `ftrace_release_mod()`.  The text-patching backend consumes this registry;
/// keeping ownership here is what makes failed load and unload safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleFtraceRecord {
    pub owner: usize,
    pub ip: usize,
    /// The module has reached COMING and the record may be patched live.
    pub enabled: bool,
    /// The callsite currently contains `CALL lupos_ftrace_caller`.
    pub traced: bool,
}

static MODULE_RECORDS: Mutex<Vec<ModuleFtraceRecord>> = Mutex::new(Vec::new());

pub type FtraceFn = fn(ip: u64, parent_ip: u64);
pub type FtraceRegsFn = fn(ip: u64, parent_ip: u64, sp: u64, bp: u64);
pub type FtraceFilterFn = fn(ip: u64, parent_ip: u64) -> bool;
pub type FtraceOpsFn = fn(ip: u64, parent_ip: u64, sp: u64, bp: u64, data: usize);

pub struct FtraceOps {
    pub func: FtraceOpsFn,
    pub filter: Option<FtraceFilterFn>,
    pub data: usize,
}

impl FtraceOps {
    pub const fn new(func: FtraceOpsFn) -> Self {
        Self {
            func,
            filter: None,
            data: 0,
        }
    }
}

unsafe impl Sync for FtraceOps {}

fn tracing_active() -> bool {
    GRAPH_USERS.load(Ordering::Acquire) != 0
        || FTRACE_SLOTS
            .iter()
            .any(|slot| slot.callback.load(Ordering::Acquire) != 0)
}

fn patch_enabled_records(records: &mut [ModuleFtraceRecord]) -> Result<(), i32> {
    let mut patched: Vec<usize> = Vec::new();
    for index in 0..records.len() {
        if !records[index].enabled || records[index].traced {
            continue;
        }
        let ip = records[index].ip;
        if let Err(error) = crate::arch::x86::kernel::ftrace::set_module_callsite(ip, true) {
            let current =
                crate::arch::x86::kernel::alternative::text_poke_read(ip, 5).unwrap_or_default();
            crate::log_error!(
                "ftrace",
                "module callsite patch failed: ip={:#x} caller={:#x} error={} bytes={:x?}",
                ip,
                crate::arch::x86::kernel::ftrace::ftrace_caller_addr(),
                error,
                current
            );
            for index in patched.into_iter().rev() {
                let rollback = &mut records[index];
                let _ = crate::arch::x86::kernel::ftrace::set_module_callsite(rollback.ip, false);
                rollback.traced = false;
            }
            return Err(-error.abs());
        }
        records[index].traced = true;
        patched.push(index);
    }
    Ok(())
}

fn unpatch_enabled_records(records: &mut [ModuleFtraceRecord]) {
    for record in records
        .iter_mut()
        .filter(|record| record.enabled && record.traced)
    {
        if crate::arch::x86::kernel::ftrace::set_module_callsite(record.ip, false).is_ok() {
            record.traced = false;
        }
    }
}

fn register_slot(
    key: usize,
    callback: usize,
    kind: usize,
    data: usize,
    filter: usize,
) -> Result<(), i32> {
    let _update = FTRACE_UPDATE_LOCK.lock();
    if FTRACE_SLOTS.iter().any(|slot| {
        slot.key.load(Ordering::Acquire) == key && slot.kind.load(Ordering::Acquire) == kind
    }) {
        return Err(-17); // EEXIST
    }
    let slot = FTRACE_SLOTS
        .iter()
        .find(|slot| slot.callback.load(Ordering::Acquire) == 0)
        .ok_or(-12)?; // ENOMEM
    let mut records = MODULE_RECORDS.lock();
    if !tracing_active() {
        patch_enabled_records(&mut records)?;
    }
    slot.key.store(key, Ordering::Relaxed);
    slot.data.store(data, Ordering::Relaxed);
    slot.filter.store(filter, Ordering::Relaxed);
    slot.kind.store(kind, Ordering::Relaxed);
    slot.callback.store(callback, Ordering::Release);
    Ok(())
}

fn unregister_slots(mut matches: impl FnMut(usize, usize) -> bool) -> bool {
    let _update = FTRACE_UPDATE_LOCK.lock();
    let mut removed = false;
    for slot in &FTRACE_SLOTS {
        let callback = slot.callback.load(Ordering::Acquire);
        let kind = slot.kind.load(Ordering::Relaxed);
        let key = slot.key.load(Ordering::Relaxed);
        if callback != 0 && matches(key, kind) {
            slot.callback.store(0, Ordering::Release);
            slot.kind.store(0, Ordering::Relaxed);
            slot.key.store(0, Ordering::Relaxed);
            removed = true;
        }
    }
    if !tracing_active() {
        unpatch_enabled_records(&mut MODULE_RECORDS.lock());
    }
    if removed {
        while FTRACE_IN_FLIGHT.load(Ordering::Acquire) != 0 {
            core::hint::spin_loop();
        }
    }
    removed
}

pub fn register_ftrace_function(probe: FtraceFn) -> Result<(), i32> {
    register_slot(probe as usize, probe as usize, FTRACE_KIND_SIMPLE, 0, 0)
}

pub fn unregister_ftrace_function() {
    unregister_slots(|_, kind| kind == FTRACE_KIND_SIMPLE);
}

pub fn register_ftrace_regs_function(probe: FtraceRegsFn) -> Result<(), i32> {
    register_slot(probe as usize, probe as usize, FTRACE_KIND_REGS, 0, 0)
}

pub fn unregister_ftrace_regs_function() {
    unregister_slots(|_, kind| kind == FTRACE_KIND_REGS);
}

pub fn register_ftrace_ops(ops: &'static FtraceOps) -> Result<(), i32> {
    register_slot(
        ops as *const FtraceOps as usize,
        ops.func as usize,
        FTRACE_KIND_OPS,
        ops.data,
        ops.filter.map_or(0, |filter| filter as usize),
    )
}

pub fn unregister_ftrace_ops(ops: &'static FtraceOps) -> Result<(), i32> {
    if unregister_slots(|key, kind| {
        kind == FTRACE_KIND_OPS && key == ops as *const FtraceOps as usize
    }) {
        Ok(())
    } else {
        Err(-2) // ENOENT
    }
}

pub(crate) fn set_graph_tracing(enable: bool) -> Result<(), i32> {
    let _update = FTRACE_UPDATE_LOCK.lock();
    let old = GRAPH_USERS.load(Ordering::Acquire);
    if enable {
        if old == 0 && !tracing_active() {
            patch_enabled_records(&mut MODULE_RECORDS.lock())?;
        }
        GRAPH_USERS.store(old.saturating_add(1), Ordering::Release);
    } else if old != 0 {
        GRAPH_USERS.store(old - 1, Ordering::Release);
        if !tracing_active() {
            unpatch_enabled_records(&mut MODULE_RECORDS.lock());
        }
    }
    Ok(())
}

/// Call site placed at the entry of any instrumented function.
/// Cheap when no tracer is attached.
#[inline]
pub fn ftrace_function_trace_call(ip: u64, parent_ip: u64) {
    ftrace_function_trace_call_with_regs(ip, parent_ip, 0, 0);
}

pub fn ftrace_function_trace_call_with_regs(ip: u64, parent_ip: u64, sp: u64, bp: u64) {
    let cpu =
        (crate::kernel::sched::current_cpu() as usize).min(crate::kernel::sched::MAX_CPUS - 1);
    if FTRACE_RECURSION[cpu]
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    FTRACE_IN_FLIGHT.fetch_add(1, Ordering::AcqRel);
    for slot in &FTRACE_SLOTS {
        let callback = slot.callback.load(Ordering::Acquire);
        if callback == 0 {
            continue;
        }
        let filter = slot.filter.load(Ordering::Relaxed);
        if filter != 0 {
            let filter: FtraceFilterFn = unsafe { core::mem::transmute(filter) };
            if !filter(ip, parent_ip) {
                continue;
            }
        }
        match slot.kind.load(Ordering::Relaxed) {
            FTRACE_KIND_SIMPLE => {
                let f: FtraceFn = unsafe { core::mem::transmute(callback) };
                f(ip, parent_ip);
            }
            FTRACE_KIND_REGS => {
                let f: FtraceRegsFn = unsafe { core::mem::transmute(callback) };
                f(ip, parent_ip, sp, bp);
            }
            FTRACE_KIND_OPS => {
                let f: FtraceOpsFn = unsafe { core::mem::transmute(callback) };
                f(ip, parent_ip, sp, bp, slot.data.load(Ordering::Relaxed));
            }
            _ => {}
        }
    }
    FTRACE_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
    FTRACE_RECURSION[cpu].store(0, Ordering::Release);
}

/// Default function tracer probe: push a TraceEvent into the global ring.
pub fn function_trace_call(ip: u64, parent_ip: u64) {
    TRACE_RB.push(TraceEvent {
        ts_nsec: crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000,
        ev_type: TRACE_FN,
        cpu: 0,
        pid: 0,
        arg0: ip,
        arg1: parent_ip,
    });
}

/// `ftrace_module_init()` — consume a relocated module callsite table while
/// the module is still `MODULE_STATE_UNFORMED`.
///
/// Linux sorts module callsites at load time because modpost cannot globally
/// sort the per-object contributions.  Zero entries are linker padding and
/// are skipped by `ftrace_process_locs()`.
pub fn module_init(owner: usize, callsites: &mut [usize]) -> Result<(), i32> {
    let _update = FTRACE_UPDATE_LOCK.lock();
    let mut records = MODULE_RECORDS.lock();
    if records.iter().any(|record| record.owner == owner) {
        return Err(-17); // EEXIST
    }

    callsites.sort_unstable();
    let trace_active = tracing_active();
    for ip in callsites.iter().copied().filter(|ip| *ip != 0) {
        let traced = crate::arch::x86::kernel::ftrace::prepare_module_callsite(ip, trace_active)
            .map_err(|error| -error.abs())?;
        records.push(ModuleFtraceRecord {
            owner,
            ip,
            enabled: false,
            traced,
        });
    }
    Ok(())
}

/// `ftrace_module_enable()` — publish records only after module text has its
/// final permissions and the architecture has converted every entry site to
/// its disabled/NOP form.
pub fn module_enable(owner: usize) -> Result<(), i32> {
    let _update = FTRACE_UPDATE_LOCK.lock();
    for record in MODULE_RECORDS
        .lock()
        .iter_mut()
        .filter(|record| record.owner == owner)
    {
        record.enabled = true;
    }
    Ok(())
}

/// `ftrace_release_mod()` — discard all addresses backed by module memory
/// only after every live callsite has been restored.
///
/// Text-poke failure is retryable: successfully restored records remain
/// disabled, while the complete owner set stays registered so module memory
/// cannot be released with a callsite still targeting the ftrace trampoline.
/// Linux invokes this on formation failure as well as unload.
fn release_module_with(
    owner: usize,
    mut set_callsite: impl FnMut(usize, bool) -> Result<(), i32>,
) -> Result<(), i32> {
    let _update = FTRACE_UPDATE_LOCK.lock();
    let mut records = MODULE_RECORDS.lock();
    for record in records
        .iter_mut()
        .filter(|record| record.owner == owner && record.enabled && record.traced)
    {
        set_callsite(record.ip, false).map_err(|error| -error.abs())?;
        record.traced = false;
    }
    records.retain(|record| record.owner != owner);
    Ok(())
}

pub fn release_module(owner: usize) -> Result<(), i32> {
    release_module_with(owner, crate::arch::x86::kernel::ftrace::set_module_callsite)
}

pub fn module_records(owner: usize) -> Vec<ModuleFtraceRecord> {
    MODULE_RECORDS
        .lock()
        .iter()
        .filter(|record| record.owner == owner)
        .copied()
        .collect()
}

pub fn location_registered(ip: usize) -> bool {
    MODULE_RECORDS
        .lock()
        .iter()
        .any(|record| record.ip == ip && record.enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_HITS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    static TEST_REGS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    static TEST_OPS_HITS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn test_probe(_ip: u64, _parent: u64) {
        TEST_HITS.fetch_add(1, Ordering::Relaxed);
    }

    fn test_regs_probe(ip: u64, parent: u64, sp: u64, bp: u64) {
        TEST_REGS.store(ip ^ parent ^ sp ^ bp, Ordering::Relaxed);
    }

    fn second_probe(_ip: u64, _parent: u64) {
        TEST_HITS.fetch_add(10, Ordering::Relaxed);
    }

    fn ops_probe(ip: u64, _parent: u64, _sp: u64, _bp: u64, data: usize) {
        TEST_OPS_HITS.store(ip + data as u64, Ordering::Relaxed);
    }

    fn only_1000(ip: u64, _parent: u64) -> bool {
        ip == 0x1000
    }

    static TEST_OPS: FtraceOps = FtraceOps {
        func: ops_probe,
        filter: Some(only_1000),
        data: 7,
    };

    #[test]
    fn register_then_call_records_hit() {
        let _guard = TEST_LOCK.lock();
        unregister_ftrace_function();
        unregister_ftrace_regs_function();
        TEST_HITS.store(0, Ordering::Relaxed);
        register_ftrace_function(test_probe).unwrap();
        ftrace_function_trace_call(0x1000, 0x2000);
        ftrace_function_trace_call(0x1000, 0x2000);
        assert_eq!(TEST_HITS.load(Ordering::Relaxed), 2);
        unregister_ftrace_function();
    }

    #[test]
    fn no_probe_is_zero_overhead() {
        let _guard = TEST_LOCK.lock();
        unregister_ftrace_function();
        unregister_ftrace_regs_function();
        ftrace_function_trace_call(0x1000, 0x2000); // no-op
    }

    #[test]
    fn multiple_callbacks_run_and_duplicate_registration_is_rejected() {
        let _guard = TEST_LOCK.lock();
        unregister_ftrace_function();
        TEST_HITS.store(0, Ordering::Relaxed);
        register_ftrace_function(test_probe).unwrap();
        register_ftrace_function(second_probe).unwrap();
        assert_eq!(register_ftrace_function(test_probe), Err(-17));
        ftrace_function_trace_call(0x1000, 0x2000);
        assert_eq!(TEST_HITS.load(Ordering::Relaxed), 11);
        unregister_ftrace_function();
    }

    #[test]
    fn register_aware_callback_receives_live_state() {
        let _guard = TEST_LOCK.lock();
        unregister_ftrace_function();
        unregister_ftrace_regs_function();
        TEST_REGS.store(0, Ordering::Relaxed);
        register_ftrace_regs_function(test_regs_probe).unwrap();
        ftrace_function_trace_call_with_regs(1, 2, 4, 8);
        assert_eq!(TEST_REGS.load(Ordering::Relaxed), 15);
        unregister_ftrace_regs_function();
    }

    #[test]
    fn ftrace_ops_filter_and_private_data_are_applied() {
        let _guard = TEST_LOCK.lock();
        TEST_OPS_HITS.store(0, Ordering::Relaxed);
        register_ftrace_ops(&TEST_OPS).unwrap();
        ftrace_function_trace_call_with_regs(0x999, 0, 1, 2);
        assert_eq!(TEST_OPS_HITS.load(Ordering::Relaxed), 0);
        ftrace_function_trace_call_with_regs(0x1000, 0, 1, 2);
        assert_eq!(TEST_OPS_HITS.load(Ordering::Relaxed), 0x1007);
        unregister_ftrace_ops(&TEST_OPS).unwrap();
        assert_eq!(unregister_ftrace_ops(&TEST_OPS), Err(-2));
    }

    #[test]
    fn failed_module_release_retains_records_and_retries_only_live_sites() {
        let _guard = TEST_LOCK.lock();
        const OWNER: usize = usize::MAX - 1;
        MODULE_RECORDS.lock().extend([
            ModuleFtraceRecord {
                owner: OWNER,
                ip: 0x1000,
                enabled: true,
                traced: true,
            },
            ModuleFtraceRecord {
                owner: OWNER,
                ip: 0x2000,
                enabled: true,
                traced: true,
            },
        ]);

        let mut first_attempt = Vec::new();
        assert_eq!(
            release_module_with(OWNER, |ip, enabled| {
                assert!(!enabled);
                first_attempt.push(ip);
                if ip == 0x2000 { Err(-5) } else { Ok(()) }
            }),
            Err(-5)
        );
        assert_eq!(first_attempt, [0x1000, 0x2000]);
        assert_eq!(
            module_records(OWNER)
                .iter()
                .map(|record| (record.ip, record.traced))
                .collect::<Vec<_>>(),
            [(0x1000, false), (0x2000, true)]
        );

        let mut retry = Vec::new();
        release_module_with(OWNER, |ip, enabled| {
            assert!(!enabled);
            retry.push(ip);
            Ok(())
        })
        .unwrap();
        assert_eq!(retry, [0x2000]);
        assert!(module_records(OWNER).is_empty());
    }
}
