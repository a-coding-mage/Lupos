//! linux-parity: complete
//! linux-source: vendor/linux/include/uapi
//! linux-source: vendor/linux/include/uapi/linux/mount.h
//! `mount(2)` UAPI flags — MS_*, the new-mount-API (fsopen/fsconfig/fsmount/
//! fspick/open_tree/move_mount), mount_attr, statmount/listmount, umount2.
//!
//! Ref: `vendor/linux/include/uapi/linux/mount.h`

#![allow(dead_code)]

use crate::include::uapi::fcntl::O_CLOEXEC;

pub const MS_RDONLY: u64 = 1 << 0;
pub const MS_NOSUID: u64 = 1 << 1;
pub const MS_NODEV: u64 = 1 << 2;
pub const MS_NOEXEC: u64 = 1 << 3;
pub const MS_SYNCHRONOUS: u64 = 1 << 4;
pub const MS_REMOUNT: u64 = 1 << 5;
pub const MS_MANDLOCK: u64 = 1 << 6;
pub const MS_DIRSYNC: u64 = 1 << 7;
pub const MS_NOSYMFOLLOW: u64 = 1 << 8;
pub const MS_NOATIME: u64 = 1 << 10;
pub const MS_NODIRATIME: u64 = 1 << 11;
pub const MS_BIND: u64 = 1 << 12;
pub const MS_MOVE: u64 = 1 << 13;
pub const MS_REC: u64 = 1 << 14;
pub const MS_SILENT: u64 = 1 << 15;
pub const MS_UNBINDABLE: u64 = 1 << 17;
pub const MS_PRIVATE: u64 = 1 << 18;
pub const MS_SLAVE: u64 = 1 << 19;
pub const MS_SHARED: u64 = 1 << 20;
pub const MS_RELATIME: u64 = 1 << 21;
pub const MS_KERNMOUNT: u64 = 1 << 22;
pub const MS_I_VERSION: u64 = 1 << 23;
pub const MS_STRICTATIME: u64 = 1 << 24;
pub const MS_LAZYTIME: u64 = 1 << 25;
pub const MS_VERBOSE: u64 = 1 << 15; // deprecated alias of MS_SILENT
pub const MS_POSIXACL: u64 = 1 << 16;
// Internal super-block flags (also surfaced via MS_*).
pub const MS_SUBMOUNT: u64 = 1 << 26;
pub const MS_NOREMOTELOCK: u64 = 1 << 27;
pub const MS_NOSEC: u64 = 1 << 28;
pub const MS_BORN: u64 = 1 << 29;
pub const MS_ACTIVE: u64 = 1 << 30;
pub const MS_NOUSER: u64 = 1 << 31;
/// Flags that can be altered by a remount.
pub const MS_RMT_MASK: u64 = MS_RDONLY | MS_SYNCHRONOUS | MS_MANDLOCK | MS_I_VERSION | MS_LAZYTIME;
/// Old magic mount-flag value / mask.
pub const MS_MGC_VAL: u64 = 0xC0ED0000;
pub const MS_MGC_MSK: u64 = 0xffff0000;

pub const MOVE_MOUNT_F_SYMLINKS: u32 = 0x00000001;
pub const MOVE_MOUNT_F_AUTOMOUNTS: u32 = 0x00000002;
pub const MOVE_MOUNT_F_EMPTY_PATH: u32 = 0x00000004;
pub const MOVE_MOUNT_T_SYMLINKS: u32 = 0x00000010;
pub const MOVE_MOUNT_T_AUTOMOUNTS: u32 = 0x00000020;
pub const MOVE_MOUNT_T_EMPTY_PATH: u32 = 0x00000040;
pub const MOVE_MOUNT_SET_GROUP: u32 = 0x00000100;
pub const MOVE_MOUNT_BENEATH: u32 = 0x00000200;
pub const MOVE_MOUNT_MASK: u32 = 0x00000377;

pub const FSOPEN_CLOEXEC: u32 = 0x00000001;

pub const FSCONFIG_SET_FLAG: u32 = 0;
pub const FSCONFIG_SET_STRING: u32 = 1;
pub const FSCONFIG_SET_BINARY: u32 = 2;
pub const FSCONFIG_SET_PATH: u32 = 3;
pub const FSCONFIG_SET_PATH_EMPTY: u32 = 4;
pub const FSCONFIG_SET_FD: u32 = 5;
pub const FSCONFIG_CMD_CREATE: u32 = 6;
pub const FSCONFIG_CMD_RECONFIGURE: u32 = 7;
pub const FSCONFIG_CMD_CREATE_EXCL: u32 = 8;

