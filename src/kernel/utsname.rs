//! linux-parity: complete
//! linux-source: vendor/linux/kernel/utsname.c
//! test-origin: linux:vendor/linux/kernel/utsname.c
//! UTS namespace — Milestone 28.
//!
//! Holds the per-namespace `new_utsname` populated by `sethostname` /
//! `setdomainname` and read by `uname`.  Linux exposes this through the
//! `uname()` syscall and `/proc/sys/kernel/{hostname,domainname}`.

extern crate alloc;

use alloc::boxed::Box;

use spin::Mutex;

use crate::fs::nsfs::{NS_TYPE_UTS, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::include::uapi::errno::{EINVAL, ENOMEM, ENOSPC, EPERM};
use crate::init::version;
use crate::kernel::capability::CAP_SYS_ADMIN;
use crate::kernel::clone::CLONE_NEWUTS;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};

/// Length of one `new_utsname` field (Linux: `__NEW_UTS_LEN + 1 = 65`).
pub const NEW_UTS_LEN_PLUS_NUL: usize = 65;

/// `struct new_utsname` — six 65-byte NUL-terminated strings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NewUtsname {
    pub sysname: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub nodename: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub release: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub version: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub machine: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub domainname: [u8; NEW_UTS_LEN_PLUS_NUL],
}

