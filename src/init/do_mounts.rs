//! linux-parity: partial
//! linux-source: vendor/linux/init/do_mounts.c
//! test-origin: linux:vendor/linux/init/do_mounts.c
//! Initial root mount command-line state and root-device selection.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;
use crate::include::uapi::mount::{MS_RDONLY, MS_SILENT};

pub const SAVED_ROOT_NAME_LEN: usize = 64;
pub const MSEC_PER_SEC: i32 = 1000;
pub const ROOT_NFS: u32 = 255;
pub const ROOT_CIFS: u32 = 254;
pub const ROOT_GENERIC: u32 = 253;
pub const ROOT_RAM0: u32 = 1 << 20;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoMountsState {
    pub root_mountflags: u64,
    pub saved_root_name: String,
    pub root_wait: i32,
    pub root_delay: u32,
    pub root_mount_data: Option<String>,
    pub root_fs_names: Option<String>,
}

impl DoMountsState {
    pub fn new() -> Self {
        Self {
            root_mountflags: MS_RDONLY | MS_SILENT,
            saved_root_name: String::new(),
            root_wait: 0,
            root_delay: 0,
            root_mount_data: None,
            root_fs_names: None,
        }
    }

    pub fn readonly(&mut self, arg: &str) -> bool {
        if !arg.is_empty() {
            return false;
        }
        self.root_mountflags |= MS_RDONLY;
        true
    }

    pub fn readwrite(&mut self, arg: &str) -> bool {
        if !arg.is_empty() {
            return false;
        }
        self.root_mountflags &= !MS_RDONLY;
        true
    }

    pub fn root_dev_setup(&mut self, line: &str) -> bool {
        self.saved_root_name = strscpy_64(line);
        true
    }

    pub fn rootwait_setup(&mut self, arg: &str) -> bool {
        if !arg.is_empty() {
            return false;
        }
        self.root_wait = -1;
        true
    }

    pub fn rootwait_timeout_setup(&mut self, arg: &str) -> bool {
        let Some(sec) = parse_i32(arg) else {
            self.root_wait = -1;
            return true;
        };
        if sec < 0 {
            self.root_wait = -1;
            return true;
        }
        let Some(ms) = sec.checked_mul(MSEC_PER_SEC) else {
            self.root_wait = -1;
            return true;
        };
        self.root_wait = ms;
        true
    }

    pub fn root_data_setup(&mut self, arg: &str) -> bool {
        self.root_mount_data = Some(String::from(arg));
        true
    }

    pub fn fs_names_setup(&mut self, arg: &str) -> bool {
        self.root_fs_names = Some(String::from(arg));
        true
    }

