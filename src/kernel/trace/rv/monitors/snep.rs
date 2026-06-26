//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/snep/snep.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/snep/snep.c
//! RV monitor: schedule does not enable preempt.

pub const MONITOR_NAME: &str = "snep";
pub const MONITOR_DESCRIPTION: &str = "schedule does not enable preempt.";
pub const MODULE_AUTHOR: &str = "Gabriele Monaco <gmonaco@redhat.com>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnepState {
    NonSchedulingContext,
    SchedulingContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnepEvent {
    PreemptDisable,
    PreemptEnable,
    ScheduleEntry,
    ScheduleExit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnepMonitor {
    pub state: SnepState,
    pub violated: bool,
}

impl SnepMonitor {
    pub const fn new() -> Self {
        Self {
            state: SnepState::NonSchedulingContext,
            violated: false,
        }
    }

    pub fn event(&mut self, event: SnepEvent) -> bool {
        match snep_transition(self.state, event) {
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

impl Default for SnepMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn snep_transition(state: SnepState, event: SnepEvent) -> Option<SnepState> {
    match (state, event) {
        (SnepState::NonSchedulingContext, SnepEvent::PreemptDisable) => {
            Some(SnepState::NonSchedulingContext)
        }
        (SnepState::NonSchedulingContext, SnepEvent::PreemptEnable) => {
            Some(SnepState::NonSchedulingContext)
        }
        (SnepState::NonSchedulingContext, SnepEvent::ScheduleEntry) => {
            Some(SnepState::SchedulingContext)
        }
        (SnepState::SchedulingContext, SnepEvent::ScheduleExit) => {
            Some(SnepState::NonSchedulingContext)
        }
        _ => None,
    }
}

pub const fn snep_final_state(state: SnepState) -> bool {
    matches!(state, SnepState::NonSchedulingContext)
}

pub const fn no_event_during_preempt(event_fired: bool, in_preempt: bool) -> bool {
    !(event_fired && in_preempt)
}

pub const fn preempt_enable_during_schedule_violates(in_schedule: bool) -> bool {
    in_schedule
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snep_automaton_matches_linux_header_and_trace_hooks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/snep/snep.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/snep/snep.h"
        ));
        assert!(source.contains("#define MODULE_NAME \"snep\""));
        assert!(source.contains("rv_attach_trace_probe(\"snep\", preempt_disable"));
        assert!(source.contains("rv_attach_trace_probe(\"snep\", preempt_enable"));
        assert!(source.contains("rv_attach_trace_probe(\"snep\", sched_entry_tp"));
        assert!(source.contains("rv_attach_trace_probe(\"snep\", sched_exit_tp"));
        assert!(source.contains("rv_register_monitor(&rv_this, &rv_sched);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(header.contains("non_scheduling_context_snep"));
        assert!(header.contains("scheduling_contex_snep"));
        assert!(header.contains("schedule_exit_snep"));
        assert!(header.contains("non_scheduling_context_snep,"));
        assert!(header.contains("scheduling_contex_snep,"));

        let mut monitor = SnepMonitor::new();
        assert!(monitor.event(SnepEvent::PreemptDisable));
        assert!(monitor.event(SnepEvent::PreemptEnable));
        assert!(monitor.event(SnepEvent::ScheduleEntry));
        assert_eq!(monitor.state, SnepState::SchedulingContext);
        assert!(!monitor.event(SnepEvent::PreemptEnable));
        assert!(monitor.violated);

        let mut monitor = SnepMonitor::new();
        assert!(monitor.event(SnepEvent::ScheduleEntry));
        assert!(monitor.event(SnepEvent::ScheduleExit));
        assert_eq!(monitor.state, SnepState::NonSchedulingContext);
        assert!(snep_final_state(SnepState::NonSchedulingContext));
        assert!(!snep_final_state(SnepState::SchedulingContext));
        assert!(preempt_enable_during_schedule_violates(true));
    }
}
