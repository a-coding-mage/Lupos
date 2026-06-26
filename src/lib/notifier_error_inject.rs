//! linux-parity: complete
//! linux-source: vendor/linux/lib/notifier-error-inject.c
//! test-origin: linux:vendor/linux/lib/notifier-error-inject.c
//! Shared notifier error-injection debugfs helpers.

pub const MAX_ERRNO: i32 = 4095;
pub const DEBUGFS_ROOT: &str = "notifier-error-inject";
pub const ACTIONS_DIR: &str = "actions";
pub const ERROR_FILE: &str = "error";
pub const FOPS_ERRNO_FORMAT: &str = "%lld\n";
pub const S_IFREG: u32 = 0o100000;
pub const S_IRUSR: u32 = 0o400;
pub const S_IWUSR: u32 = 0o200;
pub const DEBUGFS_ERRNO_MODE: u32 = S_IFREG | S_IRUSR | S_IWUSR;
pub const NOTIFY_DONE: i32 = 0x0000;
pub const NOTIFY_OK: i32 = 0x0001;
pub const NOTIFY_STOP_MASK: i32 = 0x8000;
pub const NOTIFY_BAD: i32 = NOTIFY_STOP_MASK | 0x0002;
pub const MODULE_INIT: &str = "err_inject_init";
pub const MODULE_EXIT: &str = "err_inject_exit";
pub const MODULE_DESCRIPTION: &str = "Notifier error injection module";
pub const MODULE_AUTHOR: &str = "Akinobu Mita <akinobu.mita@gmail.com>";
pub const MODULE_LICENSE: &str = "GPL";
pub const EXPORT_NOTIFIER_ERR_INJECT_DIR: &str = "notifier_err_inject_dir";
pub const EXPORT_NOTIFIER_ERR_INJECT_INIT: &str = "notifier_err_inject_init";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifierErrInjectAction {
    pub val: u64,
    pub error: i32,
    pub name: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotifierCall {
    NotifierErrInjectCallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotifierBlockState {
    pub notifier_call: NotifierCall,
    pub priority: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DebugfsErrnoFile {
    pub name: &'static str,
    pub mode: u32,
    pub parent: &'static str,
    pub value: i32,
    pub fops_format: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackReport {
    pub matched_action: Option<&'static str>,
    pub errno: i32,
    pub notify_code: i32,
    pub logged: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitReport {
    pub name: &'static str,
    pub parent_present: bool,
    pub notifier_block: NotifierBlockState,
    pub dir_created: bool,
    pub actions_dir: &'static str,
    pub action_dirs: usize,
    pub error_files: usize,
    pub error_file_mode: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleLifecycleReport {
    pub root_dir: &'static str,
    pub init_return: i32,
    pub exit_removes_root: bool,
}

pub fn clamp_errno(value: i64) -> i32 {
    value.clamp(-(MAX_ERRNO as i64), 0) as i32
}

pub fn clamp_errno_u64(value: u64) -> i32 {
    (value as i32).clamp(-MAX_ERRNO, 0)
}

pub const fn notifier_from_errno(err: i32) -> i32 {
    if err != 0 {
        NOTIFY_STOP_MASK | (NOTIFY_OK - err)
    } else {
        NOTIFY_OK
    }
}

pub const fn notifier_to_errno(ret: i32) -> i32 {
    let ret = ret & !NOTIFY_STOP_MASK;
    if ret > NOTIFY_OK { NOTIFY_OK - ret } else { 0 }
}

pub const fn debugfs_errno_get(value: i32) -> i64 {
    value as i64
}

pub fn debugfs_errno_set(value: &mut i32, val: u64) -> i32 {
    *value = clamp_errno_u64(val);
    0
}

pub const fn debugfs_create_errno(
    name: &'static str,
    mode: u32,
    parent: &'static str,
    value: i32,
) -> DebugfsErrnoFile {
    DebugfsErrnoFile {
        name,
        mode,
        parent,
        value,
        fops_format: FOPS_ERRNO_FORMAT,
    }
}

pub const fn notifier_err_inject_action(name: &'static str, val: u64) -> NotifierErrInjectAction {
    NotifierErrInjectAction {
        val,
        error: 0,
        name: Some(name),
    }
}

pub const fn notifier_err_inject_action_terminator() -> NotifierErrInjectAction {
    NotifierErrInjectAction {
        val: 0,
        error: 0,
        name: None,
    }
}

pub fn notifier_err_inject_action_is_terminator(action: &NotifierErrInjectAction) -> bool {
    action.name.is_none()
}

pub fn notifier_err_inject_errno(
    actions: &[NotifierErrInjectAction],
    val: u64,
) -> Option<(&'static str, i32)> {
    actions
        .iter()
        .take_while(|action| !notifier_err_inject_action_is_terminator(action))
        .find(|action| action.val == val)
        .and_then(|action| action.name.map(|name| (name, action.error)))
}

pub fn notifier_err_inject_callback_report(
    actions: &[NotifierErrInjectAction],
    val: u64,
) -> CallbackReport {
    let matched = notifier_err_inject_errno(actions, val);
    let errno = matched.map(|(_, err)| err).unwrap_or(0);
    CallbackReport {
        matched_action: matched.map(|(name, _)| name),
        errno,
        notify_code: notifier_from_errno(errno),
        logged: errno != 0,
    }
}

pub fn notifier_err_inject_callback(actions: &[NotifierErrInjectAction], val: u64) -> i32 {
    notifier_err_inject_callback_report(actions, val).notify_code
}

pub const fn notifier_err_inject_init_sets_priority(priority: i32) -> i32 {
    priority
}

pub fn notifier_err_inject_init_report(
    name: &'static str,
    parent_present: bool,
    actions: &[NotifierErrInjectAction],
    priority: i32,
) -> InitReport {
    let action_count = actions
        .iter()
        .take_while(|action| !notifier_err_inject_action_is_terminator(action))
        .count();
    InitReport {
        name,
        parent_present,
        notifier_block: NotifierBlockState {
            notifier_call: NotifierCall::NotifierErrInjectCallback,
            priority,
        },
        dir_created: true,
        actions_dir: ACTIONS_DIR,
        action_dirs: action_count,
        error_files: action_count,
        error_file_mode: DEBUGFS_ERRNO_MODE,
    }
}

pub const fn err_inject_init_report() -> ModuleLifecycleReport {
    ModuleLifecycleReport {
        root_dir: DEBUGFS_ROOT,
        init_return: 0,
        exit_removes_root: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifier_error_inject_matches_linux_debugfs_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/notifier-error-inject.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/notifier-error-inject.h"
        ));
        assert!(source.contains("*(int *)data = clamp_t(int, val, -MAX_ERRNO, 0);"));
        assert!(source.contains("*val = *(int *)data;"));
        assert!(source.contains("DEFINE_SIMPLE_ATTRIBUTE_SIGNED(fops_errno"));
        assert!(source.contains("\"%lld\\n\""));
        assert!(source.contains("debugfs_create_file(name, mode, parent, value, &fops_errno);"));
        assert!(source.contains("container_of(nb, struct notifier_err_inject, nb);"));
        assert!(source.contains("for (action = err_inject->actions; action->name; action++)"));
        assert!(source.contains("if (action->val == val)"));
        assert!(source.contains("pr_info(\"Injecting error (%d) to %s\\n\""));
        assert!(source.contains("return notifier_from_errno(err);"));
        assert!(source.contains("umode_t mode = S_IFREG | S_IRUSR | S_IWUSR;"));
        assert!(source.contains("err_inject->nb.notifier_call = notifier_err_inject_callback;"));
        assert!(source.contains("err_inject->nb.priority = priority;"));
        assert!(source.contains("debugfs_create_dir(\"actions\", dir);"));
        assert!(
            source.contains("debugfs_create_errno(\"error\", mode, action_dir, &action->error);")
        );
        assert!(source.contains("struct dentry *notifier_err_inject_dir;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(notifier_err_inject_dir);"));
        assert!(source.contains("debugfs_create_dir(\"notifier-error-inject\", NULL);"));
        assert!(source.contains("debugfs_remove_recursive(notifier_err_inject_dir);"));
        assert!(source.contains("module_init(err_inject_init);"));
        assert!(source.contains("module_exit(err_inject_exit);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(notifier_err_inject_init);"));
        assert!(header.contains("struct notifier_err_inject_action"));
        assert!(header.contains("#define NOTIFIER_ERR_INJECT_ACTION(action)"));
        assert!(header.contains("struct notifier_block nb;"));
        assert!(header.contains("/* The last slot must be terminated with zero sentinel */"));

        let actions = [
            NotifierErrInjectAction {
                val: 1,
                error: -12,
                name: Some("A"),
            },
            NotifierErrInjectAction {
                val: 2,
                error: 0,
                name: Some("B"),
            },
            notifier_err_inject_action_terminator(),
            NotifierErrInjectAction {
                val: 9,
                error: -22,
                name: Some("ignored-after-terminator"),
            },
        ];
        let mut errno = 17;
        assert_eq!(clamp_errno(-5000), -4095);
        assert_eq!(clamp_errno(5), 0);
        assert_eq!(clamp_errno_u64((-5000_i32) as u32 as u64), -4095);
        assert_eq!(clamp_errno_u64(5), 0);
        assert_eq!(debugfs_errno_get(errno), 17);
        assert_eq!(debugfs_errno_set(&mut errno, (-5000_i32) as u32 as u64), 0);
        assert_eq!(errno, -4095);
        assert_eq!(
            debugfs_create_errno(ERROR_FILE, DEBUGFS_ERRNO_MODE, "action-A", -12),
            DebugfsErrnoFile {
                name: ERROR_FILE,
                mode: S_IFREG | S_IRUSR | S_IWUSR,
                parent: "action-A",
                value: -12,
                fops_format: "%lld\n",
            }
        );
        assert_eq!(
            notifier_err_inject_action("MEM_GOING_ONLINE", 1),
            NotifierErrInjectAction {
                val: 1,
                error: 0,
                name: Some("MEM_GOING_ONLINE"),
            }
        );
        assert_eq!(
            notifier_err_inject_action_terminator(),
            NotifierErrInjectAction {
                val: 0,
                error: 0,
                name: None,
            }
        );
        assert!(notifier_err_inject_action_is_terminator(&actions[2]));
        assert_eq!(notifier_from_errno(0), NOTIFY_OK);
        assert_eq!(notifier_from_errno(-12), NOTIFY_STOP_MASK | 13);
        assert_eq!(notifier_to_errno(NOTIFY_STOP_MASK | 13), -12);
        assert_eq!(notifier_err_inject_errno(&actions, 1), Some(("A", -12)));
        assert_eq!(notifier_err_inject_errno(&actions, 9), None);
        assert_eq!(
            notifier_err_inject_callback_report(&actions, 1),
            CallbackReport {
                matched_action: Some("A"),
                errno: -12,
                notify_code: NOTIFY_STOP_MASK | 13,
                logged: true,
            }
        );
        assert_eq!(
            notifier_err_inject_callback(&actions, 1),
            NOTIFY_STOP_MASK | 13
        );
        assert_eq!(notifier_err_inject_callback(&actions, 9), NOTIFY_OK);
        assert_eq!(notifier_err_inject_init_sets_priority(7), 7);
        assert_eq!(
            notifier_err_inject_init_report("memory", true, &actions, 7),
            InitReport {
                name: "memory",
                parent_present: true,
                notifier_block: NotifierBlockState {
                    notifier_call: NotifierCall::NotifierErrInjectCallback,
                    priority: 7,
                },
                dir_created: true,
                actions_dir: ACTIONS_DIR,
                action_dirs: 2,
                error_files: 2,
                error_file_mode: DEBUGFS_ERRNO_MODE,
            }
        );
        assert_eq!(
            err_inject_init_report(),
            ModuleLifecycleReport {
                root_dir: DEBUGFS_ROOT,
                init_return: 0,
                exit_removes_root: true,
            }
        );
        assert_eq!(MODULE_LICENSE, "GPL");
        assert_eq!(MODULE_INIT, "err_inject_init");
        assert_eq!(MODULE_EXIT, "err_inject_exit");
        assert_eq!(EXPORT_NOTIFIER_ERR_INJECT_DIR, "notifier_err_inject_dir");
        assert_eq!(EXPORT_NOTIFIER_ERR_INJECT_INIT, "notifier_err_inject_init");
    }
}
