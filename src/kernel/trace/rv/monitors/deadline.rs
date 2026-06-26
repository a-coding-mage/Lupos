//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/deadline/deadline.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/deadline/deadline.c
//! RV monitor container for deadline scheduler specifications.

pub const MONITOR_NAME: &str = "deadline";
pub const MONITOR_DESCRIPTION: &str = "container for several deadline scheduler specifications.";
pub const MODULE_AUTHOR: &str = "Gabriele Monaco <gmonaco@redhat.com>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeadlineMonitor {
    pub name: &'static str,
    pub description: &'static str,
    pub enabled: bool,
}

pub const RV_DEADLINE: DeadlineMonitor = DeadlineMonitor {
    name: MONITOR_NAME,
    description: MONITOR_DESCRIPTION,
    enabled: false,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeadlineRegistration {
    pub monitor: DeadlineMonitor,
    pub ext_sched_class: Option<usize>,
    pub warned_missing_ext_sched_class: bool,
}

pub const fn register_deadline(
    config_sched_class_ext: bool,
    ext_sched_class: Option<usize>,
) -> DeadlineRegistration {
    DeadlineRegistration {
        monitor: RV_DEADLINE,
        ext_sched_class: if config_sched_class_ext {
            ext_sched_class
        } else {
            None
        },
        warned_missing_ext_sched_class: config_sched_class_ext && ext_sched_class.is_none(),
    }
}

pub fn unregister_deadline(monitor: &mut DeadlineMonitor) {
    monitor.enabled = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadline_monitor_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/deadline/deadline.c"
        ));
        assert!(source.contains("#define MODULE_NAME \"deadline\""));
        assert!(source.contains(".name = \"deadline\""));
        assert!(source.contains(MONITOR_DESCRIPTION));
        assert!(source.contains(".enable = NULL"));
        assert!(source.contains(".disable = NULL"));
        assert!(source.contains(".reset = NULL"));
        assert!(source.contains(".enabled = 0"));
        assert!(source.contains("rv_ext_sched_class"));
        assert!(source.contains("kallsyms_lookup_name(\"ext_sched_class\")"));
        assert!(source.contains("rv_register_monitor(&rv_deadline, NULL);"));
        assert!(source.contains("rv_unregister_monitor(&rv_deadline);"));
        assert!(source.contains("module_init(register_deadline);"));
        assert!(source.contains("module_exit(unregister_deadline);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert!(source.contains(MODULE_AUTHOR));

        let no_ext = register_deadline(false, Some(0x1000));
        assert_eq!(no_ext.monitor, RV_DEADLINE);
        assert_eq!(no_ext.ext_sched_class, None);
        assert!(!no_ext.warned_missing_ext_sched_class);

        let missing_ext = register_deadline(true, None);
        assert!(missing_ext.warned_missing_ext_sched_class);

        let with_ext = register_deadline(true, Some(0x2000));
        assert_eq!(with_ext.ext_sched_class, Some(0x2000));
        assert!(!with_ext.warned_missing_ext_sched_class);
    }
}