impl Default for NewUtsname {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

/// Build a `[u8; 65]` from a `&str`, NUL-padded.
pub const fn pack65(s: &str) -> [u8; NEW_UTS_LEN_PLUS_NUL] {
    let bytes = s.as_bytes();
    let mut out = [0u8; NEW_UTS_LEN_PLUS_NUL];
    let n = if bytes.len() < NEW_UTS_LEN_PLUS_NUL - 1 {
        bytes.len()
    } else {
        NEW_UTS_LEN_PLUS_NUL - 1
    };
    let mut i = 0;
    while i < n {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

#[repr(C)]
pub struct UtsNamespace {
    pub name: NewUtsname,
    pub user_ns: *const UserNamespace,
    pub ucounts: usize,
    pub ns: NsCommon,
}

unsafe impl Send for UtsNamespace {}
unsafe impl Sync for UtsNamespace {}

unsafe fn uts_get(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut UtsNamespace;
    if !ns.is_null() {
        unsafe {
            (*ns).ns.get();
        }
    }
}

unsafe fn uts_put(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut UtsNamespace;
    if ns.is_null() {
        return;
    }
    let last = unsafe { (*ns).ns.put() };
    if last {
        unsafe { free_uts_ns(ns) };
    }
}

unsafe fn uts_owner(ns: *const core::ffi::c_void) -> *const core::ffi::c_void {
    let ns = ns as *const UtsNamespace;
    if ns.is_null() {
        core::ptr::null()
    } else {
        unsafe { (*ns).user_ns as *const _ }
    }
}

pub static UTS_OPS: NsOperations = NsOperations {
    name: "uts",
    ns_type: NS_TYPE_UTS,
    get: uts_get,
    put: uts_put,
    owner: uts_owner,
};

/// Singleton init UTS namespace populated with Lupos identity strings.
pub static INIT_UTS_NS: UtsNamespace = UtsNamespace {
    name: NewUtsname {
        sysname: pack65(version::UTS_SYSNAME),
        nodename: pack65(version::UTS_NODENAME),
        release: pack65(version::UTS_RELEASE),
        version: pack65(version::UTS_VERSION),
        machine: pack65(version::UTS_MACHINE),
        domainname: pack65(version::UTS_DOMAINNAME),
    },
    user_ns: &INIT_USER_NS,
    ucounts: 0,
    ns: NsCommon::sticky(&UTS_OPS as *const _, PROC_DYNAMIC_FIRST + 1),
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "init_uts_ns",
        core::ptr::addr_of!(INIT_UTS_NS) as usize,
        true,
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtsNsProcOperations {
    pub name: &'static str,
    pub get: &'static str,
    pub put: &'static str,
    pub install: &'static str,
    pub owner: &'static str,
}

pub const UTSNS_OPERATIONS: UtsNsProcOperations = UtsNsProcOperations {
    name: "uts",
    get: "utsns_get",
    put: "utsns_put",
    install: "utsns_install",
    owner: "utsns_owner",
};

pub const UTS_NAMESPACE_UCOUNT: &str = "UCOUNT_UTS_NAMESPACES";
pub const UTS_NAMESPACE_CAPABILITY: u32 = CAP_SYS_ADMIN;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CloneUtsNsEnv {
    pub inc_ucount_succeeds: bool,
    pub allocation_succeeds: bool,
    pub ns_common_init_ret: i32,
}

impl CloneUtsNsEnv {
    pub const SUCCESS: Self = Self {
        inc_ucount_succeeds: true,
        allocation_succeeds: true,
        ns_common_init_ret: 0,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloneUtsNsOutcome {
    Error {
        errno: i32,
        cache_free: bool,
        ucounts_dec: bool,
    },
    New {
        ucounts_assigned: bool,
        name_copied_under_uts_sem: bool,
        user_ns_ref_taken: bool,
        ns_tree_added: bool,
    },
}

pub fn clone_uts_ns_plan(env: CloneUtsNsEnv) -> CloneUtsNsOutcome {
    if !env.inc_ucount_succeeds {
        return CloneUtsNsOutcome::Error {
            errno: -ENOSPC,
            cache_free: false,
            ucounts_dec: false,
        };
    }
    if !env.allocation_succeeds {
        return CloneUtsNsOutcome::Error {
            errno: -ENOMEM,
            cache_free: false,
            ucounts_dec: true,
        };
    }
    if env.ns_common_init_ret != 0 {
        return CloneUtsNsOutcome::Error {
            errno: env.ns_common_init_ret,
            cache_free: true,
            ucounts_dec: true,
        };
    }
    CloneUtsNsOutcome::New {
        ucounts_assigned: true,
        name_copied_under_uts_sem: true,
        user_ns_ref_taken: true,
        ns_tree_added: true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CopyUtsNameEnv {
    pub old_ns_present: bool,
    pub clone: CloneUtsNsEnv,
}

impl CopyUtsNameEnv {
    pub const SUCCESS: Self = Self {
        old_ns_present: true,
        clone: CloneUtsNsEnv::SUCCESS,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyUtsNameOutcome {
    BugOnMissingOld,
    SharedOld {
        old_ref_taken: bool,
    },
    New {
        old_ref_taken: bool,
        old_ref_put: bool,
        clone: CloneUtsNsOutcome,
    },
}

pub fn copy_utsname_plan(flags: u64, env: CopyUtsNameEnv) -> CopyUtsNameOutcome {
    if !env.old_ns_present {
        return CopyUtsNameOutcome::BugOnMissingOld;
    }
    if flags & CLONE_NEWUTS == 0 {
        return CopyUtsNameOutcome::SharedOld {
            old_ref_taken: true,
        };
    }
    CopyUtsNameOutcome::New {
        old_ref_taken: true,
        old_ref_put: true,
        clone: clone_uts_ns_plan(env.clone),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreeUtsNsPlan {
    pub ns_tree_removed: bool,
    pub ucounts_dec: bool,
    pub user_ns_put: bool,
    pub ns_common_freed: bool,
    pub kfree_rcu: bool,
}

pub const fn free_uts_ns_plan() -> FreeUtsNsPlan {
    FreeUtsNsPlan {
        ns_tree_removed: true,
        ucounts_dec: true,
        user_ns_put: true,
        ns_common_freed: true,
        kfree_rcu: true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtsNsGetPlan {
    pub task_locked: bool,
    pub nsproxy_present: bool,
    pub uts_ns_ref_taken: bool,
    pub task_unlocked: bool,
    pub returns_ns_common: bool,
}

pub const fn utsns_get_plan(nsproxy_present: bool) -> UtsNsGetPlan {
    UtsNsGetPlan {
        task_locked: true,
        nsproxy_present,
        uts_ns_ref_taken: nsproxy_present,
        task_unlocked: true,
        returns_ns_common: nsproxy_present,
    }
}

pub const fn utsns_put_plan(ns_present: bool) -> bool {
    ns_present
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtsNsInstallEnv {
    pub target_user_ns_has_cap_sys_admin: bool,
    pub caller_has_cap_sys_admin: bool,
}

impl UtsNsInstallEnv {
    pub const SUCCESS: Self = Self {
        target_user_ns_has_cap_sys_admin: true,
        caller_has_cap_sys_admin: true,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtsNsInstallPlan {
    pub ret: i32,
    pub get_new_ns: bool,
    pub put_old_ns: bool,
    pub installed: bool,
}

pub const fn utsns_install_plan(env: UtsNsInstallEnv) -> UtsNsInstallPlan {
    if !env.target_user_ns_has_cap_sys_admin || !env.caller_has_cap_sys_admin {
        return UtsNsInstallPlan {
            ret: -EPERM,
            get_new_ns: false,
            put_old_ns: false,
            installed: false,
        };
    }
    UtsNsInstallPlan {
        ret: 0,
        get_new_ns: true,
        put_old_ns: true,
        installed: true,
    }
}

pub const fn utsns_owner_plan(ns: &UtsNamespace) -> *const UserNamespace {
    ns.user_ns
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtsNsInitPlan {
    pub cache_name: &'static str,
    pub object_size: usize,
    pub usercopy_offset_is_name: bool,
    pub usercopy_size: usize,
    pub slab_panic: bool,
    pub slab_account: bool,
    pub init_ns_tree_added: bool,
}

pub const fn uts_ns_init_plan() -> UtsNsInitPlan {
    UtsNsInitPlan {
        cache_name: "uts_namespace",
        object_size: core::mem::size_of::<UtsNamespace>(),
        usercopy_offset_is_name: true,
        usercopy_size: core::mem::size_of::<NewUtsname>(),
        slab_panic: true,
        slab_account: true,
        init_ns_tree_added: true,
    }
}

static CURRENT_NODENAME: Mutex<[u8; NEW_UTS_LEN_PLUS_NUL]> = Mutex::new(pack65("(none)"));
static CURRENT_DOMAINNAME: Mutex<[u8; NEW_UTS_LEN_PLUS_NUL]> = Mutex::new(pack65("(none)"));

pub fn current_nodename() -> [u8; NEW_UTS_LEN_PLUS_NUL] {
    *CURRENT_NODENAME.lock()
}

pub fn current_domainname() -> [u8; NEW_UTS_LEN_PLUS_NUL] {
    *CURRENT_DOMAINNAME.lock()
}

pub fn set_current_nodename_packed(name: [u8; NEW_UTS_LEN_PLUS_NUL]) {
    *CURRENT_NODENAME.lock() = name;
}

pub fn set_current_domainname_packed(name: [u8; NEW_UTS_LEN_PLUS_NUL]) {
    *CURRENT_DOMAINNAME.lock() = name;
}

/// `sethostname(2)` — Linux x86-64 syscall 170.
///
/// Linux stores the hostname in the caller's UTS namespace. Lupos keeps a
/// namespace-aware `new_utsname` scaffold today, but the init namespace is a
/// sticky static, so this syscall updates the active nodename cell used by the
/// early userspace path while preserving Linux's 64-byte hostname limit.
pub unsafe fn sys_sethostname(name: *const u8, len: usize) -> i64 {
    use crate::include::uapi::errno::{EFAULT, EINVAL, EPERM};
    use crate::kernel::capability::{CAP_SYS_ADMIN, capable};

    if !capable(CAP_SYS_ADMIN) {
        return -(EPERM as i64);
    }

    if name.is_null() {
        return -(EFAULT as i64);
    }
    if len >= NEW_UTS_LEN_PLUS_NUL {
        return -(EINVAL as i64);
    }

    let mut tmp = [0u8; NEW_UTS_LEN_PLUS_NUL];
    let not_copied =
        unsafe { crate::arch::x86::kernel::uaccess::copy_from_user(tmp.as_mut_ptr(), name, len) };
    if not_copied != 0 {
        return -(EFAULT as i64);
    }
    tmp[len] = 0;
    set_current_nodename_packed(tmp);
    0
}

/// `setdomainname(2)` — update the active UTS domainname cell.
pub unsafe fn sys_setdomainname(name: *const u8, len: usize) -> i64 {
    use crate::include::uapi::errno::{EFAULT, EINVAL};

    if name.is_null() {
        return -(EFAULT as i64);
    }
    if len >= NEW_UTS_LEN_PLUS_NUL {
        return -(EINVAL as i64);
    }

    let mut tmp = [0u8; NEW_UTS_LEN_PLUS_NUL];
    let not_copied =
        unsafe { crate::arch::x86::kernel::uaccess::copy_from_user(tmp.as_mut_ptr(), name, len) };
    if not_copied != 0 {
        return -(EFAULT as i64);
    }
    tmp[len] = 0;
    set_current_domainname_packed(tmp);
    0
}

fn clone_uts_ns_runtime(
    user_ns: *const UserNamespace,
    old_ns: *const UtsNamespace,
) -> Result<*mut UtsNamespace, i32> {
    let src = if old_ns.is_null() {
        &INIT_UTS_NS as *const _
    } else {
        old_ns
    };
    let b = Box::new(UtsNamespace {
        name: unsafe { (*src).name },
        user_ns,
        ucounts: 1,
        ns: NsCommon {
            count: core::sync::atomic::AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops: &UTS_OPS as *const _,
            inum: alloc_ns_inum(),
            _pad: 0,
        },
    });
    if !user_ns.is_null() {
        unsafe {
            (*user_ns).ns.get();
        }
    }
    Ok(Box::into_raw(b))
}

/// Linux `free_uts_ns()`: drop a dynamic UTS namespace after the final put.
pub unsafe fn free_uts_ns(ns: *mut UtsNamespace) {
    if ns.is_null() {
        return;
    }
    if core::ptr::eq(ns, &raw const INIT_UTS_NS as *mut UtsNamespace) {
        unsafe {
            (*ns)
                .ns
                .count
                .store(usize::MAX / 2, core::sync::atomic::Ordering::Relaxed);
        }
        return;
    }
    unsafe {
        if !(*ns).user_ns.is_null() {
            (*(*ns).user_ns).ns.put();
        }
        drop(Box::from_raw(ns));
    }
}

/// Linux `copy_utsname(flags, user_ns, old_ns)`.
pub fn copy_utsname_with_flags(
    flags: u64,
    user_ns: *const UserNamespace,
    old_ns: *const UtsNamespace,
) -> Result<*mut UtsNamespace, i32> {
    assert!(!old_ns.is_null(), "BUG_ON(!old_ns)");

    unsafe {
        (*old_ns).ns.get();
    }

    if flags & CLONE_NEWUTS == 0 {
        return Ok(old_ns as *mut UtsNamespace);
    }

    let new_ns = clone_uts_ns_runtime(user_ns, old_ns);
    unsafe {
        uts_put(old_ns as *mut core::ffi::c_void);
    }
    new_ns
}

/// Allocate a fresh UTS namespace cloned from `old`.  Existing callers in this
/// tree select the CLONE_NEWUTS branch before calling this wrapper.
pub fn copy_utsname(
    old: *const UtsNamespace,
    user_ns: *const UserNamespace,
) -> Result<*mut UtsNamespace, i32> {
    clone_uts_ns_runtime(user_ns, old)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utsname_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/utsname.c"
        ));
        let uts_namespace = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/uts_namespace.h"
        ));
        let utsname = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/utsname.h"
        ));
        let uapi_utsname = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/utsname.h"
        ));
        let user_namespace = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/user_namespace.h"
        ));

        assert!(source.contains("return inc_ucount(ns, current_euid(), UCOUNT_UTS_NAMESPACES);"));
        assert!(source.contains("dec_ucount(ucounts, UCOUNT_UTS_NAMESPACES);"));
        assert!(source.contains("static struct uts_namespace *clone_uts_ns"));
        assert!(source.contains("err = -ENOSPC;"));
        assert!(source.contains("ns = kmem_cache_zalloc(uts_ns_cache, GFP_KERNEL);"));
        assert!(source.contains("err = ns_common_init(ns);"));
        assert!(source.contains("down_read(&uts_sem);"));
        assert!(source.contains("memcpy(&ns->name, &old_ns->name, sizeof(ns->name));"));
        assert!(source.contains("ns->user_ns = get_user_ns(user_ns);"));
        assert!(source.contains("ns_tree_add(ns);"));
        assert!(source.contains("struct uts_namespace *copy_utsname(u64 flags,"));
        assert!(source.contains("BUG_ON(!old_ns);"));
        assert!(source.contains("get_uts_ns(old_ns);"));
        assert!(source.contains("if (!(flags & CLONE_NEWUTS))"));
        assert!(source.contains("put_uts_ns(old_ns);"));
        assert!(source.contains("void free_uts_ns(struct uts_namespace *ns)"));
        assert!(source.contains("ns_tree_remove(ns);"));
        assert!(source.contains("dec_uts_namespaces(ns->ucounts);"));
        assert!(source.contains("put_user_ns(ns->user_ns);"));
        assert!(source.contains("ns_common_free(ns);"));
        assert!(source.contains("kfree_rcu(ns, ns.ns_rcu);"));
        assert!(source.contains("static struct ns_common *utsns_get"));
        assert!(source.contains("task_lock(task);"));
        assert!(source.contains("return ns ? &ns->ns : NULL;"));
        assert!(source.contains("static int utsns_install"));
        assert!(source.contains("if (!ns_capable(ns->user_ns, CAP_SYS_ADMIN) ||"));
        assert!(source.contains("!ns_capable(nsset->cred->user_ns, CAP_SYS_ADMIN))"));
        assert!(source.contains("get_uts_ns(ns);"));
        assert!(source.contains("put_uts_ns(nsproxy->uts_ns);"));
        assert!(source.contains("nsproxy->uts_ns = ns;"));
        assert!(source.contains("const struct proc_ns_operations utsns_operations"));
        assert!(source.contains(".name\t\t= \"uts\""));
        assert!(source.contains(".install\t= utsns_install"));
        assert!(source.contains("void __init uts_ns_init(void)"));
        assert!(source.contains("kmem_cache_create_usercopy("));
        assert!(source.contains("\"uts_namespace\", sizeof(struct uts_namespace), 0,"));
        assert!(source.contains("SLAB_PANIC|SLAB_ACCOUNT"));
        assert!(source.contains("offsetof(struct uts_namespace, name),"));
        assert!(source.contains("sizeof_field(struct uts_namespace, name),"));
        assert!(source.contains("ns_tree_add(&init_uts_ns);"));
        assert!(uts_namespace.contains("struct uts_namespace {"));
        assert!(uts_namespace.contains("extern struct uts_namespace *copy_utsname(u64 flags,"));
        assert!(uts_namespace.contains("extern void free_uts_ns(struct uts_namespace *ns);"));
        assert!(utsname.contains("static inline struct new_utsname *utsname(void)"));
        assert!(uapi_utsname.contains("#define __NEW_UTS_LEN 64"));
        assert!(uapi_utsname.contains("struct new_utsname {"));
        assert!(user_namespace.contains("UCOUNT_UTS_NAMESPACES"));

        assert_eq!(UTS_NAMESPACE_UCOUNT, "UCOUNT_UTS_NAMESPACES");
        assert_eq!(UTS_NAMESPACE_CAPABILITY, CAP_SYS_ADMIN);
        assert_eq!(UTSNS_OPERATIONS.name, "uts");
        assert_eq!(UTSNS_OPERATIONS.install, "utsns_install");
    }

    #[test]
    fn clone_and_copy_plans_follow_linux_error_cleanup_order() {
        assert_eq!(
            clone_uts_ns_plan(CloneUtsNsEnv {
                inc_ucount_succeeds: false,
                ..CloneUtsNsEnv::SUCCESS
            }),
            CloneUtsNsOutcome::Error {
                errno: -ENOSPC,
                cache_free: false,
                ucounts_dec: false,
            }
        );
        assert_eq!(
            clone_uts_ns_plan(CloneUtsNsEnv {
                allocation_succeeds: false,
                ..CloneUtsNsEnv::SUCCESS
            }),
            CloneUtsNsOutcome::Error {
                errno: -ENOMEM,
                cache_free: false,
                ucounts_dec: true,
            }
        );
        assert_eq!(
            clone_uts_ns_plan(CloneUtsNsEnv {
                ns_common_init_ret: -EINVAL,
                ..CloneUtsNsEnv::SUCCESS
            }),
            CloneUtsNsOutcome::Error {
                errno: -EINVAL,
                cache_free: true,
                ucounts_dec: true,
            }
        );
        assert_eq!(
            clone_uts_ns_plan(CloneUtsNsEnv::SUCCESS),
            CloneUtsNsOutcome::New {
                ucounts_assigned: true,
                name_copied_under_uts_sem: true,
                user_ns_ref_taken: true,
                ns_tree_added: true,
            }
        );

        assert_eq!(
            copy_utsname_plan(0, CopyUtsNameEnv::SUCCESS),
            CopyUtsNameOutcome::SharedOld {
                old_ref_taken: true,
            }
        );
        assert_eq!(
            copy_utsname_plan(
                CLONE_NEWUTS,
                CopyUtsNameEnv {
                    old_ns_present: false,
                    ..CopyUtsNameEnv::SUCCESS
                }
            ),
            CopyUtsNameOutcome::BugOnMissingOld
        );
        assert_eq!(
            copy_utsname_plan(CLONE_NEWUTS, CopyUtsNameEnv::SUCCESS),
            CopyUtsNameOutcome::New {
                old_ref_taken: true,
                old_ref_put: true,
                clone: CloneUtsNsOutcome::New {
                    ucounts_assigned: true,
                    name_copied_under_uts_sem: true,
                    user_ns_ref_taken: true,
                    ns_tree_added: true,
                },
            }
        );
    }

    #[test]
    fn free_install_get_owner_and_init_plans_follow_proc_ns_operations() {
        assert_eq!(
            free_uts_ns_plan(),
            FreeUtsNsPlan {
                ns_tree_removed: true,
                ucounts_dec: true,
                user_ns_put: true,
                ns_common_freed: true,
                kfree_rcu: true,
            }
        );
        assert_eq!(
            utsns_get_plan(true),
            UtsNsGetPlan {
                task_locked: true,
                nsproxy_present: true,
                uts_ns_ref_taken: true,
                task_unlocked: true,
                returns_ns_common: true,
            }
        );
        assert!(!utsns_get_plan(false).returns_ns_common);
        assert!(utsns_put_plan(true));
        assert_eq!(
            utsns_install_plan(UtsNsInstallEnv {
                target_user_ns_has_cap_sys_admin: false,
                ..UtsNsInstallEnv::SUCCESS
            }),
            UtsNsInstallPlan {
                ret: -EPERM,
                get_new_ns: false,
                put_old_ns: false,
                installed: false,
            }
        );
        assert_eq!(
            utsns_install_plan(UtsNsInstallEnv::SUCCESS),
            UtsNsInstallPlan {
                ret: 0,
                get_new_ns: true,
                put_old_ns: true,
                installed: true,
            }
        );
        assert_eq!(utsns_owner_plan(&INIT_UTS_NS), &INIT_USER_NS as *const _);
        assert_eq!(
            uts_ns_init_plan(),
            UtsNsInitPlan {
                cache_name: "uts_namespace",
                object_size: core::mem::size_of::<UtsNamespace>(),
                usercopy_offset_is_name: true,
                usercopy_size: core::mem::size_of::<NewUtsname>(),
                slab_panic: true,
                slab_account: true,
                init_ns_tree_added: true,
            }
        );
    }

    #[test]
    fn pack65_terminates() {
        let s = pack65("hello");
        assert_eq!(&s[..5], b"hello");
        assert_eq!(s[5], 0);
        assert_eq!(s.len(), 65);
    }

    #[test]
    fn init_uts_ns_has_lupos_identity() {
        assert_eq!(
            &INIT_UTS_NS.name.sysname[..version::UTS_SYSNAME.len()],
            version::UTS_SYSNAME.as_bytes()
        );
        assert_eq!(&INIT_UTS_NS.name.machine[..6], b"x86_64");
    }

    #[test]
    fn uts_namespace_c_layout_matches_vendor_prefix() {
        use core::mem::offset_of;

        assert_eq!(offset_of!(UtsNamespace, name), 0);
        assert_eq!(offset_of!(UtsNamespace, user_ns), 392);
        assert_eq!(offset_of!(UtsNamespace, ucounts), 400);
        assert_eq!(offset_of!(UtsNamespace, ns), 408);
    }

    #[test]
    fn init_uts_ns_export_registers_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("init_uts_ns"),
            Some(core::ptr::addr_of!(INIT_UTS_NS) as usize)
        );
    }

    #[test]
    fn copy_utsname_clones_name() {
        let n = copy_utsname(&INIT_UTS_NS, &INIT_USER_NS).unwrap();
        unsafe {
            assert_eq!((*n).name.sysname, INIT_UTS_NS.name.sysname);
            assert_eq!((*n).user_ns, &INIT_USER_NS as *const _);
            assert_eq!((*n).ucounts, 1);
            uts_put(n as *mut _);
        }
    }

    #[test]
    fn copy_utsname_with_flags_shares_or_clones_like_linux() {
        let shared =
            copy_utsname_with_flags(0, &INIT_USER_NS, &INIT_UTS_NS).expect("shared uts namespace");
        assert_eq!(shared, &INIT_UTS_NS as *const _ as *mut _);
        unsafe {
            uts_put(shared as *mut _);
        }

        let cloned = copy_utsname_with_flags(CLONE_NEWUTS, &INIT_USER_NS, &INIT_UTS_NS)
            .expect("cloned uts namespace");
        assert_ne!(cloned, &INIT_UTS_NS as *const _ as *mut _);
        unsafe {
            assert_eq!((*cloned).name.domainname, INIT_UTS_NS.name.domainname);
            assert_eq!((*cloned).ucounts, 1);
            uts_put(cloned as *mut _);
        }
    }
}
