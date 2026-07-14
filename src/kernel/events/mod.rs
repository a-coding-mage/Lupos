//! linux-parity: partial
//! linux-source: vendor/linux/kernel/events
//! test-origin: linux:vendor/linux/kernel/events
//! perf events — `perf_event_open` syscall surface.
//!
//! M63 implements:
//!   - `PerfEventAttr` ABI struct (Linux UAPI).
//!   - `PerfEvent` in-kernel state.
//!   - `sys_perf_event_open` returning a synthetic fd.
//!   - Software events `PERF_COUNT_SW_CPU_CLOCK` (monotonic ns counter).
//!
//! Hardware PMU + sampling + mmap'd ring buffer are deferred.

extern crate alloc;

use alloc::sync::Arc;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicI32, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::EOPNOTSUPP;
use crate::kernel::module::{export_symbol, find_symbol};

pub mod attr;
pub mod linux_sources;
pub use attr::PerfEventAttr;

// ── perf_event_attr.type ──
pub const PERF_TYPE_HARDWARE: u32 = 0;
pub const PERF_TYPE_SOFTWARE: u32 = 1;

// ── PERF_COUNT_SW_* ──
pub const PERF_COUNT_SW_CPU_CLOCK: u64 = 0;
pub const PERF_COUNT_SW_TASK_CLOCK: u64 = 1;

pub const PERF_FORMAT_TOTAL_TIME_ENABLED: u64 = 1 << 0;
pub const PERF_FORMAT_TOTAL_TIME_RUNNING: u64 = 1 << 1;
pub const PERF_FORMAT_ID: u64 = 1 << 2;
pub const PERF_FORMAT_GROUP: u64 = 1 << 3;

pub struct PerfEvent {
    pub fd: i32,
    pub id: u64,
    pub attr: PerfEventAttr,
    inner: Mutex<PerfEventInner>,
}

struct PerfEventInner {
    /// Snapshot of jiffies at open time; reads return current jiffies × 1ms minus this.
    open_ts_ns: u64,
}

static NEXT_FD: AtomicI32 = AtomicI32::new(200);
static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static EVENTS: Mutex<alloc::vec::Vec<Arc<PerfEvent>>> = Mutex::new(alloc::vec::Vec::new());

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("perf_pmu_register", linux_perf_pmu_register as usize, true);
    export_symbol_once(
        "perf_pmu_unregister",
        linux_perf_pmu_unregister as usize,
        true,
    );
    export_symbol_once(
        "perf_event_sysfs_show",
        linux_perf_event_sysfs_show as usize,
        true,
    );
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerfReadRecord {
    pub value: u64,
    pub time_enabled: Option<u64>,
    pub time_running: Option<u64>,
    pub id: Option<u64>,
}

/// `sys_perf_event_open(attr, pid, cpu, group_fd, flags)`.
/// Linux syscall 298.  Returns a synthetic positive fd in M63.
pub unsafe fn sys_perf_event_open(
    attr: *const PerfEventAttr,
    _pid: i32,
    _cpu: i32,
    _group_fd: i32,
    _flags: u64,
) -> i64 {
    if attr.is_null() {
        return -22; // EINVAL
    }
    let attr = unsafe { *attr };
    if attr.type_ != PERF_TYPE_SOFTWARE {
        return -95; // EOPNOTSUPP — only software events in M63
    }
    if attr.config != PERF_COUNT_SW_CPU_CLOCK && attr.config != PERF_COUNT_SW_TASK_CLOCK {
        return -95;
    }
    let fd = NEXT_FD.fetch_add(1, Ordering::AcqRel);
    let id = NEXT_ID.fetch_add(1, Ordering::AcqRel) as u64;
    let now = crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000;
    let ev = Arc::new(PerfEvent {
        fd,
        id,
        attr,
        inner: Mutex::new(PerfEventInner { open_ts_ns: now }),
    });
    EVENTS.lock().push(ev);
    fd as i64
}

/// `perf_event_read_value(fd) -> u64`.  Synthetic: returns ns elapsed since open.
pub fn perf_event_read_value(fd: i32) -> Option<u64> {
    perf_event_read_record(fd).map(|record| record.value)
}