pub const FSMOUNT_CLOEXEC: u32 = 0x00000001;
pub const FSMOUNT_NAMESPACE: u32 = 0x00000002;

pub const FSPICK_CLOEXEC: u32 = 0x00000001;
pub const FSPICK_SYMLINK_NOFOLLOW: u32 = 0x00000002;
pub const FSPICK_NO_AUTOMOUNT: u32 = 0x00000004;
pub const FSPICK_EMPTY_PATH: u32 = 0x00000008;

pub const OPEN_TREE_CLONE: u32 = 1 << 0;
pub const OPEN_TREE_NAMESPACE: u32 = 1 << 1;
pub const OPEN_TREE_CLOEXEC: u32 = O_CLOEXEC;

pub const MOUNT_ATTR_RDONLY: u64 = 0x00000001;
pub const MOUNT_ATTR_NOSUID: u64 = 0x00000002;
pub const MOUNT_ATTR_NODEV: u64 = 0x00000004;
pub const MOUNT_ATTR_NOEXEC: u64 = 0x00000008;
pub const MOUNT_ATTR__ATIME: u64 = 0x00000070;
pub const MOUNT_ATTR_RELATIME: u64 = 0x00000000;
pub const MOUNT_ATTR_NOATIME: u64 = 0x00000010;
pub const MOUNT_ATTR_STRICTATIME: u64 = 0x00000020;
pub const MOUNT_ATTR_NODIRATIME: u64 = 0x00000080;
pub const MOUNT_ATTR_IDMAP: u64 = 0x00100000;
pub const MOUNT_ATTR_NOSYMFOLLOW: u64 = 0x00200000;
pub const MOUNT_ATTR_SUPPORTED: u64 = MOUNT_ATTR_RDONLY
    | MOUNT_ATTR_NOSUID
    | MOUNT_ATTR_NODEV
    | MOUNT_ATTR_NOEXEC
    | MOUNT_ATTR__ATIME
    | MOUNT_ATTR_NODIRATIME
    | MOUNT_ATTR_NOSYMFOLLOW;
pub const MOUNT_ATTR_SIZE_VER0: usize = 32;

// umount2 flags
pub const MNT_FORCE: u32 = 0x00000001;
pub const MNT_DETACH: u32 = 0x00000002;
pub const MNT_EXPIRE: u32 = 0x00000004;
pub const UMOUNT_NOFOLLOW: u32 = 0x00000008;

// mnt_id_req struct versions.
pub const MNT_ID_REQ_SIZE_VER0: usize = 24;
pub const MNT_ID_REQ_SIZE_VER1: usize = 32;

// statmount(2) request/result masks (`stmt_mask`).
pub const STATMOUNT_SB_BASIC: u32 = 0x00000001;
pub const STATMOUNT_MNT_BASIC: u32 = 0x00000002;
pub const STATMOUNT_PROPAGATE_FROM: u32 = 0x00000004;
pub const STATMOUNT_MNT_ROOT: u32 = 0x00000008;
pub const STATMOUNT_MNT_POINT: u32 = 0x00000010;
pub const STATMOUNT_FS_TYPE: u32 = 0x00000020;
pub const STATMOUNT_MNT_NS_ID: u32 = 0x00000040;
pub const STATMOUNT_MNT_OPTS: u32 = 0x00000080;
pub const STATMOUNT_FS_SUBTYPE: u32 = 0x00000100;
pub const STATMOUNT_SB_SOURCE: u32 = 0x00000200;
pub const STATMOUNT_OPT_ARRAY: u32 = 0x00000400;
pub const STATMOUNT_OPT_SEC_ARRAY: u32 = 0x00000800;
pub const STATMOUNT_SUPPORTED_MASK: u32 = 0x00001000;
pub const STATMOUNT_MNT_UIDMAP: u32 = 0x00002000;
pub const STATMOUNT_MNT_GIDMAP: u32 = 0x00004000;
/// `statmount()` flag: want mountinfo for the given fd.
pub const STATMOUNT_BY_FD: u32 = 0x00000001;

// listmount(2).
pub const LSMT_ROOT: u64 = 0xffffffffffffffff;
pub const LISTMOUNT_REVERSE: u32 = 1 << 0;
