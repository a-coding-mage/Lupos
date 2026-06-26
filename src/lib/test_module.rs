//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_module.c
//! test-origin: linux:vendor/linux/lib/test_module.c
//! Minimal module-loading test module.

pub const PR_FMT_DEFINE: &str = "#define pr_fmt(fmt) KBUILD_MODNAME \": \" fmt";
pub const LINUX_INIT_INCLUDE: &str = "#include <linux/init.h>";
pub const LINUX_MODULE_INCLUDE: &str = "#include <linux/module.h>";
pub const LINUX_PRINTK_INCLUDE: &str = "#include <linux/printk.h>";
pub const INIT_FUNCTION: &str = "test_module_init";
pub const EXIT_FUNCTION: &str = "test_module_exit";
pub const INIT_ATTRIBUTE: &str = "__init";
pub const EXIT_ATTRIBUTE: &str = "__exit";
pub const MODULE_AUTHOR: &str = "Kees Cook <keescook@chromium.org>";
pub const MODULE_DESCRIPTION: &str = "module loading subsystem test module";
pub const MODULE_LICENSE: &str = "GPL";
pub const INIT_MESSAGE: &str = "Hello, world\n";
pub const EXIT_MESSAGE: &str = "Goodbye\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxTestModuleHook {
    pub hook_macro: &'static str,
    pub function: &'static str,
    pub attribute: &'static str,
    pub printk_call: &'static str,
}

pub const TEST_MODULE_INIT_HOOK: LinuxTestModuleHook = LinuxTestModuleHook {
    hook_macro: "module_init",
    function: INIT_FUNCTION,
    attribute: INIT_ATTRIBUTE,
    printk_call: "pr_warn(\"Hello, world\\n\");",
};

pub const TEST_MODULE_EXIT_HOOK: LinuxTestModuleHook = LinuxTestModuleHook {
    hook_macro: "module_exit",
    function: EXIT_FUNCTION,
    attribute: EXIT_ATTRIBUTE,
    printk_call: "pr_warn(\"Goodbye\\n\");",
};

pub fn test_module_init() -> i32 {
    0
}

pub fn test_module_init_message() -> &'static str {
    INIT_MESSAGE
}

pub fn test_module_exit_message() -> &'static str {
    EXIT_MESSAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_metadata_and_messages_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_module.c"
        ));
        assert!(source.contains(PR_FMT_DEFINE));
        assert!(source.contains(LINUX_INIT_INCLUDE));
        assert!(source.contains(LINUX_MODULE_INCLUDE));
        assert!(source.contains(LINUX_PRINTK_INCLUDE));
        assert!(source.contains("static int __init test_module_init(void)"));
        assert!(source.contains("static void __exit test_module_exit(void)"));
        assert!(source.contains(TEST_MODULE_INIT_HOOK.printk_call));
        assert!(source.contains(TEST_MODULE_EXIT_HOOK.printk_call));
        assert!(source.contains("module_init(test_module_init);"));
        assert!(source.contains("module_exit(test_module_exit);"));
        assert_eq!(
            TEST_MODULE_INIT_HOOK,
            LinuxTestModuleHook {
                hook_macro: "module_init",
                function: "test_module_init",
                attribute: "__init",
                printk_call: "pr_warn(\"Hello, world\\n\");",
            }
        );
        assert_eq!(
            TEST_MODULE_EXIT_HOOK,
            LinuxTestModuleHook {
                hook_macro: "module_exit",
                function: "test_module_exit",
                attribute: "__exit",
                printk_call: "pr_warn(\"Goodbye\\n\");",
            }
        );
        assert!(source.contains(MODULE_AUTHOR));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert_eq!(test_module_init(), 0);
        assert_eq!(test_module_init_message(), INIT_MESSAGE);
        assert_eq!(test_module_exit_message(), EXIT_MESSAGE);
    }
}
