//! linux-parity: complete
//! linux-source: vendor/linux/init/noinitramfs.c
//! test-origin: linux:vendor/linux/init/noinitramfs.c
//! Default rootfs fallback when no initramfs image is supplied.

use crate::include::uapi::stat::{S_IFCHR, S_IRUSR, S_IWUSR};

pub const DEFAULT_DEV_MODE: u32 = 0o755;
pub const DEFAULT_ROOT_MODE: u32 = 0o700;
pub const DEFAULT_CONSOLE_MODE: u32 = S_IFCHR | S_IRUSR | S_IWUSR;
pub const DEFAULT_CONSOLE_DEV: u32 = new_encode_dev(mkdev(5, 1));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultRootfsOp {
    UsermodehelperEnable,
    Mkdir {
        path: &'static str,
        mode: u32,
    },
    Mknod {
        path: &'static str,
        mode: u32,
        dev: u32,
    },
}

pub const DEFAULT_ROOTFS_OPS: [DefaultRootfsOp; 4] = [
    DefaultRootfsOp::UsermodehelperEnable,
    DefaultRootfsOp::Mkdir {
        path: "/dev",
        mode: DEFAULT_DEV_MODE,
    },
    DefaultRootfsOp::Mknod {
        path: "/dev/console",
        mode: DEFAULT_CONSOLE_MODE,
        dev: DEFAULT_CONSOLE_DEV,
    },
    DefaultRootfsOp::Mkdir {
        path: "/root",
        mode: DEFAULT_ROOT_MODE,
    },
];

pub const fn default_rootfs_plan() -> &'static [DefaultRootfsOp] {
    &DEFAULT_ROOTFS_OPS
}

pub const fn mkdev(major: u32, minor: u32) -> u32 {
    (major << 20) | (minor & ((1 << 20) - 1))
}

pub const fn major(dev: u32) -> u32 {
    dev >> 20
}

pub const fn minor(dev: u32) -> u32 {
    dev & ((1 << 20) - 1)
}

pub const fn new_encode_dev(dev: u32) -> u32 {
    let major = major(dev);
    let minor = minor(dev);
    (minor & 0xff) | (major << 8) | ((minor & !0xff) << 12)
}

pub const fn default_rootfs_result(first_error: i32) -> Result<(), i32> {
    if first_error < 0 {
        Err(first_error)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rootfs_ops_match_linux_noinitramfs_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/noinitramfs.c"
        ));
        assert!(source.contains("usermodehelper_enable();"));
        assert!(source.contains("init_mkdir(\"/dev\", 0755);"));
        assert!(source.contains("init_mknod(\"/dev/console\""));
        assert!(source.contains("S_IFCHR | S_IRUSR | S_IWUSR"));
        assert!(source.contains("new_encode_dev(MKDEV(5, 1))"));
        assert!(source.contains("init_mkdir(\"/root\", 0700);"));
        assert!(source.contains("rootfs_initcall(default_rootfs);"));

        assert_eq!(DEFAULT_CONSOLE_MODE, 0o020600);
        assert_eq!(major(mkdev(5, 1)), 5);
        assert_eq!(minor(mkdev(5, 1)), 1);
        assert_eq!(DEFAULT_CONSOLE_DEV, 0x501);
        assert_eq!(
            default_rootfs_plan()[2],
            DefaultRootfsOp::Mknod {
                path: "/dev/console",
                mode: DEFAULT_CONSOLE_MODE,
                dev: DEFAULT_CONSOLE_DEV,
            }
        );
        assert_eq!(default_rootfs_result(-5), Err(-5));
    }
}
