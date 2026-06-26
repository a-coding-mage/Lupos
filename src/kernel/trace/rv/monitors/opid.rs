//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/opid
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/opid
//! RV monitor wrapper for the `opid` automaton.

pub mod opid;

pub const LINUX_SOURCE_FILES: [&str; 4] = ["Kconfig", "opid.c", "opid.h", "opid_trace.h"];
pub const KCONFIG_SYMBOL: &str = "RV_MON_OPID";
pub const TRACE_EVENT_PREFIX: &str = "opid";

pub use opid::{
    OpidEnv, OpidEvent, OpidMonitor, OpidState, PREEMPT_MASK, ha_get_env, ha_verify_constraint,
    ha_verify_guards,
};

#[cfg(test)]
mod tests {
    use super::*;

    const KCONFIG: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/kernel/trace/rv/monitors/opid/Kconfig"
    ));
    const OPID_C: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/kernel/trace/rv/monitors/opid/opid.c"
    ));
    const OPID_H: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/kernel/trace/rv/monitors/opid/opid.h"
    ));
    const OPID_TRACE_H: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/kernel/trace/rv/monitors/opid/opid_trace.h"
    ));

    #[test]
    fn wrapper_source_set_matches_linux_opid_directory() {
        assert_eq!(
            LINUX_SOURCE_FILES,
            ["Kconfig", "opid.c", "opid.h", "opid_trace.h"]
        );
        assert!(KCONFIG.contains("config RV_MON_OPID"));
        assert!(KCONFIG.contains("depends on RV_MON_SCHED"));
        assert!(KCONFIG.contains("select HA_MON_EVENTS_IMPLICIT"));
        assert!(KCONFIG.contains("Monitor to ensure operations like wakeup and need resched"));
    }

    #[test]
    fn wrapper_exports_match_linux_monitor_metadata() {
        assert!(OPID_C.contains("#define MODULE_NAME \"opid\""));
        assert!(OPID_C.contains("#define RV_MON_TYPE RV_MON_PER_CPU"));
        assert!(OPID_C.contains("#include \"opid.h\""));
        assert!(OPID_C.contains(".name = \"opid\""));
        assert!(OPID_C.contains(".description = \"operations with preemption and irq disabled.\""));
        assert!(OPID_C.contains("module_init(register_opid);"));
        assert!(OPID_C.contains("module_exit(unregister_opid);"));
        assert!(OPID_C.contains("MODULE_LICENSE(\"GPL\");"));

        assert_eq!(KCONFIG_SYMBOL, "RV_MON_OPID");
        assert_eq!(TRACE_EVENT_PREFIX, "opid");
        assert_eq!(OpidMonitor::NAME, "opid");
        assert_eq!(
            OpidMonitor::DESCRIPTION,
            "operations with preemption and irq disabled."
        );
    }

    #[test]
    fn wrapper_reexports_generated_automaton_and_trace_events() {
        assert!(OPID_H.contains("enum states_opid"));
        assert!(OPID_H.contains("any_opid"));
        assert!(OPID_H.contains("sched_need_resched_opid"));
        assert!(OPID_H.contains("sched_waking_opid"));
        assert!(OPID_H.contains("irq_off_opid"));
        assert!(OPID_H.contains("preempt_off_opid"));
        assert!(OPID_H.contains(".function = {"));

        assert!(OPID_TRACE_H.contains("DEFINE_EVENT(event_da_monitor, event_opid"));
        assert!(OPID_TRACE_H.contains("DEFINE_EVENT(error_da_monitor, error_opid"));
        assert!(OPID_TRACE_H.contains("DEFINE_EVENT(error_env_da_monitor, error_env_opid"));

        let monitor = OpidMonitor::new();
        assert_eq!(
            monitor.transition(OpidState::Any, OpidEvent::SchedNeedResched),
            OpidState::Any
        );
        assert!(ha_verify_guards(
            OpidState::Any,
            OpidEvent::SchedWaking,
            OpidState::Any,
            true,
            2,
            true
        ));
        assert!(ha_verify_constraint(
            OpidState::Any,
            OpidEvent::SchedNeedResched,
            OpidState::Any,
            true,
            0,
            true
        ));
        assert_eq!(ha_get_env(OpidEnv::IrqOff, true, 0, true), 1);
    }
}
