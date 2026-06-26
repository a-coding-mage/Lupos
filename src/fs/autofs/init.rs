//! linux-parity: complete
//! linux-source: vendor/linux/fs/autofs/init.c
//! test-origin: linux:vendor/linux/fs/autofs/init.c
//! autofs filesystem type registration metadata.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutofsFileSystemType {
    pub name: &'static str,
    pub init_fs_context: &'static str,
    pub parameters: &'static str,
    pub kill_sb: &'static str,
}

pub const AUTOFS_FS_TYPE: AutofsFileSystemType = AutofsFileSystemType {
    name: "autofs",
    init_fs_context: "autofs_init_fs_context",
    parameters: "autofs_param_specs",
    kill_sb: "autofs_kill_sb",
};

pub const MODULE_ALIASES: &[&str] = &["autofs", "fs:autofs"];
pub const MODULE_DESCRIPTION: &str = "Kernel automounter support";
pub const MODULE_LICENSE: &str = "GPL";

pub const fn init_autofs_fs_result(register_filesystem_result: i32) -> Result<(), i32> {
    if register_filesystem_result == 0 {
        Ok(())
    } else {
        Err(register_filesystem_result)
    }
}

pub const fn init_autofs_fs_unwinds_dev_ioctl(register_filesystem_result: i32) -> bool {
    register_filesystem_result != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autofs_init_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/autofs/init.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/init.h>"));
        assert!(source.contains("#include \"autofs_i.h\""));
        assert!(source.contains("struct file_system_type autofs_fs_type"));
        assert!(source.contains(".name\t\t= \"autofs\""));
        assert!(source.contains(".init_fs_context = autofs_init_fs_context"));
        assert!(source.contains(".parameters\t= autofs_param_specs"));
        assert!(source.contains(".kill_sb\t= autofs_kill_sb"));
        assert!(source.contains("MODULE_ALIAS_FS(\"autofs\")"));
        assert!(source.contains("MODULE_ALIAS(\"autofs\")"));
        assert!(source.contains("autofs_dev_ioctl_init();"));
        assert!(source.contains("err = register_filesystem(&autofs_fs_type);"));
        assert!(source.contains("if (err)"));
        assert!(source.contains("autofs_dev_ioctl_exit();"));
        assert!(source.contains("unregister_filesystem(&autofs_fs_type);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Kernel automounter support\")"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));

        assert_eq!(AUTOFS_FS_TYPE.name, "autofs");
        assert_eq!(AUTOFS_FS_TYPE.init_fs_context, "autofs_init_fs_context");
        assert_eq!(init_autofs_fs_result(0), Ok(()));
        assert_eq!(init_autofs_fs_result(-12), Err(-12));
        assert!(!init_autofs_fs_unwinds_dev_ioctl(0));
        assert!(init_autofs_fs_unwinds_dev_ioctl(-12));
    }
}
