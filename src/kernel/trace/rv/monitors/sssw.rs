//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/sssw/sssw.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/sssw/sssw.c
//! RV monitor: set-state sleep and wakeup.

pub const MONITOR_NAME: &str = "sssw";
pub const MONITOR_DESCRIPTION: &str = "set state sleep and wakeup.";
pub const MODULE_AUTHOR: &str = "Gabriele Monaco <gmonaco@redhat.com>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SsswState {
    Runnable,
    SignalWakeup,
    Sleepable,
    Sleeping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SsswEvent {
    SchedSetStateRunnable,
    SchedSetStateSleepable,
    SchedSwitchBlocking,
    SchedSwitchIn,
    SchedSwitchPreempt,
    SchedSwitchSuspend,
    SchedSwitchYield,
    SchedWakeup,
    SignalDeliver,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SsswMonitor {
    pub state: SsswState,
    pub violated: bool,
}

impl SsswMonitor {
    pub const fn new() -> Self {
        Self {
            state: SsswState::Runnable,
            violated: false,
        }
    }

    pub fn event(&mut self, event: SsswEvent) -> bool {
        match sssw_transition(self.state, event) {
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

impl Default for SsswMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn sssw_transition(state: SsswState, event: SsswEvent) -> Option<SsswState> {
    use SsswEvent::*;
    use SsswState::*;

    match (state, event) {
        (Runnable, SchedSetStateRunnable) => Some(Runnable),
        (Runnable, SchedSetStateSleepable) => Some(Sleepable),
        (Runnable, SchedSwitchBlocking) => Some(Sleeping),
        (Runnable, SchedSwitchIn) => Some(Runnable),
        (Runnable, SchedSwitchPreempt) => Some(Runnable),
        (Runnable, SchedSwitchYield) => Some(Runnable),
        (Runnable, SchedWakeup) => Some(Runnable),
        (Runnable, SignalDeliver) => Some(Runnable),
        (SignalWakeup, SchedSetStateSleepable) => Some(Sleepable),
        (SignalWakeup, SchedSwitchIn) => Some(SignalWakeup),
        (SignalWakeup, SchedSwitchPreempt) => Some(SignalWakeup),
        (SignalWakeup, SchedSwitchYield) => Some(SignalWakeup),
        (SignalWakeup, SchedWakeup) => Some(SignalWakeup),
        (SignalWakeup, SignalDeliver) => Some(Runnable),
        (Sleepable, SchedSetStateRunnable) => Some(Runnable),
        (Sleepable, SchedSetStateSleepable) => Some(Sleepable),
        (Sleepable, SchedSwitchBlocking) => Some(Sleeping),
        (Sleepable, SchedSwitchIn) => Some(Sleepable),
        (Sleepable, SchedSwitchPreempt) => Some(Sleepable),
        (Sleepable, SchedSwitchSuspend) => Some(Sleeping),
        (Sleepable, SchedSwitchYield) => Some(SignalWakeup),
        (Sleepable, SchedWakeup) => Some(Runnable),
        (Sleepable, SignalDeliver) => Some(Sleepable),
        (Sleeping, SchedWakeup) => Some(Runnable),
        _ => None,
    }
}

pub const fn sssw_final_state(state: SsswState) -> bool {
    matches!(state, SsswState::Runnable)
}

pub fn sleep_wakeup_balanced(sleeps: u32, wakeups: u32) -> bool {
    sleeps == wakeups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sssw_automaton_matches_linux_header_and_trace_hooks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/sssw/sssw.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/sssw/sssw.h"
        ));
        assert!(source.contains("#define MODULE_NAME \"sssw\""));
        assert!(source.contains("handle_sched_set_state"));
        assert!(source.contains("da_handle_start_event(tsk, sched_set_state_runnable_sssw);"));
        assert!(source.contains("da_handle_event(tsk, sched_set_state_sleepable_sssw);"));
        assert!(source.contains("da_handle_event(prev, sched_switch_preempt_sssw);"));
        assert!(source.contains("da_handle_event(prev, sched_switch_yield_sssw);"));
        assert!(source.contains("da_handle_event(prev, sched_switch_blocking_sssw);"));
        assert!(source.contains("da_handle_event(prev, sched_switch_suspend_sssw);"));
        assert!(source.contains("da_handle_event(next, sched_switch_in_sssw);"));
        assert!(source.contains("da_handle_start_event(p, sched_wakeup_sssw);"));
        assert!(source.contains("da_handle_event(current, signal_deliver_sssw);"));
        assert!(source.contains("rv_attach_trace_probe(\"sssw\", sched_switch"));
        assert!(source.contains("rv_register_monitor(&rv_this, &rv_sched);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(header.contains("enum states_sssw"));
        assert!(header.contains("runnable_sssw"));
        assert!(header.contains("signal_wakeup_sssw"));
        assert!(header.contains("sleepable_sssw"));
        assert!(header.contains("sleeping_sssw"));
        assert!(header.contains(".initial_state = runnable_sssw"));
        assert!(header.contains(".final_states = { 1, 0, 0, 0 }"));

        let mut monitor = SsswMonitor::new();
        assert!(monitor.event(SsswEvent::SchedSetStateSleepable));
        assert_eq!(monitor.state, SsswState::Sleepable);
        assert!(monitor.event(SsswEvent::SchedSwitchBlocking));
        assert_eq!(monitor.state, SsswState::Sleeping);
        assert!(monitor.event(SsswEvent::SchedWakeup));
        assert_eq!(monitor.state, SsswState::Runnable);
        assert!(!monitor.event(SsswEvent::SchedSwitchSuspend));
        assert!(monitor.violated);
        assert!(sssw_final_state(SsswState::Runnable));
        assert!(!sssw_final_state(SsswState::Sleeping));
    }

    #[test]
    fn imbalance_is_violation() {
        assert!(!sleep_wakeup_balanced(3, 2));
        assert!(sleep_wakeup_balanced(2, 2));
    }
}
