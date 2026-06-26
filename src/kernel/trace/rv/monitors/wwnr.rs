//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/wwnr/wwnr.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/wwnr/wwnr.c
//! RV monitor: wakeup while not running.

pub const MONITOR_NAME: &str = "wwnr";
pub const MONITOR_DESCRIPTION: &str = "wakeup while not running per-task testing model.";
pub const MODULE_AUTHOR: &str = "Daniel Bristot de Oliveira <bristot@kernel.org>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WwnrState {
    NotRunning,
    Running,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WwnrEvent {
    SwitchIn,
    SwitchOut,
    Wakeup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WwnrMonitor {
    pub state: WwnrState,
    pub violated: bool,
}

impl WwnrMonitor {
    pub const fn new() -> Self {
        Self {
            state: WwnrState::NotRunning,
            violated: false,
        }
    }

    pub fn event(&mut self, event: WwnrEvent) -> bool {
        match wwnr_transition(self.state, event) {
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

    pub fn handle_switch_out(&mut self) -> bool {
        self.event(WwnrEvent::SwitchOut)
    }

    pub fn handle_switch_in(&mut self) -> bool {
        self.event(WwnrEvent::SwitchIn)
    }

    pub fn handle_wakeup(&mut self) -> bool {
        self.event(WwnrEvent::Wakeup)
    }
}

impl Default for WwnrMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn wwnr_transition(state: WwnrState, event: WwnrEvent) -> Option<WwnrState> {
    match (state, event) {
        (WwnrState::NotRunning, WwnrEvent::SwitchIn) => Some(WwnrState::Running),
        (WwnrState::NotRunning, WwnrEvent::Wakeup) => Some(WwnrState::NotRunning),
        (WwnrState::Running, WwnrEvent::SwitchOut) => Some(WwnrState::NotRunning),
        _ => None,
    }
}

pub const fn wwnr_final_state(state: WwnrState) -> bool {
    matches!(state, WwnrState::NotRunning)
}

pub fn check(target_pid: i32, target_running: bool, wakeup_called: bool) -> bool {
    let _ = target_pid;
    !(target_running && wakeup_called)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wwnr_automaton_matches_linux_header_and_trace_hooks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/wwnr/wwnr.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/wwnr/wwnr.h"
        ));
        assert!(source.contains("#define MODULE_NAME \"wwnr\""));
        assert!(source.contains("rv_attach_trace_probe(\"wwnr\", sched_switch, handle_switch);"));
        assert!(source.contains("rv_attach_trace_probe(\"wwnr\", sched_wakeup, handle_wakeup);"));
        assert!(source.contains("rv_register_monitor(&rv_this, NULL);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(header.contains("enum states_wwnr"));
        assert!(header.contains("not_running_wwnr"));
        assert!(header.contains("running_wwnr"));
        assert!(header.contains("wakeup_wwnr"));
        assert!(header.contains("{       running_wwnr,      INVALID_STATE,   not_running_wwnr }"));
        assert!(header.contains("{      INVALID_STATE,   not_running_wwnr,      INVALID_STATE }"));

        let mut monitor = WwnrMonitor::new();
        assert!(monitor.handle_wakeup());
        assert_eq!(monitor.state, WwnrState::NotRunning);
        assert!(monitor.handle_switch_in());
        assert_eq!(monitor.state, WwnrState::Running);
        assert!(!monitor.handle_wakeup());
        assert!(monitor.violated);
        assert!(!check(1, true, true));
        assert!(check(1, false, true));
        assert!(wwnr_final_state(WwnrState::NotRunning));
        assert!(!wwnr_final_state(WwnrState::Running));
    }
}
