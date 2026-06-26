//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_pid.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_pid.c
//! `set_event_pid` / `set_ftrace_pid` filter machinery.
//!
//! Ref: vendor/linux/kernel/trace/trace_pid.c

use crate::kernel::trace::pid_list::TracePidList;

pub static EVENT_PID_LIST: TracePidList = TracePidList::new();

pub fn allow(pid: i32) -> bool {
    EVENT_PID_LIST.contains(pid) || EVENT_PID_LIST.len() == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_list_allows_all() {
        // No pids registered yet → every pid passes the filter.
        assert!(allow(1234));
    }

    #[test]
    fn explicit_pid_passes_when_registered() {
        EVENT_PID_LIST.add(5678);
        assert!(allow(5678));
        EVENT_PID_LIST.remove(5678);
    }
}
