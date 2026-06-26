//! linux-parity: partial
//! linux-source: vendor/linux/security
//! `security_*()` dispatch functions.
//!
//! Each fn iterates the registered-LSM list and short-circuits on the first
//! non-zero return.  Mirrors `vendor/linux/security/security.c::call_int_hook`.

use super::lsm_list::{call_int_hook, call_void_hook};

pub fn security_task_alloc(task_id: u32, clone_flags: u64) -> i32 {
    call_int_hook(|h| h.task_alloc.map(|f| f(task_id, clone_flags)).unwrap_or(0))
}

pub fn security_task_free(task_id: u32) {
    call_void_hook(|h| {
        if let Some(f) = h.task_free {
            f(task_id);
        }
    });
}

pub fn security_bprm_creds_for_exec(filename: &[u8]) -> i32 {
    call_int_hook(|h| h.bprm_creds_for_exec.map(|f| f(filename)).unwrap_or(0))
}

pub fn security_bprm_check(filename: &[u8]) -> i32 {
    call_int_hook(|h| h.bprm_check.map(|f| f(filename)).unwrap_or(0))
}

pub fn security_bprm_committing_creds(filename: &[u8]) {
    call_void_hook(|h| {
        if let Some(f) = h.bprm_committing_creds {
            f(filename);
        }
    });
}

pub fn security_bprm_committed_creds(filename: &[u8]) {
    call_void_hook(|h| {
        if let Some(f) = h.bprm_committed_creds {
            f(filename);
        }
    });
}

pub fn security_cred_prepare() -> i32 {
    call_int_hook(|h| h.cred_prepare.map(|f| f()).unwrap_or(0))
}

pub fn security_capable(cap: u32) -> i32 {
    call_int_hook(|h| h.capable.map(|f| f(cap)).unwrap_or(0))
}

pub fn security_path_open(path: &[u8], flags: i32) -> i32 {
    call_int_hook(|h| h.path_open.map(|f| f(path, flags)).unwrap_or(0))
}

pub fn security_inode_permission(ino: u64, mask: u32) -> i32 {
    call_int_hook(|h| h.inode_permission.map(|f| f(ino, mask)).unwrap_or(0))
}

pub fn security_socket_create(family: i32, kind: i32, proto: i32) -> i32 {
    call_int_hook(|h| h.socket_create.map(|f| f(family, kind, proto)).unwrap_or(0))
}
