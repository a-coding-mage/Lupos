//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/scpd/scpd.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/scpd/scpd.c
//! RV monitor: schedule called with preemption disabled.

pub const MONITOR_NAME: &str = "scpd";
pub const MONITOR_DESCRIPTION: &str = "schedule called with preemption disabled.";
pub const MODULE_AUTHOR: &str = "Gabriele Monaco <gmonaco@redhat.com>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScpdState {
    CantSched,
    CanSched,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScpdEvent {
    PreemptDisable,
    PreemptEnable,
    ScheduleEntry,
    ScheduleExit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScpdMonitor {
    pub state: ScpdState,
    pub violated: bool,
}

impl ScpdMonitor {
    pub const fn new() -> Self {
        Self {
            state: ScpdState::CantSched,
            violated: false,
        }
    }

    pub fn event(&mut self, event: ScpdEvent) -> bool {
        match scpd_transition(self.state, event) {
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

impl Default for ScpdMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn scpd_transition(state: ScpdState, event: ScpdEvent) -> Option<ScpdState> {
    match (state, event) {
        (ScpdState::CantSched, ScpdEvent::PreemptDisable) => Some(ScpdState::CanSched),
        (ScpdState::CanSched, ScpdEvent::PreemptEnable) => Some(ScpdState::CantSched),
        (ScpdState::CanSched, ScpdEvent::ScheduleEntry) => Some(ScpdState::CanSched),
        (ScpdState::CanSched, ScpdEvent::ScheduleExit) => Some(ScpdState::CanSched),
        _ => None,
    }
}

pub const fn scpd_final_state(state: ScpdState) -> bool {
    matches!(state, ScpdState::CantSched)
}

pub const fn schedule_called(preempt_disabled: bool) -> bool {
    !preempt_disabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scpd_automaton_matches_linux_header_and_trace_hooks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/scpd/scpd.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/scpd/scpd.h"
        ));
        assert!(source.contains("#define MODULE_NAME \"scpd\""));
        assert!(source.contains("rv_attach_trace_probe(\"scpd\", preempt_disable"));
        assert!(source.contains("rv_attach_trace_probe(\"scpd\", preempt_enable"));
        assert!(source.contains("rv_attach_trace_probe(\"scpd\", sched_entry_tp"));
        assert!(source.contains("rv_attach_trace_probe(\"scpd\", sched_exit_tp"));
        assert!(source.contains("rv_register_monitor(&rv_this, &rv_sched);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(header.contains("cant_sched_scpd"));
        assert!(header.contains("can_sched_scpd"));
        assert!(header.contains("schedule_entry_scpd"));
        assert!(header.contains(
            "{     can_sched_scpd,     INVALID_STATE,     INVALID_STATE,     INVALID_STATE }"
        ));
        assert!(header.contains(
            "{     INVALID_STATE,    cant_sched_scpd,     can_sched_scpd,     can_sched_scpd }"
        ));

        let mut monitor = ScpdMonitor::new();
        assert!(!monitor.event(ScpdEvent::ScheduleEntry));
        assert!(monitor.violated);

        let mut monitor = ScpdMonitor::new();
        assert!(monitor.event(ScpdEvent::PreemptDisable));
        assert!(monitor.event(ScpdEvent::ScheduleEntry));
        assert!(monitor.event(ScpdEvent::ScheduleExit));
        assert!(monitor.event(ScpdEvent::PreemptEnable));
        assert_eq!(monitor.state, ScpdState::CantSched);
        assert!(scpd_final_state(ScpdState::CantSched));
        assert!(!scpd_final_state(ScpdState::CanSched));
        assert!(!schedule_called(true));
        assert!(schedule_called(false));
    }
}
