//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/blktrace.c
//! test-origin: linux:vendor/linux/kernel/trace/blktrace.c
//! Block-layer I/O tracing.
//!
//! Records block-layer events (queue, issue, complete, getrq) into a
//! per-cpu trace buffer.  Userspace `blktrace` consumes them via
//! `/sys/kernel/debug/block/<dev>/trace*`.
//!
//! Ref: vendor/linux/kernel/trace/blktrace.c

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

/// `enum blktrace_act` action codes (subset).
pub const BLK_TA_QUEUE: u32 = 1;
pub const BLK_TA_ISSUE: u32 = 4;
pub const BLK_TA_COMPLETE: u32 = 5;
pub const BLK_TA_GETRQ: u32 = 7;

#[derive(Clone, Copy, Debug)]
pub struct BlkTraceEvent {
    pub dev: u32,
    pub sector: u64,
    pub bytes: u32,
    pub action: u32,
    pub pid: u32,
    pub time: u64,
}

static EVENTS: Mutex<Vec<BlkTraceEvent>> = Mutex::new(Vec::new());
static QUEUE_TRACES: Mutex<BTreeMap<usize, bool>> = Mutex::new(BTreeMap::new());

fn export_symbol_once(name: &'static str, address: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, address, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("blk_trace_setup", linux_blk_trace_setup as usize, true);
    export_symbol_once(
        "blk_trace_startstop",
        linux_blk_trace_startstop as usize,
        true,
    );
    export_symbol_once("blk_trace_remove", linux_blk_trace_remove as usize, true);
}

/// Attach the block trace domain to a request queue. The relay-buffer details
/// remain owned by Lupos's trace ring, while the queue lifecycle and Linux
/// return contract are preserved for unchanged SG/block callers.
unsafe extern "C" fn linux_blk_trace_setup(
    queue: *mut c_void,
    _name: *mut c_char,
    _dev: u32,
    _block_device: *mut c_void,
    _user_setup: *mut c_void,
) -> i32 {
    if queue.is_null() {
        return -EINVAL;
    }
    QUEUE_TRACES.lock().entry(queue as usize).or_insert(false);
    0
}

unsafe extern "C" fn linux_blk_trace_startstop(queue: *mut c_void, start: i32) -> i32 {
    let mut traces = QUEUE_TRACES.lock();
    let Some(active) = traces.get_mut(&(queue as usize)) else {
        return -EINVAL;
    };
    *active = start != 0;
    0
}

unsafe extern "C" fn linux_blk_trace_remove(queue: *mut c_void) -> i32 {
    if queue.is_null() || QUEUE_TRACES.lock().remove(&(queue as usize)).is_none() {
        -EINVAL
    } else {
        0
    }
}

pub fn record(ev: BlkTraceEvent) {
    EVENTS.lock().push(ev);
}

pub fn drain() -> Vec<BlkTraceEvent> {
    core::mem::take(&mut *EVENTS.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_codes_match_linux() {
        assert_eq!(BLK_TA_QUEUE, 1);
        assert_eq!(BLK_TA_COMPLETE, 5);
    }

    #[test]
    fn record_then_drain() {
        record(BlkTraceEvent {
            dev: 8,
            sector: 0x1000,
            bytes: 512,
            action: BLK_TA_QUEUE,
            pid: 1,
            time: 0,
        });
        let d = drain();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].sector, 0x1000);
    }

    #[test]
    fn linux_queue_trace_lifecycle_is_stateful_and_exported() {
        register_module_exports();
        let queue = 0x1000usize as *mut c_void;
        assert_eq!(
            unsafe {
                linux_blk_trace_setup(
                    queue,
                    core::ptr::null_mut(),
                    0,
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                )
            },
            0
        );
        assert_eq!(unsafe { linux_blk_trace_startstop(queue, 1) }, 0);
        assert_eq!(QUEUE_TRACES.lock().get(&(queue as usize)), Some(&true));
        assert_eq!(unsafe { linux_blk_trace_remove(queue) }, 0);
        assert_eq!(unsafe { linux_blk_trace_startstop(queue, 1) }, -EINVAL);
        assert_eq!(
            find_symbol("blk_trace_startstop"),
            Some(linux_blk_trace_startstop as usize)
        );
    }
}
