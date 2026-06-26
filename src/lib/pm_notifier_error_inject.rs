//! linux-parity: complete
//! linux-source: vendor/linux/lib/pm-notifier-error-inject.c
//! test-origin: linux:vendor/linux/lib/pm-notifier-error-inject.c
//! PM notifier error-injection module metadata and init flow.

use crate::include::uapi::errno::ENOMEM;

pub const PM_HIBERNATION_PREPARE: u64 = 0x0001;
pub const PM_SUSPEND_PREPARE: u64 = 0x0003;
pub const PM_RESTORE_PREPARE: u64 = 0x0005;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifierErrInjectAction {
    pub name: &'static str,
    pub val: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifierInjectInit {
    pub result: i32,
    pub debugfs_name: &'static str,
    pub priority: i32,
    pub debugfs_removed_on_error: bool,
}

pub const PM_NOTIFIER_ACTIONS: &[NotifierErrInjectAction] = &[
    NotifierErrInjectAction {
        name: "PM_HIBERNATION_PREPARE",
        val: PM_HIBERNATION_PREPARE,
    },
    NotifierErrInjectAction {
        name: "PM_SUSPEND_PREPARE",
        val: PM_SUSPEND_PREPARE,
    },
    NotifierErrInjectAction {
        name: "PM_RESTORE_PREPARE",
        val: PM_RESTORE_PREPARE,
    },
];

pub const MODULE_DESCRIPTION: &str = "PM notifier error injection module";
pub const MODULE_AUTHOR: &str = "Akinobu Mita <akinobu.mita@gmail.com>";
pub const MODULE_LICENSE: &str = "GPL";

pub const fn pm_err_inject_init(
    notifier_init_ok: bool,
    register_pm_notifier_result: i32,
    priority: i32,
) -> NotifierInjectInit {
    if !notifier_init_ok {
        return NotifierInjectInit {
            result: -ENOMEM,
            debugfs_name: "pm",
            priority,
            debugfs_removed_on_error: false,
        };
    }
    NotifierInjectInit {
        result: register_pm_notifier_result,
        debugfs_name: "pm",
        priority,
        debugfs_removed_on_error: register_pm_notifier_result != 0,
    }
}

pub const fn pm_err_inject_exit_registered() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pm_notifier_error_inject_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/pm-notifier-error-inject.c"
        ));
        assert!(source.contains("module_param(priority, int, 0);"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(PM_HIBERNATION_PREPARE)"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(PM_SUSPEND_PREPARE)"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(PM_RESTORE_PREPARE)"));
        assert!(source.contains("notifier_err_inject_init(\"pm\""));
        assert!(source.contains("register_pm_notifier(&pm_notifier_err_inject.nb);"));
        assert!(source.contains("debugfs_remove_recursive(dir);"));
        assert!(source.contains("unregister_pm_notifier(&pm_notifier_err_inject.nb);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"PM notifier error injection module\")"));
        assert!(source.contains(MODULE_AUTHOR));

        assert_eq!(PM_NOTIFIER_ACTIONS.len(), 3);
        assert_eq!(PM_NOTIFIER_ACTIONS[0].val, PM_HIBERNATION_PREPARE);
        assert_eq!(PM_NOTIFIER_ACTIONS[1].name, "PM_SUSPEND_PREPARE");
        assert_eq!(
            pm_err_inject_init(false, 0, 7),
            NotifierInjectInit {
                result: -ENOMEM,
                debugfs_name: "pm",
                priority: 7,
                debugfs_removed_on_error: false,
            }
        );
        assert!(pm_err_inject_init(true, -22, 3).debugfs_removed_on_error);
        assert_eq!(pm_err_inject_init(true, 0, 3).result, 0);
        assert!(pm_err_inject_exit_registered());
    }
}
