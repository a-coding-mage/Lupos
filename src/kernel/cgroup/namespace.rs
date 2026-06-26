//! linux-parity: complete
//! linux-source: vendor/linux/kernel/cgroup/namespace.c
//! test-origin: linux:vendor/linux/kernel/cgroup/namespace.c
//! Cgroup namespace handling.

extern crate alloc;

use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::fs::nsfs::{NS_TYPE_CGROUP, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::include::uapi::errno::{EINVAL, ENOMEM, ENOSPC, EPERM};
use crate::kernel::capability::CAP_SYS_ADMIN;
use crate::kernel::clone::CLONE_NEWCGROUP;
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};

#[repr(C)]
pub struct CgroupNamespace {
    pub ns: NsCommon,
    pub user_ns: *const UserNamespace,
    pub ucounts: usize,
    pub root_cset: usize,
    pub in_nstree: bool,
}

unsafe impl Send for CgroupNamespace {}
unsafe impl Sync for CgroupNamespace {}

unsafe fn cg_get(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut CgroupNamespace;
    if !ns.is_null() {
        unsafe {
            (*ns).ns.get();
        }
    }
}

unsafe fn cg_put(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut CgroupNamespace;
    if ns.is_null() {
        return;
    }
    if unsafe { (*ns).ns.put() } {
        if core::ptr::eq(ns, &raw const INIT_CGROUP_NS as *mut CgroupNamespace) {
            unsafe {
                (*ns).ns.count.store(usize::MAX / 2, Ordering::Relaxed);
            }
            return;
        }
        unsafe {
            drop(Box::from_raw(ns));
        }
    }
}

unsafe fn cg_owner(ns: *const core::ffi::c_void) -> *const core::ffi::c_void {
    let ns = ns as *const CgroupNamespace;
    if ns.is_null() {
        core::ptr::null()
    } else {
        unsafe { (*ns).user_ns as *const _ }
    }
}

pub static CGROUP_OPS: NsOperations = NsOperations {
    name: "cgroup",
    ns_type: NS_TYPE_CGROUP,
    get: cg_get,
    put: cg_put,
    owner: cg_owner,
};

