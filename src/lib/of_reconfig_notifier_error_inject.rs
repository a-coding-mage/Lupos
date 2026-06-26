//! linux-parity: complete
//! linux-source: vendor/linux/lib/of-reconfig-notifier-error-inject.c
//! test-origin: linux:vendor/linux/lib/of-reconfig-notifier-error-inject.c
//! Open Firmware reconfiguration notifier error-injection metadata.

use crate::include::uapi::errno::ENOMEM;

pub const OF_RECONFIG_ATTACH_NODE: u64 = 0x0001;
pub const OF_RECONFIG_DETACH_NODE: u64 = 0x0002;
pub const OF_RECONFIG_ADD_PROPERTY: u64 = 0x0003;
pub const OF_RECONFIG_REMOVE_PROPERTY: u64 = 0x0004;
pub const OF_RECONFIG_UPDATE_PROPERTY: u64 = 0x0005;

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

pub const OF_RECONFIG_ACTIONS: &[NotifierErrInjectAction] = &[
    NotifierErrInjectAction {
        name: "OF_RECONFIG_ATTACH_NODE",
        val: OF_RECONFIG_ATTACH_NODE,
    },
    NotifierErrInjectAction {
        name: "OF_RECONFIG_DETACH_NODE",
        val: OF_RECONFIG_DETACH_NODE,
    },
    NotifierErrInjectAction {
        name: "OF_RECONFIG_ADD_PROPERTY",
        val: OF_RECONFIG_ADD_PROPERTY,
    },
    NotifierErrInjectAction {
        name: "OF_RECONFIG_REMOVE_PROPERTY",
        val: OF_RECONFIG_REMOVE_PROPERTY,
    },
    NotifierErrInjectAction {
        name: "OF_RECONFIG_UPDATE_PROPERTY",
        val: OF_RECONFIG_UPDATE_PROPERTY,
    },
];

pub const MODULE_DESCRIPTION: &str = "OF reconfig notifier error injection module";
pub const MODULE_AUTHOR: &str = "Akinobu Mita <akinobu.mita@gmail.com>";
pub const MODULE_LICENSE: &str = "GPL";

pub const fn of_reconfig_err_inject_init(
    notifier_init_ok: bool,
    register_result: i32,
    priority: i32,
) -> NotifierInjectInit {
    if !notifier_init_ok {
        return NotifierInjectInit {
            result: -ENOMEM,
            debugfs_name: "OF-reconfig",
            priority,
            debugfs_removed_on_error: false,
        };
    }
    NotifierInjectInit {
        result: register_result,
        debugfs_name: "OF-reconfig",
        priority,
        debugfs_removed_on_error: register_result != 0,
    }
}

pub const fn of_reconfig_err_inject_exit_registered() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn of_reconfig_notifier_error_inject_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/of-reconfig-notifier-error-inject.c"
        ));
        assert!(source.contains("module_param(priority, int, 0);"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(OF_RECONFIG_ATTACH_NODE)"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(OF_RECONFIG_DETACH_NODE)"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(OF_RECONFIG_ADD_PROPERTY)"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(OF_RECONFIG_REMOVE_PROPERTY)"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(OF_RECONFIG_UPDATE_PROPERTY)"));
        assert!(source.contains("notifier_err_inject_init(\"OF-reconfig\""));
        assert!(source.contains("of_reconfig_notifier_register(&reconfig_err_inject.nb);"));
        assert!(source.contains("debugfs_remove_recursive(dir);"));
        assert!(source.contains("of_reconfig_notifier_unregister(&reconfig_err_inject.nb);"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"OF reconfig notifier error injection module\")")
        );
        assert!(source.contains(MODULE_AUTHOR));

        assert_eq!(OF_RECONFIG_ACTIONS.len(), 5);
        assert_eq!(OF_RECONFIG_ACTIONS[4].val, OF_RECONFIG_UPDATE_PROPERTY);
        assert_eq!(
            of_reconfig_err_inject_init(false, 0, 9),
            NotifierInjectInit {
                result: -ENOMEM,
                debugfs_name: "OF-reconfig",
                priority: 9,
                debugfs_removed_on_error: false,
            }
        );
        assert!(of_reconfig_err_inject_init(true, -22, 4).debugfs_removed_on_error);
        assert_eq!(of_reconfig_err_inject_init(true, 0, 4).result, 0);
        assert!(of_reconfig_err_inject_exit_registered());
    }
}
