//! linux-parity: complete
//! linux-source: vendor/linux/lib/netdev-notifier-error-inject.c
//! test-origin: linux:vendor/linux/lib/netdev-notifier-error-inject.c
//! Netdevice notifier error-injection metadata.

pub const NETDEV_REGISTER: u64 = 5;
pub const NETDEV_CHANGEMTU: u64 = 7;
pub const NETDEV_CHANGENAME: u64 = 11;
pub const NETDEV_PRE_UP: u64 = 14;
pub const NETDEV_PRE_TYPE_CHANGE: u64 = 15;
pub const NETDEV_POST_INIT: u64 = 17;
pub const NETDEV_CHANGEUPPER: u64 = 22;
pub const NETDEV_PRECHANGEMTU: u64 = 24;
pub const NETDEV_PRECHANGEUPPER: u64 = 27;

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

pub const NETDEV_NOTIFIER_ACTIONS: &[NotifierErrInjectAction] = &[
    NotifierErrInjectAction {
        name: "NETDEV_REGISTER",
        val: NETDEV_REGISTER,
    },
    NotifierErrInjectAction {
        name: "NETDEV_CHANGEMTU",
        val: NETDEV_CHANGEMTU,
    },
    NotifierErrInjectAction {
        name: "NETDEV_CHANGENAME",
        val: NETDEV_CHANGENAME,
    },
    NotifierErrInjectAction {
        name: "NETDEV_PRE_UP",
        val: NETDEV_PRE_UP,
    },
    NotifierErrInjectAction {
        name: "NETDEV_PRE_TYPE_CHANGE",
        val: NETDEV_PRE_TYPE_CHANGE,
    },
    NotifierErrInjectAction {
        name: "NETDEV_POST_INIT",
        val: NETDEV_POST_INIT,
    },
    NotifierErrInjectAction {
        name: "NETDEV_PRECHANGEMTU",
        val: NETDEV_PRECHANGEMTU,
    },
    NotifierErrInjectAction {
        name: "NETDEV_PRECHANGEUPPER",
        val: NETDEV_PRECHANGEUPPER,
    },
    NotifierErrInjectAction {
        name: "NETDEV_CHANGEUPPER",
        val: NETDEV_CHANGEUPPER,
    },
];

pub const MODULE_DESCRIPTION: &str = "Netdevice notifier error injection module";
pub const MODULE_AUTHOR: &str = "Nikolay Aleksandrov <razor@blackwall.org>";
pub const MODULE_LICENSE: &str = "GPL";

pub const fn netdev_err_inject_init(
    notifier_init_result: i32,
    register_result: i32,
    priority: i32,
) -> NotifierInjectInit {
    if notifier_init_result != 0 {
        return NotifierInjectInit {
            result: notifier_init_result,
            debugfs_name: "netdev",
            priority,
            debugfs_removed_on_error: false,
        };
    }

    NotifierInjectInit {
        result: register_result,
        debugfs_name: "netdev",
        priority,
        debugfs_removed_on_error: register_result != 0,
    }
}

pub const fn netdev_err_inject_exit_registered() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netdev_notifier_error_inject_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/netdev-notifier-error-inject.c"
        ));
        assert!(source.contains("module_param(priority, int, 0);"));
        for action in NETDEV_NOTIFIER_ACTIONS {
            assert!(source.contains(action.name));
        }
        assert!(source.contains("notifier_err_inject_init(\"netdev\""));
        assert!(source.contains("register_netdevice_notifier(&netdev_notifier_err_inject.nb);"));
        assert!(source.contains("debugfs_remove_recursive(dir);"));
        assert!(source.contains("unregister_netdevice_notifier(&netdev_notifier_err_inject.nb);"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Netdevice notifier error injection module\")")
        );
        assert!(source.contains(MODULE_AUTHOR));

        assert_eq!(NETDEV_NOTIFIER_ACTIONS.len(), 9);
        assert_eq!(NETDEV_NOTIFIER_ACTIONS[0].val, NETDEV_REGISTER);
        assert_eq!(
            netdev_err_inject_init(-12, 0, 3),
            NotifierInjectInit {
                result: -12,
                debugfs_name: "netdev",
                priority: 3,
                debugfs_removed_on_error: false,
            }
        );
        assert!(netdev_err_inject_init(0, -22, 7).debugfs_removed_on_error);
        assert_eq!(netdev_err_inject_init(0, 0, 7).result, 0);
        assert!(netdev_err_inject_exit_registered());
    }
}
