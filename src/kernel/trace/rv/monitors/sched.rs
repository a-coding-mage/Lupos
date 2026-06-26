//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/sched/sched.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/sched/sched.c
//! RV monitor container for scheduler specifications.

pub const MONITOR_NAME: &str = "sched";
pub const MONITOR_DESCRIPTION: &str = "container for several scheduler monitor specifications.";
pub const MODULE_AUTHOR: &str = "Gabriele Monaco <gmonaco@redhat.com>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedMonitor {
    pub name: &'static str,
    pub description: &'static str,
    pub enabled: bool,
}

pub const RV_SCHED: SchedMonitor = SchedMonitor {
    name: MONITOR_NAME,
    description: MONITOR_DESCRIPTION,
    enabled: false,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sched_monitor_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/sched/sched.c"
        ));
        assert!(source.contains("#define MODULE_NAME \"sched\""));
        assert!(source.contains(".name = \"sched\""));
        assert!(source.contains(MONITOR_DESCRIPTION));
        assert!(source.contains(".enable = NULL"));
        assert!(source.contains(".disable = NULL"));
        assert!(source.contains(".reset = NULL"));
        assert!(source.contains(".enabled = 0"));
        assert!(source.contains("rv_register_monitor(&rv_sched, NULL);"));
        assert!(source.contains("rv_unregister_monitor(&rv_sched);"));
        assert!(source.contains("module_init(register_sched);"));
        assert!(source.contains("module_exit(unregister_sched);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert!(source.contains(MODULE_AUTHOR));
        assert_eq!(RV_SCHED.name, "sched");
        assert!(!RV_SCHED.enabled);
    }
}
