//! linux-parity: partial
//! linux-source: vendor/linux/security
//! Linux Security Module (LSM) framework — M64.
//!
//! Mirrors the design of `vendor/linux/security/security.c` and
//! `vendor/linux/include/linux/lsm_hook_defs.h`.
//!
//! M64 ships:
//! - A static `LsmHooks` table of optional function pointers, one per hook
//!   that Linux defines (subset).
//! - A registry of registered LSMs (max 8).
//! - `security_*()` dispatch functions that walk the registry, calling each
//!   LSM's hook in order; first non-zero return short-circuits.
//! - The capabilities LSM, registered as the default and only LSM.
//!
//! Per-process LSM blobs (`task_security`, `cred_security`, `inode_security`)
//! are deferred — only the cap LSM is registered, and it stores no per-task
//! state of its own.

pub mod apparmor;
pub mod apparmorfs;
pub mod blobs;
pub mod bpf;
pub mod cap_lsm;
pub mod certs;
pub mod dispatch;
pub mod hooks;
pub mod inode;
pub mod integrity;
pub mod ipe;
pub mod keys;
pub mod landlock;
pub mod linux_sources;
pub mod lsm_list;
pub mod lsm_notifier;
pub mod min_addr;
pub mod platform_certs;
pub mod selinux;
pub mod smack;
pub mod tomoyo;

pub use dispatch::*;
pub use hooks::LsmHooks;
pub use lsm_list::{lsm_active_count, lsm_active_ids, register_lsm};

/// Initialise the security framework.  Idempotent.  Registers the cap LSM and
/// built-in Linux key/certificate types.
pub fn init() {
    cap_lsm::register();
    apparmor::init();
    keys::init();
    certs::init();
    platform_certs::init();
    integrity::init();
}
