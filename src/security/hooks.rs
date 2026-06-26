//! linux-parity: partial
//! linux-source: vendor/linux/security
//! LSM hook table.
//!
//! Each slot is `Option<fn(...)>`; `None` means the LSM doesn't hook that
//! point.  Mirrors `vendor/linux/include/linux/lsm_hook_defs.h` (subset).
//!
//! Hooks return:
//! - `0` for allow / no opinion
//! - negative errno for deny

#[derive(Clone, Copy)]
pub struct LsmHooks {
    pub name: &'static str,
    pub id: u64,

    // ── Task lifecycle ──
    pub task_alloc: Option<fn(task_id: u32, clone_flags: u64) -> i32>,
    pub task_free: Option<fn(task_id: u32)>,

    // ── exec ──
    pub bprm_creds_for_exec: Option<fn(filename: &[u8]) -> i32>,
    pub bprm_check: Option<fn(filename: &[u8]) -> i32>,
    pub bprm_committing_creds: Option<fn(filename: &[u8])>,
    pub bprm_committed_creds: Option<fn(filename: &[u8])>,

    // ── credentials ──
    pub cred_alloc_blank: Option<fn() -> i32>,
    pub cred_prepare: Option<fn() -> i32>,
    pub cred_transfer: Option<fn()>,

    // ── capability check ──
    pub capable: Option<fn(cap: u32) -> i32>,

    // ── path / inode ──
    pub path_open: Option<fn(path: &[u8], flags: i32) -> i32>,
    pub inode_permission: Option<fn(ino: u64, mask: u32) -> i32>,
    pub inode_create: Option<fn(dir_ino: u64, name: &[u8], mode: u32) -> i32>,
    pub inode_unlink: Option<fn(dir_ino: u64, name: &[u8]) -> i32>,
    pub file_permission: Option<fn(fd: i32, mask: u32) -> i32>,

    // ── socket ──
    pub socket_create: Option<fn(family: i32, kind: i32, proto: i32) -> i32>,
}

pub const NOOP_HOOKS: LsmHooks = LsmHooks {
    name: "noop",
    id: LSM_ID_UNDEF,
    task_alloc: None,
    task_free: None,
    bprm_creds_for_exec: None,
    bprm_check: None,
    bprm_committing_creds: None,
    bprm_committed_creds: None,
    cred_alloc_blank: None,
    cred_prepare: None,
    cred_transfer: None,
    capable: None,
    path_open: None,
    inode_permission: None,
    inode_create: None,
    inode_unlink: None,
    file_permission: None,
    socket_create: None,
};

pub const LSM_ID_UNDEF: u64 = 0;
pub const LSM_ID_CAPABILITY: u64 = 100;
pub const LSM_ID_SELINUX: u64 = 101;
pub const LSM_ID_SMACK: u64 = 102;
pub const LSM_ID_TOMOYO: u64 = 103;
pub const LSM_ID_APPARMOR: u64 = 104;
pub const LSM_ID_YAMA: u64 = 105;
pub const LSM_ID_LOADPIN: u64 = 106;
pub const LSM_ID_SAFESETID: u64 = 107;
pub const LSM_ID_LOCKDOWN: u64 = 108;
pub const LSM_ID_BPF: u64 = 109;
pub const LSM_ID_LANDLOCK: u64 = 110;
pub const LSM_ID_IMA: u64 = 111;
pub const LSM_ID_EVM: u64 = 112;
pub const LSM_ID_IPE: u64 = 113;

pub const LSM_ATTR_UNDEF: u32 = 0;
pub const LSM_ATTR_CURRENT: u32 = 100;
pub const LSM_ATTR_EXEC: u32 = 101;
pub const LSM_ATTR_FSCREATE: u32 = 102;
pub const LSM_ATTR_KEYCREATE: u32 = 103;
pub const LSM_ATTR_PREV: u32 = 104;
pub const LSM_ATTR_SOCKCREATE: u32 = 105;

pub const LSM_FLAG_SINGLE: u32 = 0x0001;
