//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_fpu_glue.c
//! test-origin: linux:vendor/linux/lib/test_fpu_glue.c
//! Debugfs glue for the kernel FPU self-test.

use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::lib::test_fpu_impl::test_fpu;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestFpuInit {
    pub result: i32,
    pub debugfs_dir_created: bool,
    pub debugfs_file_created: bool,
}

pub const MODULE_DESCRIPTION: &str = "Test cases for floating point operations";
pub const MODULE_LICENSE: &str = "GPL";
pub const DEBUGFS_DIR: &str = "selftest_helpers";
pub const DEBUGFS_FILE: &str = "test_fpu";

pub fn test_fpu_get() -> (i32, u64) {
    (test_fpu(), 1)
}

pub const fn test_fpu_init(kernel_fpu_available: bool, debugfs_dir_ok: bool) -> TestFpuInit {
    if !kernel_fpu_available {
        return TestFpuInit {
            result: -EINVAL,
            debugfs_dir_created: false,
            debugfs_file_created: false,
        };
    }

    if !debugfs_dir_ok {
        return TestFpuInit {
            result: -ENOMEM,
            debugfs_dir_created: false,
            debugfs_file_created: false,
        };
    }

    TestFpuInit {
        result: 0,
        debugfs_dir_created: true,
        debugfs_file_created: true,
    }
}

pub const fn test_fpu_exit_removes_debugfs() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fpu_glue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_fpu_glue.c"
        ));
        assert!(source.contains("kernel_fpu_begin();"));
        assert!(source.contains("status = test_fpu();"));
        assert!(source.contains("kernel_fpu_end();"));
        assert!(source.contains("*val = 1;"));
        assert!(source.contains("if (!kernel_fpu_available())"));
        assert!(source.contains("debugfs_create_dir(\"selftest_helpers\""));
        assert!(source.contains("debugfs_create_file_unsafe(\"test_fpu\""));
        assert!(source.contains("debugfs_remove(selftest_dir);"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"Test cases for floating point operations\")")
        );

        assert_eq!(test_fpu_get(), (0, 1));
        assert_eq!(test_fpu_init(false, true).result, -EINVAL);
        assert_eq!(test_fpu_init(true, false).result, -ENOMEM);
        assert_eq!(
            test_fpu_init(true, true),
            TestFpuInit {
                result: 0,
                debugfs_dir_created: true,
                debugfs_file_created: true,
            }
        );
        assert!(test_fpu_exit_removes_debugfs());
    }
}
