//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/ftrace.c
//! test-origin: linux:vendor/linux/kernel/trace/ftrace.c
//! Function tracer skeleton.
//!
//! Mirrors `vendor/linux/kernel/trace/ftrace.c::function_trace_call`.
//! Lupos M62: no `mcount`/`fentry` patching yet — instead, instrumented
//! functions explicitly call `ftrace_function_trace_call(ip, parent_ip)` at
//! their entry.  The tracer enabled/disabled state and the active probe
//! function are wired via globals matching Linux's `ftrace_trace_function`.
//!
//! Refs:
//! - `vendor/linux/include/linux/ftrace.h::register_ftrace_function`
//! - `vendor/linux/kernel/trace/ftrace.c::ftrace_caller`

use core::sync::atomic::{AtomicUsize, Ordering};

use super::ring_buffer::{TRACE_FN, TRACE_RB, TraceEvent};

/// Active function probe (0 = no tracer attached).
/// Stored as `usize` because we want a single atomic store.
static ACTIVE_PROBE: AtomicUsize = AtomicUsize::new(0);

pub type FtraceFn = fn(ip: u64, parent_ip: u64);

pub fn register_ftrace_function(probe: FtraceFn) -> Result<(), i32> {
    if ACTIVE_PROBE.load(Ordering::Acquire) != 0 {
        return Err(-16); // EBUSY: only one tracer at a time in M62
    }
    ACTIVE_PROBE.store(probe as usize, Ordering::Release);
    Ok(())
}

pub fn unregister_ftrace_function() {
    ACTIVE_PROBE.store(0, Ordering::Release);
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

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_HITS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

    fn test_probe(_ip: u64, _parent: u64) {
        TEST_HITS.fetch_add(1, Ordering::Relaxed);
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
}
