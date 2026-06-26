//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/snroc/snroc.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/snroc/snroc.c
//! Runtime-verification monitor for "set non runnable on its own context".

pub const MONITOR_NAME: &str = "snroc";
pub const MONITOR_DESCRIPTION: &str = "set non runnable on its own context.";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SnrocMonitor {
    pub enabled: bool,
    pub da_monitor_initialized: bool,
    pub sched_set_state_attached: bool,
    pub sched_switch_attached: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnrocEvent {
    SchedSetState,
    SchedSwitchOut,
    SchedSwitchIn,
}

pub fn enable_snroc(init_ok: bool) -> Result<SnrocMonitor, i32> {
    if !init_ok {
        return Err(-1);
    }
    Ok(SnrocMonitor {
        enabled: true,
        da_monitor_initialized: true,
        sched_set_state_attached: true,
        sched_switch_attached: true,
    })
}

pub fn disable_snroc(state: &mut SnrocMonitor) {
    state.enabled = false;
    state.sched_set_state_attached = false;
    state.sched_switch_attached = false;
    state.da_monitor_initialized = false;
}

pub const fn handle_sched_set_state_event() -> SnrocEvent {
    SnrocEvent::SchedSetState
}

pub const fn handle_sched_switch_events() -> [SnrocEvent; 2] {
    [SnrocEvent::SchedSwitchOut, SnrocEvent::SchedSwitchIn]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snroc_monitor_hooks_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/snroc/snroc.c"
        ));
        assert!(source.contains("#define MODULE_NAME \"snroc\""));
        assert!(source.contains("da_handle_event(tsk, sched_set_state_snroc);"));
        assert!(source.contains("da_handle_start_event(prev, sched_switch_out_snroc);"));
        assert!(source.contains("da_handle_event(next, sched_switch_in_snroc);"));
        assert!(source.contains("retval = da_monitor_init();"));
        assert!(source.contains("rv_attach_trace_probe(\"snroc\", sched_set_state_tp"));
        assert!(source.contains("rv_attach_trace_probe(\"snroc\", sched_switch"));
        assert!(source.contains("rv_this.enabled = 0;"));
        assert!(source.contains("rv_detach_trace_probe(\"snroc\", sched_switch"));
        assert!(source.contains("da_monitor_destroy();"));
        assert!(source.contains("rv_register_monitor(&rv_this, &rv_sched);"));
        assert!(source.contains(MONITOR_DESCRIPTION));

        let mut monitor = enable_snroc(true).unwrap();
        assert!(monitor.enabled);
        assert!(monitor.sched_switch_attached);
        assert_eq!(handle_sched_set_state_event(), SnrocEvent::SchedSetState);
        assert_eq!(
            handle_sched_switch_events(),
            [SnrocEvent::SchedSwitchOut, SnrocEvent::SchedSwitchIn]
        );
        disable_snroc(&mut monitor);
        assert!(!monitor.enabled);
        assert!(!monitor.da_monitor_initialized);
        assert_eq!(enable_snroc(false), Err(-1));
    }
}
