//! linux-parity: complete
//! linux-source: vendor/linux/lib/memory-notifier-error-inject.c
//! test-origin: linux:vendor/linux/lib/memory-notifier-error-inject.c
//! Memory hotplug notifier error-injection action table.

pub const MEM_ONLINE: u64 = 0;
pub const MEM_GOING_OFFLINE: u64 = 1;
pub const MEM_OFFLINE: u64 = 2;
pub const MEM_GOING_ONLINE: u64 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifierErrInjectAction {
    pub name: &'static str,
    pub value: u64,
    pub error: i32,
}

pub const MEMORY_NOTIFIER_ACTIONS: [NotifierErrInjectAction; 2] = [
    NotifierErrInjectAction {
        name: "MEM_GOING_ONLINE",
        value: MEM_GOING_ONLINE,
        error: 0,
    },
    NotifierErrInjectAction {
        name: "MEM_GOING_OFFLINE",
        value: MEM_GOING_OFFLINE,
        error: 0,
    },
];

pub const MODULE_DESCRIPTION: &str = "memory notifier error injection module";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Akinobu Mita <akinobu.mita@gmail.com>";

pub fn memory_notifier_actions() -> &'static [NotifierErrInjectAction] {
    &MEMORY_NOTIFIER_ACTIONS
}

pub fn err_inject_init_result(debugfs_result: Result<(), i32>, register_result: i32) -> i32 {
    if let Err(error) = debugfs_result {
        return error;
    }
    register_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_notifier_error_inject_matches_linux_action_table() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/memory-notifier-error-inject.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/notifier-error-inject.h"
        ));
        assert!(source.contains("module_param(priority, int, 0);"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(MEM_GOING_ONLINE)"));
        assert!(source.contains("NOTIFIER_ERR_INJECT_ACTION(MEM_GOING_OFFLINE)"));
        assert!(source.contains("notifier_err_inject_init(\"memory\""));
        assert!(source.contains("register_memory_notifier(&memory_notifier_err_inject.nb);"));
        assert!(source.contains("unregister_memory_notifier(&memory_notifier_err_inject.nb);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"memory notifier error injection module\")"));
        assert!(header.contains("#define NOTIFIER_ERR_INJECT_ACTION(action)"));

        assert_eq!(
            memory_notifier_actions(),
            &[
                NotifierErrInjectAction {
                    name: "MEM_GOING_ONLINE",
                    value: 3,
                    error: 0,
                },
                NotifierErrInjectAction {
                    name: "MEM_GOING_OFFLINE",
                    value: 1,
                    error: 0,
                },
            ]
        );
        assert_eq!(err_inject_init_result(Ok(()), 0), 0);
        assert_eq!(err_inject_init_result(Ok(()), -16), -16);
        assert_eq!(err_inject_init_result(Err(-12), 0), -12);
        assert_eq!(MODULE_LICENSE, "GPL");
    }
}
