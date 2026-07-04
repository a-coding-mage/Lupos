//! linux-parity: partial
//! linux-source: vendor/linux/init
//! Linux `init/` tree.
//!
//! Linux build files are folded into this module set rather than mirrored as
//! standalone Rust files: `Makefile`'s obj list maps to the modules below,
//! the `CONFIG_BLK_DEV_INITRD ? initramfs.o : noinitramfs.o` split is compiled
//! together and selected at runtime, `CONFIG_GENERIC_CALIBRATE_DELAY` maps to
//! `calibrate.rs`, and `.kunitconfig`'s KUNIT/BLK_DEV_INITRD/INITRAMFS_TEST
//! symbols are covered by `initramfs_test.rs`. `.gitignore` has no runtime
//! counterpart.

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
