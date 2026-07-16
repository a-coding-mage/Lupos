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

/// Active function probe (0 = no tracer attached).
/// Stored as `usize` because we want a single atomic store.
static ACTIVE_PROBE: AtomicUsize = AtomicUsize::new(0);
/// Register-aware callback selected by `FTRACE_OPS_FL_SAVE_REGS`-style users.
static ACTIVE_REGS_PROBE: AtomicUsize = AtomicUsize::new(0);
static FTRACE_UPDATE_LOCK: Mutex<()> = Mutex::new(());

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

fn tracing_active() -> bool {
    ACTIVE_PROBE.load(Ordering::Acquire) != 0
        || ACTIVE_REGS_PROBE.load(Ordering::Acquire) != 0
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

pub fn register_ftrace_function(probe: FtraceFn) -> Result<(), i32> {
    let _update = FTRACE_UPDATE_LOCK.lock();
    if ACTIVE_PROBE.load(Ordering::Acquire) != 0 {
        return Err(-16); // EBUSY: only one tracer at a time in M62
    }

    let mut records = MODULE_RECORDS.lock();
    if !tracing_active() {
        patch_enabled_records(&mut records)?;
    }
    ACTIVE_PROBE.store(probe as usize, Ordering::Release);
    Ok(())
}

pub fn unregister_ftrace_function() {
    let _update = FTRACE_UPDATE_LOCK.lock();
    ACTIVE_PROBE.store(0, Ordering::Release);
    if !tracing_active() {
        unpatch_enabled_records(&mut MODULE_RECORDS.lock());
    }
}

pub fn register_ftrace_regs_function(probe: FtraceRegsFn) -> Result<(), i32> {
    let _update = FTRACE_UPDATE_LOCK.lock();
    if ACTIVE_REGS_PROBE.load(Ordering::Acquire) != 0 {
        return Err(-16);
    }
    let mut records = MODULE_RECORDS.lock();
    if !tracing_active() {
        patch_enabled_records(&mut records)?;
    }
    ACTIVE_REGS_PROBE.store(probe as usize, Ordering::Release);
    Ok(())
}

pub fn unregister_ftrace_regs_function() {
    let _update = FTRACE_UPDATE_LOCK.lock();
    ACTIVE_REGS_PROBE.store(0, Ordering::Release);
    if !tracing_active() {
        unpatch_enabled_records(&mut MODULE_RECORDS.lock());
    }
}

/// Call site placed at the entry of any instrumented function.
/// Cheap when no tracer is attached.
#[inline]
pub fn ftrace_function_trace_call(ip: u64, parent_ip: u64) {
    let probe = ACTIVE_PROBE.load(Ordering::Relaxed);
    if probe == 0 {
        return;
    }
    let f: FtraceFn = unsafe { core::mem::transmute(probe) };
    f(ip, parent_ip);
}

pub fn ftrace_function_trace_call_with_regs(ip: u64, parent_ip: u64, sp: u64, bp: u64) {
    ftrace_function_trace_call(ip, parent_ip);
    let probe = ACTIVE_REGS_PROBE.load(Ordering::Relaxed);
    if probe != 0 {
        let f: FtraceRegsFn = unsafe { core::mem::transmute(probe) };
        f(ip, parent_ip, sp, bp);
    }
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

/// `ftrace_release_mod()` — unconditionally discard all addresses backed by
/// module memory.  Linux invokes this on formation failure as well as unload.
pub fn release_module(owner: usize) {
    let _update = FTRACE_UPDATE_LOCK.lock();
    let mut records = MODULE_RECORDS.lock();
    for record in records
        .iter_mut()
        .filter(|record| record.owner == owner && record.enabled && record.traced)
    {
        if crate::arch::x86::kernel::ftrace::set_module_callsite(record.ip, false).is_ok() {
            record.traced = false;
        }
    }
    records.retain(|record| record.owner != owner);
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

    fn test_probe(_ip: u64, _parent: u64) {
        TEST_HITS.fetch_add(1, Ordering::Relaxed);
    }

    fn test_regs_probe(ip: u64, parent: u64, sp: u64, bp: u64) {
        TEST_REGS.store(ip ^ parent ^ sp ^ bp, Ordering::Relaxed);
    }

    #[test]
    fn register_then_call_records_hit() {
        TEST_HITS.store(0, Ordering::Relaxed);
        register_ftrace_function(test_probe).unwrap();
        ftrace_function_trace_call(0x1000, 0x2000);
        ftrace_function_trace_call(0x1000, 0x2000);
        assert_eq!(TEST_HITS.load(Ordering::Relaxed), 2);
        unregister_ftrace_function();
    }

    #[test]
    fn no_probe_is_zero_overhead() {
        unregister_ftrace_function();
        ftrace_function_trace_call(0x1000, 0x2000); // no-op
    }

    #[test]
    fn second_register_returns_ebusy() {
        register_ftrace_function(test_probe).unwrap();
        assert_eq!(register_ftrace_function(test_probe), Err(-16));
        unregister_ftrace_function();
    }

    #[test]
    fn register_aware_callback_receives_live_state() {
        unregister_ftrace_function();
        unregister_ftrace_regs_function();
        TEST_REGS.store(0, Ordering::Relaxed);
        register_ftrace_regs_function(test_regs_probe).unwrap();
        ftrace_function_trace_call_with_regs(1, 2, 4, 8);
        assert_eq!(TEST_REGS.load(Ordering::Relaxed), 15);
        unregister_ftrace_regs_function();
    }
}