pub fn perf_event_read_record(fd: i32) -> Option<PerfReadRecord> {
    let g = EVENTS.lock();
    for ev in g.iter() {
        if ev.fd == fd {
            let inner = ev.inner.lock();
            let now = crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000;
            let elapsed = now.saturating_sub(inner.open_ts_ns);
            return Some(PerfReadRecord {
                value: elapsed,
                time_enabled: if ev.attr.read_format & PERF_FORMAT_TOTAL_TIME_ENABLED != 0 {
                    Some(elapsed)
                } else {
                    None
                },
                time_running: if ev.attr.read_format & PERF_FORMAT_TOTAL_TIME_RUNNING != 0 {
                    Some(elapsed)
                } else {
                    None
                },
                id: if ev.attr.read_format & PERF_FORMAT_ID != 0 {
                    Some(ev.id)
                } else {
                    None
                },
            });
        }
    }
    None
}

/// `perf_pmu_register` - `vendor/linux/kernel/events/core.c:12838`.
///
/// Lupos supports the `perf_event_open` software-clock syscall surface, but not
/// registration of driver-owned hardware PMUs. Fail closed so module callers
/// can take their normal Linux error paths without a fabricated PMU.
pub unsafe extern "C" fn linux_perf_pmu_register(
    _pmu: *mut c_void,
    _name: *const c_char,
    _type: i32,
) -> i32 {
    -EOPNOTSUPP
}

/// `perf_pmu_unregister` - `vendor/linux/kernel/events/core.c:13024`.
pub unsafe extern "C" fn linux_perf_pmu_unregister(_pmu: *mut c_void) -> i32 {
    0
}

/// `perf_event_sysfs_show` - `vendor/linux/kernel/events/core.c:15339`.
pub unsafe extern "C" fn linux_perf_event_sysfs_show(
    _dev: *mut c_void,
    _attr: *mut c_void,
    _page: *mut c_char,
) -> isize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_event_attr_size_is_120_or_more() {
        // Linux's attr struct is 128 bytes in modern kernels (varies with version).
        // We require ≥120 to leave room for the documented field set.
        assert!(core::mem::size_of::<PerfEventAttr>() >= 120);
    }

    #[test]
    fn sys_perf_event_open_returns_fd_for_sw_cpu_clock() {
        let mut a = PerfEventAttr::default();
        a.type_ = PERF_TYPE_SOFTWARE;
        a.size = core::mem::size_of::<PerfEventAttr>() as u32;
        a.config = PERF_COUNT_SW_CPU_CLOCK;
        let fd = unsafe { sys_perf_event_open(&a, 0, -1, -1, 0) };
        assert!(fd > 0);
    }

    #[test]
    fn sys_perf_event_open_rejects_hardware() {
        let mut a = PerfEventAttr::default();
        a.type_ = PERF_TYPE_HARDWARE;
        let fd = unsafe { sys_perf_event_open(&a, 0, -1, -1, 0) };
        assert_eq!(fd, -95);
    }

    #[test]
    fn perf_read_record_honors_read_format_bits() {
        let a = PerfEventAttr {
            type_: PERF_TYPE_SOFTWARE,
            size: core::mem::size_of::<PerfEventAttr>() as u32,
            config: PERF_COUNT_SW_CPU_CLOCK,
            read_format: PERF_FORMAT_TOTAL_TIME_ENABLED
                | PERF_FORMAT_TOTAL_TIME_RUNNING
                | PERF_FORMAT_ID,
            ..PerfEventAttr::default()
        };
        let fd = unsafe { sys_perf_event_open(&a, 0, -1, -1, 0) };
        assert!(fd > 0);
        let record = perf_event_read_record(fd as i32).expect("perf read");
        assert!(record.time_enabled.is_some());
        assert!(record.time_running.is_some());
        assert!(record.id.unwrap_or(0) > 0);
    }

    #[test]
    fn syscall_m78_security_bpf_perf_parity() {
        assert_eq!(
            unsafe { sys_perf_event_open(core::ptr::null(), 0, -1, -1, 0) },
            -22
        );
        let mut a = PerfEventAttr {
            type_: PERF_TYPE_SOFTWARE,
            size: core::mem::size_of::<PerfEventAttr>() as u32,
            config: PERF_COUNT_SW_TASK_CLOCK,
            ..PerfEventAttr::default()
        };
        assert!(unsafe { sys_perf_event_open(&a, 0, -1, -1, 0) } > 0);
        a.config = 9999;
        assert_eq!(unsafe { sys_perf_event_open(&a, 0, -1, -1, 0) }, -95);
    }
}