    pub fn root_delay_setup(&mut self, arg: &str) -> bool {
        let Some(delay) = parse_u32(arg) else {
            return false;
        };
        self.root_delay = delay;
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootParseResult {
    pub root_dev: u32,
    pub root_wait: i32,
}

pub fn split_fs_names(root_fs_names: &str) -> Vec<&str> {
    root_fs_names.split(',').collect()
}

pub fn parse_root_device(
    root_device_name: &str,
    root_wait: i32,
    early_lookup: Result<u32, i32>,
) -> RootParseResult {
    if root_device_name.starts_with("mtd") || root_device_name.starts_with("ubi") {
        return RootParseResult {
            root_dev: ROOT_GENERIC,
            root_wait,
        };
    }
    if root_device_name == "/dev/nfs" {
        return RootParseResult {
            root_dev: ROOT_NFS,
            root_wait,
        };
    }
    if root_device_name == "/dev/cifs" {
        return RootParseResult {
            root_dev: ROOT_CIFS,
            root_wait,
        };
    }
    if root_device_name == "/dev/ram" {
        return RootParseResult {
            root_dev: ROOT_RAM0,
            root_wait,
        };
    }

    match early_lookup {
        Ok(dev) => RootParseResult {
            root_dev: dev,
            root_wait,
        },
        Err(error) if error == -EINVAL && root_wait != 0 => RootParseResult {
            root_dev: 0,
            root_wait: 0,
        },
        Err(_) => RootParseResult {
            root_dev: 0,
            root_wait,
        },
    }
}

pub fn rootfs_should_use_tmpfs(
    tmpfs_enabled: bool,
    saved_root_name: &str,
    root_fs_names: Option<&str>,
) -> bool {
    if !tmpfs_enabled {
        return false;
    }
    if saved_root_name.is_empty() && root_fs_names.is_none() {
        return true;
    }
    root_fs_names.is_some_and(|names| names.contains("tmpfs"))
}

pub fn mount_root_kind(
    root_dev: u32,
    root_device_name: Option<&str>,
    root_fs_names: Option<&str>,
) -> RootMountKind {
    match root_dev {
        ROOT_NFS => RootMountKind::Nfs,
        ROOT_CIFS => RootMountKind::Cifs,
        ROOT_GENERIC => RootMountKind::Generic,
        0 if root_device_name.is_some() && root_fs_names.is_some() => RootMountKind::NodevThenBlock,
        _ => RootMountKind::Block,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootMountKind {
    Nfs,
    Cifs,
    Generic,
    NodevThenBlock,
    Block,
}

fn strscpy_64(line: &str) -> String {
    let max = SAVED_ROOT_NAME_LEN.saturating_sub(1);
    let mut end = line.len().min(max);
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    String::from(&line[..end])
}

fn parse_i32(arg: &str) -> Option<i32> {
    let (radix, digits) = if let Some(hex) = arg.strip_prefix("0x") {
        (16, hex)
    } else if let Some(hex) = arg.strip_prefix("0X") {
        (16, hex)
    } else {
        (10, arg)
    };
    i32::from_str_radix(digits, radix).ok()
}

fn parse_u32(arg: &str) -> Option<u32> {
    let (radix, digits) = if let Some(hex) = arg.strip_prefix("0x") {
        (16, hex)
    } else if let Some(hex) = arg.strip_prefix("0X") {
        (16, hex)
    } else {
        (10, arg)
    };
    u32::from_str_radix(digits, radix).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_mounts_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/do_mounts.c"
        ));
        assert!(source.contains("int root_mountflags = MS_RDONLY | MS_SILENT;"));
        assert!(source.contains("static char __initdata saved_root_name[64];"));
        assert!(source.contains("__setup(\"ro\", readonly);"));
        assert!(source.contains("__setup(\"rw\", readwrite);"));
        assert!(source.contains("__setup(\"root=\", root_dev_setup);"));
        assert!(source.contains("__setup(\"rootwait\", rootwait_setup);"));
        assert!(source.contains("__setup(\"rootwait=\", rootwait_timeout_setup);"));
        assert!(source.contains("static int __init split_fs_names(char *page, size_t size)"));
        assert!(source.contains("void __init mount_root(char *root_device_name)"));
        assert!(source.contains("static dev_t __init parse_root_device(char *root_device_name)"));
        assert!(source.contains("void __init prepare_namespace(void)"));
        assert!(source.contains("struct file_system_type rootfs_fs_type"));
        assert!(source.contains("void __init init_rootfs(void)"));
    }

    #[test]
    fn root_flags_and_setup_handlers_match_linux_behavior() {
        let mut state = DoMountsState::new();
        assert!(state.readwrite(""));
        assert_eq!(state.root_mountflags & MS_RDONLY, 0);
        assert!(state.readonly(""));
        assert_ne!(state.root_mountflags & MS_RDONLY, 0);
        assert!(!state.readonly("unexpected"));

        assert!(state.root_dev_setup(
            "a-very-long-root-device-name-that-will-be-copied-but-truncated-at-sixty-three-bytes"
        ));
        assert_eq!(state.saved_root_name.len(), 63);
        assert!(state.root_delay_setup("0x10"));
        assert_eq!(state.root_delay, 16);
    }

    #[test]
    fn rootwait_timeout_falls_back_to_indefinite_on_invalid_values() {
        let mut state = DoMountsState::new();
        assert!(state.rootwait_setup(""));
        assert_eq!(state.root_wait, -1);
        assert!(!state.rootwait_setup("bad"));
        assert!(state.rootwait_timeout_setup("5"));
        assert_eq!(state.root_wait, 5000);
        assert!(state.rootwait_timeout_setup("-1"));
        assert_eq!(state.root_wait, -1);
        assert!(state.rootwait_timeout_setup("9999999999"));
        assert_eq!(state.root_wait, -1);
    }

    #[test]
    fn split_fs_names_preserves_linux_zero_length_entries() {
        assert_eq!(split_fs_names("ext4,,xfs,"), &["ext4", "", "xfs", ""]);
    }

    #[test]
    fn parse_root_device_handles_special_roots_and_invalid_rootwait() {
        assert_eq!(
            parse_root_device("mtd0", -1, Err(-EINVAL)),
            RootParseResult {
                root_dev: ROOT_GENERIC,
                root_wait: -1,
            }
        );
        assert_eq!(
            parse_root_device("/dev/nfs", 0, Err(-EINVAL)).root_dev,
            ROOT_NFS
        );
        assert_eq!(
            parse_root_device("/dev/cifs", 0, Err(-EINVAL)).root_dev,
            ROOT_CIFS
        );
        assert_eq!(
            parse_root_device("/dev/ram", 0, Err(-EINVAL)).root_dev,
            ROOT_RAM0
        );
        assert_eq!(
            parse_root_device("/dev/vda1", -1, Err(-EINVAL)),
            RootParseResult {
                root_dev: 0,
                root_wait: 0,
            }
        );
        assert_eq!(
            parse_root_device("/dev/vda1", -1, Ok(0x801)).root_dev,
            0x801
        );
    }

    #[test]
    fn rootfs_tmpfs_and_mount_kind_match_linux_selection() {
        assert!(rootfs_should_use_tmpfs(true, "", None));
        assert!(rootfs_should_use_tmpfs(
            true,
            "/dev/vda1",
            Some("ext4,tmpfs")
        ));
        assert!(!rootfs_should_use_tmpfs(true, "/dev/vda1", Some("ext4")));
        assert!(!rootfs_should_use_tmpfs(false, "", None));

        assert_eq!(
            mount_root_kind(ROOT_NFS, Some("/dev/nfs"), None),
            RootMountKind::Nfs
        );
        assert_eq!(
            mount_root_kind(0, Some("rootfs"), Some("tmpfs")),
            RootMountKind::NodevThenBlock
        );
        assert_eq!(
            mount_root_kind(0x801, Some("/dev/vda1"), None),
            RootMountKind::Block
        );
    }
}
