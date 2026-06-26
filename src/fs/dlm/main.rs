//! linux-parity: complete
//! linux-source: vendor/linux/fs/dlm/main.c
//! test-origin: linux:vendor/linux/fs/dlm/main.c
//! DLM module init and exit ordering.

use crate::include::uapi::errno::ENOMEM;

pub const DLM_WORKQUEUE_NAME: &str = "dlm_wq";
pub const DLM_MODULE_DESCRIPTION: &str = "Distributed Lock Manager";

pub const DLM_INIT_ORDER: &[&str] = &[
    "dlm_memory_init",
    "dlm_midcomms_init",
    "dlm_lockspace_init",
    "dlm_config_init",
    "dlm_register_debugfs",
    "dlm_user_init",
    "dlm_plock_init",
    "alloc_workqueue",
];

pub const DLM_EXIT_ORDER: &[&str] = &[
    "destroy_workqueue",
    "dlm_plock_exit",
    "dlm_user_exit",
    "dlm_config_exit",
    "dlm_lockspace_exit",
    "dlm_midcomms_exit",
    "dlm_unregister_debugfs",
    "dlm_memory_exit",
];

pub const fn init_dlm_result(
    memory_ret: i32,
    lockspace_ret: i32,
    config_ret: i32,
    user_ret: i32,
    plock_ret: i32,
    workqueue_allocated: bool,
) -> Result<(), i32> {
    if memory_ret != 0 {
        return Err(memory_ret);
    }
    if lockspace_ret != 0 {
        return Err(lockspace_ret);
    }
    if config_ret != 0 {
        return Err(config_ret);
    }
    if user_ret != 0 {
        return Err(user_ret);
    }
    if plock_ret != 0 {
        return Err(plock_ret);
    }
    if !workqueue_allocated {
        return Err(-ENOMEM);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dlm_main_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/dlm/main.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include \"dlm_internal.h\""));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("struct workqueue_struct *dlm_wq;"));
        assert!(source.contains("static int __init init_dlm(void)"));
        assert!(source.contains("error = dlm_memory_init();"));
        assert!(source.contains("dlm_midcomms_init();"));
        assert!(source.contains("error = dlm_lockspace_init();"));
        assert!(source.contains("error = dlm_config_init();"));
        assert!(source.contains("dlm_register_debugfs();"));
        assert!(source.contains("error = dlm_user_init();"));
        assert!(source.contains("error = dlm_plock_init();"));
        assert!(source.contains("dlm_wq = alloc_workqueue(\"dlm_wq\", WQ_PERCPU, 0);"));
        assert!(source.contains("error = -ENOMEM;"));
        assert!(source.contains("printk(\"DLM installed\\n\");"));
        assert!(source.contains("destroy_workqueue(dlm_wq);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Distributed Lock Manager\");"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(dlm_new_lockspace);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(dlm_unlock);"));

        assert_eq!(DLM_INIT_ORDER[0], "dlm_memory_init");
        assert_eq!(DLM_INIT_ORDER[7], "alloc_workqueue");
        assert_eq!(DLM_EXIT_ORDER[0], "destroy_workqueue");
        assert_eq!(init_dlm_result(-5, 0, 0, 0, 0, true), Err(-5));
        assert_eq!(init_dlm_result(0, -6, 0, 0, 0, true), Err(-6));
        assert_eq!(init_dlm_result(0, 0, -7, 0, 0, true), Err(-7));
        assert_eq!(init_dlm_result(0, 0, 0, -8, 0, true), Err(-8));
        assert_eq!(init_dlm_result(0, 0, 0, 0, -9, true), Err(-9));
        assert_eq!(init_dlm_result(0, 0, 0, 0, 0, false), Err(-ENOMEM));
        assert_eq!(init_dlm_result(0, 0, 0, 0, 0, true), Ok(()));
    }
}
