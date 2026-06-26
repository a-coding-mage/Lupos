//! linux-parity: partial
//! linux-source: vendor/linux/init
//! Linux `init/` tree.

pub mod boot;
pub mod boot_trace;
pub mod calibrate;
pub mod do_mounts;
pub mod do_mounts_initrd;
pub mod do_mounts_rd;
pub mod init_task;
pub mod initcall;
pub mod initramfs;
pub mod initramfs_test;
pub mod noinitramfs;
pub mod rootfs;
pub mod start_kernel;
pub mod version;
pub mod version_timestamp;
