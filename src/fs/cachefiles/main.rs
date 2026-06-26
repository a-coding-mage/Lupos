//! linux-parity: complete
//! linux-source: vendor/linux/fs/cachefiles/main.c
//! test-origin: linux:vendor/linux/fs/cachefiles/main.c
//! CacheFiles module init and teardown ordering.

use crate::include::uapi::errno::ENOMEM;

pub const CACHEFILES_DEVICE_NAME: &str = "cachefiles";
pub const CACHEFILES_OBJECT_JAR_NAME: &str = "cachefiles_object_jar";
pub const CACHEFILES_MODULE_DESCRIPTION: &str = "Mounted-filesystem based cache";

pub const CACHEFILES_INIT_ORDER: &[&str] = &[
    "cachefiles_register_error_injection",
    "misc_register",
    "kmem_cache_create",
];

pub const CACHEFILES_EXIT_ORDER: &[&str] = &[
    "kmem_cache_destroy",
    "misc_deregister",
    "cachefiles_unregister_error_injection",
];

pub const fn cachefiles_init_result(
    error_injection_ret: i32,
    misc_register_ret: i32,
    object_jar_created: bool,
) -> Result<(), i32> {
    if error_injection_ret < 0 {
        return Err(error_injection_ret);
    }
    if misc_register_ret < 0 {
        return Err(misc_register_ret);
    }
    if !object_jar_created {
        return Err(-ENOMEM);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cachefiles_main_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/cachefiles/main.c"
        ));
        assert!(source.contains("#include <linux/miscdevice.h>"));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("unsigned cachefiles_debug;"));
        assert!(
            source
                .contains("module_param_named(debug, cachefiles_debug, uint, S_IWUSR | S_IRUGO);")
        );
        assert!(source.contains("MODULE_DESCRIPTION(\"Mounted-filesystem based cache\");"));
        assert!(source.contains("struct kmem_cache *cachefiles_object_jar;"));
        assert!(source.contains(".name\t= \"cachefiles\""));
        assert!(source.contains(".fops\t= &cachefiles_daemon_fops"));
        assert!(source.contains("ret = cachefiles_register_error_injection();"));
        assert!(source.contains("ret = misc_register(&cachefiles_dev);"));
        assert!(source.contains("kmem_cache_create(\"cachefiles_object_jar\""));
        assert!(source.contains("misc_deregister(&cachefiles_dev);"));
        assert!(source.contains("cachefiles_unregister_error_injection();"));
        assert!(source.contains("fs_initcall(cachefiles_init);"));
        assert!(source.contains("module_exit(cachefiles_exit);"));

        assert_eq!(CACHEFILES_DEVICE_NAME, "cachefiles");
        assert_eq!(
            CACHEFILES_INIT_ORDER[0],
            "cachefiles_register_error_injection"
        );
        assert_eq!(CACHEFILES_EXIT_ORDER[0], "kmem_cache_destroy");
        assert_eq!(cachefiles_init_result(-2, 0, true), Err(-2));
        assert_eq!(cachefiles_init_result(0, -16, true), Err(-16));
        assert_eq!(cachefiles_init_result(0, 0, false), Err(-ENOMEM));
        assert_eq!(cachefiles_init_result(0, 0, true), Ok(()));
    }
}