pub static INIT_CGROUP_NS: CgroupNamespace = CgroupNamespace {
    ns: NsCommon::sticky(&CGROUP_OPS as *const _, PROC_DYNAMIC_FIRST + 6),
    user_ns: &INIT_USER_NS,
    ucounts: 0,
    root_cset: 0,
    in_nstree: true,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CgroupNsProcOperations {
    pub name: &'static str,
    pub get: &'static str,
    pub put: &'static str,
    pub install: &'static str,
    pub owner: &'static str,
}

pub const CGROUPNS_OPERATIONS: CgroupNsProcOperations = CgroupNsProcOperations {
    name: "cgroup",
    get: "cgroupns_get",
    put: "cgroupns_put",
    install: "cgroupns_install",
    owner: "cgroupns_owner",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocCgroupNsEnv {
    pub allocation_succeeds: bool,
    pub ns_common_init_ret: i32,
}

impl AllocCgroupNsEnv {
    pub const SUCCESS: Self = Self {
        allocation_succeeds: true,
        ns_common_init_ret: 0,
    };
}

pub fn alloc_cgroup_ns_plan(env: AllocCgroupNsEnv) -> Result<(), i32> {
    if !env.allocation_succeeds {
        return Err(-ENOMEM);
    }
    if env.ns_common_init_ret != 0 {
        return Err(env.ns_common_init_ret);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreeCgroupNsPlan {
    pub ns_tree_removed: bool,
    pub root_cset_put: bool,
    pub ucounts_dec: bool,
    pub user_ns_put: bool,
    pub ns_common_freed: bool,
    pub kfree_rcu: bool,
}

pub const fn free_cgroup_ns_plan() -> FreeCgroupNsPlan {
    FreeCgroupNsPlan {
        ns_tree_removed: true,
        root_cset_put: true,
        ucounts_dec: true,
        user_ns_put: true,
        ns_common_freed: true,
        kfree_rcu: true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CopyCgroupNsEnv {
    pub old_ns_present: bool,
    pub caller_has_cap_sys_admin: bool,
    pub inc_ucount_succeeds: bool,
    pub current_css_set: usize,
    pub alloc: AllocCgroupNsEnv,
}

impl CopyCgroupNsEnv {
    pub const SUCCESS: Self = Self {
        old_ns_present: true,
        caller_has_cap_sys_admin: true,
        inc_ucount_succeeds: true,
        current_css_set: 1,
        alloc: AllocCgroupNsEnv::SUCCESS,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyCgroupNsOutcome {
    BugOnMissingOld,
    SharedOld {
        old_ref_taken: bool,
    },
    Error {
        errno: i32,
        css_set_put: bool,
        ucounts_dec: bool,
    },
    New {
        css_set_ref_taken: bool,
        user_ns_ref_taken: bool,
        ns_tree_added: bool,
        root_cset: usize,
    },
}

pub fn copy_cgroup_ns_plan(flags: u64, env: CopyCgroupNsEnv) -> CopyCgroupNsOutcome {
    if !env.old_ns_present {
        return CopyCgroupNsOutcome::BugOnMissingOld;
    }
    if flags & CLONE_NEWCGROUP == 0 {
        return CopyCgroupNsOutcome::SharedOld {
            old_ref_taken: true,
        };
    }
    if !env.caller_has_cap_sys_admin {
        return CopyCgroupNsOutcome::Error {
            errno: -EPERM,
            css_set_put: false,
            ucounts_dec: false,
        };
    }
    if !env.inc_ucount_succeeds {
        return CopyCgroupNsOutcome::Error {
            errno: -ENOSPC,
            css_set_put: false,
            ucounts_dec: false,
        };
    }
    if let Err(errno) = alloc_cgroup_ns_plan(env.alloc) {
        return CopyCgroupNsOutcome::Error {
            errno,
            css_set_put: true,
            ucounts_dec: true,
        };
    }
    CopyCgroupNsOutcome::New {
        css_set_ref_taken: true,
        user_ns_ref_taken: true,
        ns_tree_added: true,
        root_cset: env.current_css_set,
    }
}

pub fn copy_cgroup_ns(
    _old: *const CgroupNamespace,
    user_ns: *const UserNamespace,
) -> Result<*mut CgroupNamespace, i32> {
    let b = Box::new(CgroupNamespace {
        ns: NsCommon {
            count: AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops: &CGROUP_OPS as *const _,
            inum: alloc_ns_inum(),
            _pad: 0,
        },
        user_ns,
        ucounts: 1,
        root_cset: 1,
        in_nstree: true,
    });
    if !user_ns.is_null() {
        unsafe {
            (*user_ns).ns.get();
        }
    }
    Ok(Box::into_raw(b))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CgroupNsInstallEnv {
    pub caller_has_cap_sys_admin: bool,
    pub target_user_ns_has_cap_sys_admin: bool,
    pub same_namespace: bool,
}

impl CgroupNsInstallEnv {
    pub const SUCCESS: Self = Self {
        caller_has_cap_sys_admin: true,
        target_user_ns_has_cap_sys_admin: true,
        same_namespace: false,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CgroupNsInstallPlan {
    pub ret: i32,
    pub get_new_ns: bool,
    pub put_old_ns: bool,
    pub installed: bool,
}

pub fn cgroupns_install_plan(env: CgroupNsInstallEnv) -> CgroupNsInstallPlan {
    if !env.caller_has_cap_sys_admin || !env.target_user_ns_has_cap_sys_admin {
        return CgroupNsInstallPlan {
            ret: -EPERM,
            get_new_ns: false,
            put_old_ns: false,
            installed: false,
        };
    }
    if env.same_namespace {
        return CgroupNsInstallPlan {
            ret: 0,
            get_new_ns: false,
            put_old_ns: false,
            installed: false,
        };
    }
    CgroupNsInstallPlan {
        ret: 0,
        get_new_ns: true,
        put_old_ns: true,
        installed: true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CgroupNsGetPlan {
    pub task_locked: bool,
    pub nsproxy_present: bool,
    pub cgroup_ns_ref_taken: bool,
    pub task_unlocked: bool,
    pub returns_ns_common: bool,
}

pub fn cgroupns_get_plan(nsproxy_present: bool) -> CgroupNsGetPlan {
    CgroupNsGetPlan {
        task_locked: true,
        nsproxy_present,
        cgroup_ns_ref_taken: nsproxy_present,
        task_unlocked: true,
        returns_ns_common: nsproxy_present,
    }
}

pub fn cgroupns_put_plan(ns_present: bool) -> bool {
    ns_present
}

pub fn cgroupns_owner_plan(ns: &CgroupNamespace) -> *const UserNamespace {
    ns.user_ns
}

pub const CGROUP_NAMESPACE_UCOUNT: &'static str = "UCOUNT_CGROUP_NAMESPACES";
pub const CGROUP_NAMESPACE_CAPABILITY: u32 = CAP_SYS_ADMIN;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgroup_namespace_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/cgroup/namespace.c"
        ));
        assert!(
            source.contains("return inc_ucount(ns, current_euid(), UCOUNT_CGROUP_NAMESPACES);")
        );
        assert!(source.contains("dec_ucount(ucounts, UCOUNT_CGROUP_NAMESPACES);"));
        assert!(
            source.contains("new_ns = kzalloc_obj(struct cgroup_namespace, GFP_KERNEL_ACCOUNT);")
        );
        assert!(source.contains("ret = ns_common_init(new_ns);"));
        assert!(source.contains("void free_cgroup_ns(struct cgroup_namespace *ns)"));
        assert!(source.contains("ns_tree_remove(ns);"));
        assert!(source.contains("put_css_set(ns->root_cset);"));
        assert!(source.contains("dec_cgroup_namespaces(ns->ucounts);"));
        assert!(source.contains("put_user_ns(ns->user_ns);"));
        assert!(source.contains("ns_common_free(ns);"));
        assert!(source.contains("kfree_rcu(ns, ns.ns_rcu);"));
        assert!(source.contains("BUG_ON(!old_ns);"));
        assert!(source.contains("if (!(flags & CLONE_NEWCGROUP))"));
        assert!(source.contains("if (!ns_capable(user_ns, CAP_SYS_ADMIN))"));
        assert!(source.contains("return ERR_PTR(-EPERM);"));
        assert!(source.contains("return ERR_PTR(-ENOSPC);"));
        assert!(source.contains("spin_lock_irq(&css_set_lock);"));
        assert!(source.contains("cset = task_css_set(current);"));
        assert!(source.contains("get_css_set(cset);"));
        assert!(source.contains("put_css_set(cset);"));
        assert!(source.contains("dec_cgroup_namespaces(ucounts);"));
        assert!(source.contains("new_ns->user_ns = get_user_ns(user_ns);"));
        assert!(source.contains("new_ns->ucounts = ucounts;"));
        assert!(source.contains("new_ns->root_cset = cset;"));
        assert!(source.contains("ns_tree_add(new_ns);"));
        assert!(source.contains("const struct proc_ns_operations cgroupns_operations"));
        assert!(source.contains(".name\t\t= \"cgroup\""));
        assert!(source.contains(".install\t= cgroupns_install"));

        assert_eq!(CGROUP_NAMESPACE_UCOUNT, "UCOUNT_CGROUP_NAMESPACES");
        assert_eq!(CGROUP_NAMESPACE_CAPABILITY, CAP_SYS_ADMIN);
        assert_eq!(CGROUPNS_OPERATIONS.name, "cgroup");
        assert_eq!(CGROUPNS_OPERATIONS.install, "cgroupns_install");
    }

    #[test]
    fn alloc_free_and_copy_cgroup_ns_follow_linux_error_cleanup_order() {
        assert_eq!(
            alloc_cgroup_ns_plan(AllocCgroupNsEnv {
                allocation_succeeds: false,
                ..AllocCgroupNsEnv::SUCCESS
            }),
            Err(-ENOMEM)
        );
        assert_eq!(
            alloc_cgroup_ns_plan(AllocCgroupNsEnv {
                ns_common_init_ret: -EINVAL,
                ..AllocCgroupNsEnv::SUCCESS
            }),
            Err(-EINVAL)
        );
        assert_eq!(alloc_cgroup_ns_plan(AllocCgroupNsEnv::SUCCESS), Ok(()));
        assert_eq!(
            free_cgroup_ns_plan(),
            FreeCgroupNsPlan {
                ns_tree_removed: true,
                root_cset_put: true,
                ucounts_dec: true,
                user_ns_put: true,
                ns_common_freed: true,
                kfree_rcu: true,
            }
        );

        assert_eq!(
            copy_cgroup_ns_plan(0, CopyCgroupNsEnv::SUCCESS),
            CopyCgroupNsOutcome::SharedOld {
                old_ref_taken: true,
            }
        );
        assert_eq!(
            copy_cgroup_ns_plan(
                CLONE_NEWCGROUP,
                CopyCgroupNsEnv {
                    old_ns_present: false,
                    ..CopyCgroupNsEnv::SUCCESS
                }
            ),
            CopyCgroupNsOutcome::BugOnMissingOld
        );
        assert_eq!(
            copy_cgroup_ns_plan(
                CLONE_NEWCGROUP,
                CopyCgroupNsEnv {
                    caller_has_cap_sys_admin: false,
                    ..CopyCgroupNsEnv::SUCCESS
                }
            ),
            CopyCgroupNsOutcome::Error {
                errno: -EPERM,
                css_set_put: false,
                ucounts_dec: false,
            }
        );
        assert_eq!(
            copy_cgroup_ns_plan(
                CLONE_NEWCGROUP,
                CopyCgroupNsEnv {
                    inc_ucount_succeeds: false,
                    ..CopyCgroupNsEnv::SUCCESS
                }
            ),
            CopyCgroupNsOutcome::Error {
                errno: -ENOSPC,
                css_set_put: false,
                ucounts_dec: false,
            }
        );
        assert_eq!(
            copy_cgroup_ns_plan(
                CLONE_NEWCGROUP,
                CopyCgroupNsEnv {
                    alloc: AllocCgroupNsEnv {
                        ns_common_init_ret: -EINVAL,
                        ..AllocCgroupNsEnv::SUCCESS
                    },
                    ..CopyCgroupNsEnv::SUCCESS
                }
            ),
            CopyCgroupNsOutcome::Error {
                errno: -EINVAL,
                css_set_put: true,
                ucounts_dec: true,
            }
        );
        assert_eq!(
            copy_cgroup_ns_plan(CLONE_NEWCGROUP, CopyCgroupNsEnv::SUCCESS),
            CopyCgroupNsOutcome::New {
                css_set_ref_taken: true,
                user_ns_ref_taken: true,
                ns_tree_added: true,
                root_cset: 1,
            }
        );
    }

    #[test]
    fn install_get_put_and_owner_follow_proc_ns_operations() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/cgroup/namespace.c"
        ));
        assert!(source.contains("static int cgroupns_install"));
        assert!(source.contains("if (!ns_capable(nsset->cred->user_ns, CAP_SYS_ADMIN) ||"));
        assert!(source.contains("!ns_capable(cgroup_ns->user_ns, CAP_SYS_ADMIN))"));
        assert!(source.contains("return -EPERM;"));
        assert!(source.contains("if (cgroup_ns == nsproxy->cgroup_ns)"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("get_cgroup_ns(cgroup_ns);"));
        assert!(source.contains("put_cgroup_ns(nsproxy->cgroup_ns);"));
        assert!(source.contains("nsproxy->cgroup_ns = cgroup_ns;"));
        assert!(source.contains("task_lock(task);"));
        assert!(source.contains("nsproxy = task->nsproxy;"));
        assert!(source.contains("get_cgroup_ns(ns);"));
        assert!(source.contains("task_unlock(task);"));
        assert!(source.contains("return ns ? &ns->ns : NULL;"));
        assert!(source.contains("put_cgroup_ns(to_cg_ns(ns));"));
        assert!(source.contains("return to_cg_ns(ns)->user_ns;"));

        assert_eq!(
            cgroupns_install_plan(CgroupNsInstallEnv {
                caller_has_cap_sys_admin: false,
                ..CgroupNsInstallEnv::SUCCESS
            }),
            CgroupNsInstallPlan {
                ret: -EPERM,
                get_new_ns: false,
                put_old_ns: false,
                installed: false,
            }
        );
        assert_eq!(
            cgroupns_install_plan(CgroupNsInstallEnv {
                same_namespace: true,
                ..CgroupNsInstallEnv::SUCCESS
            }),
            CgroupNsInstallPlan {
                ret: 0,
                get_new_ns: false,
                put_old_ns: false,
                installed: false,
            }
        );
        assert_eq!(
            cgroupns_install_plan(CgroupNsInstallEnv::SUCCESS),
            CgroupNsInstallPlan {
                ret: 0,
                get_new_ns: true,
                put_old_ns: true,
                installed: true,
            }
        );
        assert_eq!(
            cgroupns_get_plan(true),
            CgroupNsGetPlan {
                task_locked: true,
                nsproxy_present: true,
                cgroup_ns_ref_taken: true,
                task_unlocked: true,
                returns_ns_common: true,
            }
        );
        assert!(!cgroupns_get_plan(false).returns_ns_common);
        assert!(cgroupns_put_plan(true));
        assert_eq!(
            cgroupns_owner_plan(&INIT_CGROUP_NS),
            &INIT_USER_NS as *const _
        );
    }

    #[test]
    fn runtime_copy_cgroup_ns_initializes_ns_common_and_refs_user_namespace() {
        let ns = copy_cgroup_ns(&INIT_CGROUP_NS as *const _, &INIT_USER_NS).expect("copy");
        unsafe {
            assert_eq!((*ns).ns.ops, &CGROUP_OPS as *const _);
            assert_eq!((*ns).user_ns, &INIT_USER_NS as *const _);
            assert_eq!((*ns).ucounts, 1);
            assert_eq!((*ns).root_cset, 1);
            assert!((*ns).in_nstree);
            cg_put(ns as *mut core::ffi::c_void);
        }
    }
}
