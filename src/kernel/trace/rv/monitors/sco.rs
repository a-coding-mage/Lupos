//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/sco/sco.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/sco/sco.c
//! RV monitor: scheduling context operations.

pub const MONITOR_NAME: &str = "sco";
pub const MONITOR_DESCRIPTION: &str = "scheduling context operations.";
pub const MODULE_AUTHOR: &str = "Gabriele Monaco <gmonaco@redhat.com>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScoState {
    ThreadContext,
    SchedulingContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScoEvent {
    SchedSetState,
    ScheduleEntry,
    ScheduleExit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoMonitor {
    pub state: ScoState,
    pub violated: bool,
}

impl ScoMonitor {
    pub const fn new() -> Self {
        Self {
            state: ScoState::ThreadContext,
            violated: false,
        }
    }

    pub fn event(&mut self, event: ScoEvent) -> bool {
        match sco_transition(self.state, event) {
            Some(next) => {
                self.state = next;
                true
            }
            None => {
                self.violated = true;
                false
            }
        }
    }
}

impl Default for ScoMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn sco_transition(state: ScoState, event: ScoEvent) -> Option<ScoState> {
    match (state, event) {
        (ScoState::ThreadContext, ScoEvent::SchedSetState) => Some(ScoState::ThreadContext),
        (ScoState::ThreadContext, ScoEvent::ScheduleEntry) => Some(ScoState::SchedulingContext),
        (ScoState::SchedulingContext, ScoEvent::ScheduleExit) => Some(ScoState::ThreadContext),
        _ => None,
    }
}

pub const fn sco_final_state(state: ScoState) -> bool {
    matches!(state, ScoState::ThreadContext)
}

pub fn class_order_ok(prev_class: u32, next_class: u32) -> bool {
    next_class >= prev_class || prev_class == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sco_automaton_matches_linux_header_and_trace_hooks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/sco/sco.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/sco/sco.h"
        ));
        assert!(source.contains("#define MODULE_NAME \"sco\""));
        assert!(source.contains("rv_attach_trace_probe(\"sco\", sched_set_state_tp"));
        assert!(source.contains("rv_attach_trace_probe(\"sco\", sched_entry_tp"));
        assert!(source.contains("rv_attach_trace_probe(\"sco\", sched_exit_tp"));
        assert!(source.contains("rv_register_monitor(&rv_this, &rv_sched);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(header.contains("enum states_sco"));
        assert!(header.contains("thread_context_sco"));
        assert!(header.contains("scheduling_context_sco"));
        assert!(header.contains("schedule_entry_sco"));
        assert!(header.contains(
            "{     thread_context_sco, scheduling_context_sco,          INVALID_STATE }"
        ));
        assert!(header.contains(
            "{          INVALID_STATE,          INVALID_STATE,     thread_context_sco }"
        ));

        let mut monitor = ScoMonitor::new();
        assert!(monitor.event(ScoEvent::SchedSetState));
        assert!(monitor.event(ScoEvent::ScheduleEntry));
        assert_eq!(monitor.state, ScoState::SchedulingContext);
        assert!(!monitor.event(ScoEvent::ScheduleEntry));
        assert!(monitor.violated);

        let mut monitor = ScoMonitor::new();
        assert!(monitor.event(ScoEvent::ScheduleEntry));
        assert!(monitor.event(ScoEvent::ScheduleExit));
        assert_eq!(monitor.state, ScoState::ThreadContext);
        assert!(sco_final_state(ScoState::ThreadContext));
        assert!(!sco_final_state(ScoState::SchedulingContext));
    }
}
