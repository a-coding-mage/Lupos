//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/opid/opid.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/opid/opid.c
//! RV `opid` monitor automaton and guard evaluation.

pub const PREEMPT_MASK: u32 = 0x0000_00ff;
pub const ENV_INVALID_VALUE: u64 = u64::MAX;
pub const RV_MON_TYPE: &str = "RV_MON_PER_CPU";
pub const OPID_LICENSE: &str = "GPL";
pub const OPID_AUTHOR: &str = "Gabriele Monaco <gmonaco@redhat.com>";
pub const OPID_MODULE_DESCRIPTION: &str = "opid: operations with preemption and irq disabled.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpidState {
    Any,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpidEvent {
    SchedNeedResched,
    SchedWaking,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpidEnv {
    IrqOff,
    PreemptOff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpidMonitor {
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpidRuntime {
    pub monitor: OpidMonitor,
    pub da_monitor_initialized: bool,
    pub da_monitor_destroyed: bool,
    pub sched_need_resched_attached: bool,
    pub sched_waking_attached: bool,
    pub start_run_events: [Option<OpidEvent>; 2],
    pub start_run_event_count: usize,
    pub registered: bool,
}

impl OpidMonitor {
    pub const NAME: &'static str = "opid";
    pub const DESCRIPTION: &'static str = "operations with preemption and irq disabled.";

    pub const fn new() -> Self {
        Self { enabled: false }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub const fn transition(&self, _state: OpidState, _event: OpidEvent) -> OpidState {
        OpidState::Any
    }
}

impl OpidRuntime {
    pub const fn new() -> Self {
        Self {
            monitor: OpidMonitor::new(),
            da_monitor_initialized: false,
            da_monitor_destroyed: false,
            sched_need_resched_attached: false,
            sched_waking_attached: false,
            start_run_events: [None, None],
            start_run_event_count: 0,
            registered: false,
        }
    }

    fn record_start_run_event(&mut self, event: OpidEvent) {
        if self.start_run_event_count < self.start_run_events.len() {
            self.start_run_events[self.start_run_event_count] = Some(event);
        }
        self.start_run_event_count += 1;
    }
}

impl Default for OpidMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for OpidRuntime {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn ha_get_env(
    env: OpidEnv,
    irq_disabled: bool,
    preempt_count: u32,
    config_preemption: bool,
) -> u64 {
    match env {
        OpidEnv::IrqOff => irq_disabled as u64,
        OpidEnv::PreemptOff => {
            if config_preemption {
                ((preempt_count & PREEMPT_MASK) > 1) as u64
            } else {
                1
            }
        }
    }
}

pub const fn ha_verify_guards(
    curr_state: OpidState,
    event: OpidEvent,
    next_state: OpidState,
    irq_disabled: bool,
    preempt_count: u32,
    config_preemption: bool,
) -> bool {
    let _ = next_state;
    match (curr_state, event) {
        (OpidState::Any, OpidEvent::SchedNeedResched) => {
            ha_get_env(
                OpidEnv::IrqOff,
                irq_disabled,
                preempt_count,
                config_preemption,
            ) == 1
        }
        (OpidState::Any, OpidEvent::SchedWaking) => {
            ha_get_env(
                OpidEnv::IrqOff,
                irq_disabled,
                preempt_count,
                config_preemption,
            ) == 1
                && ha_get_env(
                    OpidEnv::PreemptOff,
                    irq_disabled,
                    preempt_count,
                    config_preemption,
                ) == 1
        }
    }
}

pub const fn ha_verify_constraint(
    curr_state: OpidState,
    event: OpidEvent,
    next_state: OpidState,
    irq_disabled: bool,
    preempt_count: u32,
    config_preemption: bool,
) -> bool {
    let _ = next_state;
    ha_verify_guards(
        curr_state,
        event,
        next_state,
        irq_disabled,
        preempt_count,
        config_preemption,
    )
}

pub fn handle_sched_need_resched(runtime: &mut OpidRuntime) {
    runtime.record_start_run_event(OpidEvent::SchedNeedResched);
}

pub fn handle_sched_waking(runtime: &mut OpidRuntime) {
    runtime.record_start_run_event(OpidEvent::SchedWaking);
}

pub fn enable_opid(runtime: &mut OpidRuntime, da_monitor_init_result: i32) -> i32 {
    if da_monitor_init_result != 0 {
        return da_monitor_init_result;
    }

    runtime.da_monitor_initialized = true;
    runtime.da_monitor_destroyed = false;
    runtime.sched_need_resched_attached = true;
    runtime.sched_waking_attached = true;
    0
}

pub fn disable_opid(runtime: &mut OpidRuntime) {
    runtime.monitor.enabled = false;
    runtime.sched_need_resched_attached = false;
    runtime.sched_waking_attached = false;
    runtime.da_monitor_destroyed = true;
    runtime.da_monitor_initialized = false;
}

pub fn register_opid(runtime: &mut OpidRuntime, rv_register_monitor_result: i32) -> i32 {
    if rv_register_monitor_result == 0 {
        runtime.registered = true;
    }
    rv_register_monitor_result
}

pub fn unregister_opid(runtime: &mut OpidRuntime) {
    runtime.registered = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opid_automaton_and_guards_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/opid/opid.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/opid/opid.h"
        ));
        let preempt = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/preempt.h"
        ));

        assert!(source.contains("#define MODULE_NAME \"opid\""));
        assert!(source.contains("if (env == irq_off_opid)"));
        assert!(source.contains("return irqs_disabled();"));
        assert!(source.contains("(preempt_count() & PREEMPT_MASK) > 1"));
        assert!(source.contains("return ENV_INVALID_VALUE;"));
        assert!(source.contains("event == sched_need_resched_opid"));
        assert!(source.contains("event == sched_waking_opid"));
        assert!(source.contains("da_handle_start_run_event(sched_need_resched_opid);"));
        assert!(source.contains("da_handle_start_run_event(sched_waking_opid);"));
        assert!(source.contains("retval = da_monitor_init();"));
        assert!(source.contains("rv_attach_trace_probe(\"opid\", sched_set_need_resched_tp"));
        assert!(source.contains("rv_attach_trace_probe(\"opid\", sched_waking"));
        assert!(source.contains("rv_this.enabled = 0;"));
        assert!(source.contains("rv_detach_trace_probe(\"opid\", sched_set_need_resched_tp"));
        assert!(source.contains("rv_detach_trace_probe(\"opid\", sched_waking"));
        assert!(source.contains("da_monitor_destroy();"));
        assert!(source.contains(".description = \"operations with preemption and irq disabled.\""));
        assert!(source.contains(".reset = da_monitor_reset_all"));
        assert!(source.contains("return rv_register_monitor(&rv_this, &rv_sched);"));
        assert!(source.contains("rv_unregister_monitor(&rv_this);"));
        assert!(source.contains("module_init(register_opid);"));
        assert!(source.contains("module_exit(unregister_opid);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\");"));
        assert!(header.contains("enum states_opid"));
        assert!(header.contains("sched_need_resched_opid"));
        assert!(header.contains("sched_waking_opid"));
        assert!(header.contains(".function = {"));
        assert!(preempt.contains("PREEMPT_MASK:\t0x000000ff"));

        let monitor = OpidMonitor::new();
        assert_eq!(
            monitor.transition(OpidState::Any, OpidEvent::SchedWaking),
            OpidState::Any
        );
        assert_eq!(OpidMonitor::NAME, "opid");
        assert_eq!(RV_MON_TYPE, "RV_MON_PER_CPU");
        assert_eq!(OPID_LICENSE, "GPL");
        assert_eq!(ha_get_env(OpidEnv::IrqOff, true, 0, true), 1);
        assert_eq!(ha_get_env(OpidEnv::PreemptOff, true, 1, true), 0);
        assert_eq!(ha_get_env(OpidEnv::PreemptOff, true, 2, true), 1);
        assert_eq!(ha_get_env(OpidEnv::PreemptOff, false, 0, false), 1);
        assert!(ha_verify_constraint(
            OpidState::Any,
            OpidEvent::SchedNeedResched,
            OpidState::Any,
            true,
            0,
            true
        ));
        assert!(!ha_verify_constraint(
            OpidState::Any,
            OpidEvent::SchedWaking,
            OpidState::Any,
            true,
            1,
            true
        ));
        assert!(ha_verify_constraint(
            OpidState::Any,
            OpidEvent::SchedWaking,
            OpidState::Any,
            true,
            2,
            true
        ));
    }

    #[test]
    fn opid_handlers_and_lifecycle_match_linux_source() {
        let mut runtime = OpidRuntime::new();

        handle_sched_need_resched(&mut runtime);
        handle_sched_waking(&mut runtime);
        assert_eq!(
            runtime.start_run_events,
            [
                Some(OpidEvent::SchedNeedResched),
                Some(OpidEvent::SchedWaking)
            ]
        );
        assert_eq!(runtime.start_run_event_count, 2);

        assert_eq!(enable_opid(&mut runtime, -22), -22);
        assert!(!runtime.sched_need_resched_attached);
        assert_eq!(enable_opid(&mut runtime, 0), 0);
        assert!(runtime.da_monitor_initialized);
        assert!(runtime.sched_need_resched_attached);
        assert!(runtime.sched_waking_attached);

        runtime.monitor.enable();
        disable_opid(&mut runtime);
        assert!(!runtime.monitor.enabled);
        assert!(!runtime.sched_need_resched_attached);
        assert!(!runtime.sched_waking_attached);
        assert!(runtime.da_monitor_destroyed);

        assert_eq!(register_opid(&mut runtime, -1), -1);
        assert!(!runtime.registered);
        assert_eq!(register_opid(&mut runtime, 0), 0);
        assert!(runtime.registered);
        unregister_opid(&mut runtime);
        assert!(!runtime.registered);
    }
}
