//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/wip/wip.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/wip/wip.c
//! RV monitor: wakeup in preemptive context.

pub const MONITOR_NAME: &str = "wip";
pub const MONITOR_DESCRIPTION: &str = "wakeup in preemptive per-cpu testing monitor.";
pub const MODULE_AUTHOR: &str = "Daniel Bristot de Oliveira <bristot@kernel.org>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WipState {
    Preemptive,
    NonPreemptive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WipEvent {
    PreemptDisable,
    PreemptEnable,
    SchedWaking,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WipMonitor {
    pub state: WipState,
    pub violated: bool,
}

impl WipMonitor {
    pub const fn new() -> Self {
        Self {
            state: WipState::Preemptive,
            violated: false,
        }
    }

    pub fn event(&mut self, event: WipEvent) -> bool {
        match wip_transition(self.state, event) {
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

impl Default for WipMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn wip_transition(state: WipState, event: WipEvent) -> Option<WipState> {
    match (state, event) {
        (WipState::Preemptive, WipEvent::PreemptDisable) => Some(WipState::NonPreemptive),
        (WipState::NonPreemptive, WipEvent::PreemptEnable) => Some(WipState::Preemptive),
        (WipState::NonPreemptive, WipEvent::SchedWaking) => Some(WipState::NonPreemptive),
        _ => None,
    }
}

pub const fn wip_final_state(state: WipState) -> bool {
    matches!(state, WipState::Preemptive)
}

pub fn wake_in_preempt(in_non_preemptive: bool, wake_called: bool) -> bool {
    in_non_preemptive && wake_called
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wip_automaton_matches_linux_header_and_trace_hooks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/wip/wip.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/wip/wip.h"
        ));
        assert!(source.contains("#define MODULE_NAME \"wip\""));
        assert!(source.contains("rv_attach_trace_probe(\"wip\", preempt_enable"));
        assert!(source.contains("rv_attach_trace_probe(\"wip\", sched_waking"));
        assert!(source.contains("rv_attach_trace_probe(\"wip\", preempt_disable"));
        assert!(source.contains("rv_register_monitor(&rv_this, NULL);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(header.contains("enum states_wip"));
        assert!(header.contains("preemptive_wip"));
        assert!(header.contains("non_preemptive_wip"));
        assert!(header.contains("sched_waking_wip"));
        assert!(header.contains("{ non_preemptive_wip,      INVALID_STATE,      INVALID_STATE }"));
        assert!(header.contains("{      INVALID_STATE,     preemptive_wip, non_preemptive_wip }"));

        let mut monitor = WipMonitor::new();
        assert!(!monitor.event(WipEvent::SchedWaking));
        assert!(monitor.violated);

        let mut monitor = WipMonitor::new();
        assert!(monitor.event(WipEvent::PreemptDisable));
        assert_eq!(monitor.state, WipState::NonPreemptive);
        assert!(monitor.event(WipEvent::SchedWaking));
        assert!(monitor.event(WipEvent::PreemptEnable));
        assert_eq!(monitor.state, WipState::Preemptive);
        assert!(wip_final_state(WipState::Preemptive));
        assert!(!wip_final_state(WipState::NonPreemptive));
        assert!(wake_in_preempt(true, true));
        assert!(!wake_in_preempt(false, true));
    }
}
