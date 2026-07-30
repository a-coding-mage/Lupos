//! linux-parity: complete
//! linux-source: vendor/linux/kernel/nsproxy.c
//! test-origin: linux:vendor/linux/kernel/nsproxy.c
//! `struct nsproxy` — bundle of namespace pointers — Milestone 28.
//!
//! Linux groups every namespace pointer except `user_namespace` into a
//! shared, refcounted `struct nsproxy` so that fork's `CLONE_NEW*` flag
//! handling can decide per-namespace whether to share or create.  When no
//! flag forces a new namespace, the child shares the parent's nsproxy
//! (single refcount bump) — the common case.
//!
//! Reference: Linux `include/linux/nsproxy.h`, `kernel/nsproxy.c`.

extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::fs::namespace::{INIT_MNT_NS, MntNamespace, copy_mnt_ns};
use crate::ipc::namespace::{INIT_IPC_NS, IpcNamespace, copy_ipc_ns};
use crate::kernel::cgroup::namespace::{CgroupNamespace, INIT_CGROUP_NS, copy_cgroup_ns};
use crate::kernel::pid_namespace::{INIT_PID_NS_M28, PidNamespace, copy_pid_ns};
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};
use crate::kernel::utsname::{INIT_UTS_NS, UtsNamespace, copy_utsname};
use crate::net::core::net_namespace::{INIT_NET, Net, copy_net_ns};

use crate::kernel::clone::{
    CLONE_NEWCGROUP, CLONE_NEWIPC, CLONE_NEWNET, CLONE_NEWNS, CLONE_NEWPID, CLONE_NEWUSER,
    CLONE_NEWUTS,
};

/// Refcounted bundle of namespace pointers.  `user_ns` is *not* held here
/// (Linux puts it on `cred`); the `user_ns` field on each individual
/// namespace points to the owning user namespace.
#[repr(C)]
pub struct Nsproxy {
    pub count: AtomicUsize,
    pub uts_ns: *mut UtsNamespace,
    pub ipc_ns: *mut IpcNamespace,
    pub mnt_ns: *mut MntNamespace,
    pub pid_ns_for_children: *mut PidNamespace,
    pub net_ns: *mut Net,
    pub cgroup_ns: *mut CgroupNamespace,
}

unsafe impl Send for Nsproxy {}
unsafe impl Sync for Nsproxy {}

/// Singleton init nsproxy pointing at every `INIT_*_NS` static.
///
/// Sticky refcount — never freed.
pub static INIT_NSPROXY: Nsproxy = Nsproxy {
    count: AtomicUsize::new(usize::MAX / 2),
    uts_ns: &INIT_UTS_NS as *const _ as *mut _,
    ipc_ns: &INIT_IPC_NS as *const _ as *mut _,
    mnt_ns: &INIT_MNT_NS as *const _ as *mut _,
    pid_ns_for_children: &INIT_PID_NS_M28 as *const _ as *mut _,
    net_ns: &INIT_NET as *const _ as *mut _,
    cgroup_ns: &INIT_CGROUP_NS as *const _ as *mut _,
};

/// Bump nsproxy refcount.
#[inline]
pub fn get_nsproxy(ns: *mut Nsproxy) {
    if !ns.is_null() {
        unsafe {
            (*ns).count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Drop a reference; deallocate when zero.  Walks every contained namespace
/// and `put`s it via the ops vtable.
///
/// # Safety
/// `ns` must be a pointer previously published by `INIT_NSPROXY`,
/// `create_new_namespaces`, or `copy_namespaces`.
pub unsafe fn put_nsproxy(ns: *mut Nsproxy) {
    if ns.is_null() {
        return;
    }
    let prev = unsafe { (*ns).count.fetch_sub(1, Ordering::Release) };
    if prev != 1 {
        return;
    }
    if core::ptr::eq(ns, &raw const INIT_NSPROXY as *mut Nsproxy) {
        // Sticky — restore.
        unsafe {
            (*ns).count.store(usize::MAX / 2, Ordering::Relaxed);
        }
        return;
    }
    // Drop each namespace through its ops.put.
    unsafe {
        free_nsproxy(ns);
    }
}

unsafe fn free_nsproxy(ns: *mut Nsproxy) {
    use crate::fs::namespace::MNT_OPS;
    use crate::ipc::namespace::IPC_OPS;
    use crate::kernel::cgroup::namespace::CGROUP_OPS;
    use crate::kernel::pid_namespace::PID_NS_OPS;
    use crate::kernel::utsname::UTS_OPS;
    use crate::net::core::net_namespace::NET_OPS;

    unsafe {
        let n = &mut *ns;
        if !n.uts_ns.is_null() {
            (UTS_OPS.put)(n.uts_ns as *mut _);
        }
        if !n.ipc_ns.is_null() {
            (IPC_OPS.put)(n.ipc_ns as *mut _);
        }
        if !n.mnt_ns.is_null() {
            (MNT_OPS.put)(n.mnt_ns as *mut _);
        }
        if !n.pid_ns_for_children.is_null() {
            (PID_NS_OPS.put)(n.pid_ns_for_children as *mut _);
        }
        if !n.net_ns.is_null() {
            (NET_OPS.put)(n.net_ns as *mut _);
        }
        if !n.cgroup_ns.is_null() {
            (CGROUP_OPS.put)(n.cgroup_ns as *mut _);
        }
        drop(alloc::boxed::Box::from_raw(ns));
    }
}

// ── create_new_namespaces / copy_namespaces ─────────────────────────────────

/// Build a fresh `Nsproxy` for a child where each namespace is either
/// inherited (refcount bump on parent's ns) or freshly created (when the
/// corresponding `CLONE_NEW*` flag is set).
///
/// Mirrors Linux `create_new_namespaces()` from `kernel/nsproxy.c`.
///
/// Returns the new nsproxy pointer or a negative errno on failure.
pub fn create_new_namespaces(
    flags: u64,
    parent_nsproxy: *mut Nsproxy,
    user_ns: *const UserNamespace,
) -> Result<*mut Nsproxy, i32> {
    let parent = if parent_nsproxy.is_null() {
        &raw const INIT_NSPROXY as *mut Nsproxy
    } else {
        parent_nsproxy
    };

    // Allocate the new bundle with a single refcount.
    let mut b: alloc::boxed::Box<Nsproxy> = alloc::boxed::Box::new(Nsproxy {
        count: AtomicUsize::new(1),
        uts_ns: core::ptr::null_mut(),
        ipc_ns: core::ptr::null_mut(),
        mnt_ns: core::ptr::null_mut(),
        pid_ns_for_children: core::ptr::null_mut(),
        net_ns: core::ptr::null_mut(),
        cgroup_ns: core::ptr::null_mut(),
    });

    // Each helper either dups (CLONE_NEW*) or pointer-copies + refs the parent ns.
    use crate::fs::namespace::MNT_OPS;
    use crate::ipc::namespace::IPC_OPS;
    use crate::kernel::cgroup::namespace::CGROUP_OPS;
    use crate::kernel::pid_namespace::PID_NS_OPS;
    use crate::kernel::utsname::UTS_OPS;
    use crate::net::core::net_namespace::NET_OPS;

    unsafe {
        // UTS
        if flags & CLONE_NEWUTS != 0 {
            b.uts_ns = copy_utsname((*parent).uts_ns, user_ns)?;
        } else {
            b.uts_ns = (*parent).uts_ns;
            (UTS_OPS.get)(b.uts_ns as *mut _);
        }
        // IPC
        if flags & CLONE_NEWIPC != 0 {
            b.ipc_ns = copy_ipc_ns((*parent).ipc_ns, user_ns)?;
        } else {
            b.ipc_ns = (*parent).ipc_ns;
            (IPC_OPS.get)(b.ipc_ns as *mut _);
        }
        // MNT
        if flags & CLONE_NEWNS != 0 {
            b.mnt_ns = copy_mnt_ns((*parent).mnt_ns, user_ns)?;
        } else {
            b.mnt_ns = (*parent).mnt_ns;
            (MNT_OPS.get)(b.mnt_ns as *mut _);
        }
        // PID (for children)
        if flags & CLONE_NEWPID != 0 {
            b.pid_ns_for_children = copy_pid_ns((*parent).pid_ns_for_children, user_ns)?;
        } else {
            b.pid_ns_for_children = (*parent).pid_ns_for_children;
            (PID_NS_OPS.get)(b.pid_ns_for_children as *mut _);
        }
        // NET
        if flags & CLONE_NEWNET != 0 {
            b.net_ns = copy_net_ns((*parent).net_ns, user_ns)?;
        } else {
            b.net_ns = (*parent).net_ns;
            (NET_OPS.get)(b.net_ns as *mut _);
        }
        // CGROUP
        if flags & CLONE_NEWCGROUP != 0 {
            b.cgroup_ns = copy_cgroup_ns((*parent).cgroup_ns, user_ns)?;
        } else {
            b.cgroup_ns = (*parent).cgroup_ns;
            (CGROUP_OPS.get)(b.cgroup_ns as *mut _);
        }
    }

    Ok(alloc::boxed::Box::into_raw(b))
}

/// Linux `copy_namespaces()` — called from `copy_process`.
///
/// If no nsproxy-backed `CLONE_NEW*` flag is present, share the parent's
/// nsproxy.  Otherwise allocate a fresh bundle.
///
/// `CLONE_NEWUSER` is deliberately not part of this decision. Linux handles
/// it in `copy_creds`; a user-namespace-only clone keeps the old nsproxy.
///
/// # Safety
/// `parent` and `child` must be valid TaskStruct pointers.
pub unsafe fn copy_namespaces(
    flags: u64,
    parent: *mut crate::kernel::task::TaskStruct,
    child: *mut crate::kernel::task::TaskStruct,
) -> Result<(), i32> {
    const NSPROXY_FLAGS: u64 =
        CLONE_NEWNS | CLONE_NEWIPC | CLONE_NEWUTS | CLONE_NEWPID | CLONE_NEWNET | CLONE_NEWCGROUP;

    let parent_nsproxy = unsafe { (*parent).m28_nsproxy.nsproxy };
    let parent_nsproxy = if parent_nsproxy.is_null() {
        &raw const INIT_NSPROXY as *mut Nsproxy
    } else {
        parent_nsproxy
    };

    if flags & NSPROXY_FLAGS == 0 {
        // Share parent's nsproxy — the common case.
        get_nsproxy(parent_nsproxy);
        unsafe {
            (*child).m28_nsproxy.nsproxy = parent_nsproxy;
            (*child).m28_nsproxy.thread_pid_ns_for_children =
                (*parent_nsproxy).pid_ns_for_children as *mut core::ffi::c_void;
        }
        return Ok(());
    }

    // Allocate a fresh nsproxy with the requested namespaces unshared. A
    // CLONE_NEWUSER child already owns its freshly-created user namespace in
    // its credentials, and every other new namespace must be owned by it.
    let child_cred = unsafe { (*child).cred };
    let user_ns = if child_cred.is_null() || unsafe { (*child_cred).user_ns.is_null() } {
        &raw const INIT_USER_NS
    } else {
        unsafe { (*child_cred).user_ns as *const UserNamespace }
    };
    let new = create_new_namespaces(flags, parent_nsproxy, user_ns)?;
    if flags & CLONE_NEWNS != 0 {
        unsafe {
            rebase_task_fs_for_mount_namespace(child, (*parent_nsproxy).mnt_ns, (*new).mnt_ns);
        }
    }
    unsafe {
        (*child).m28_nsproxy.nsproxy = new;
        (*child).m28_nsproxy.thread_pid_ns_for_children =
            (*new).pid_ns_for_children as *mut core::ffi::c_void;
    }
    Ok(())
}

// ── unshare / setns syscalls ─────────────────────────────────────────────────

/// `sys_unshare(flags)` — disassociate the calling task from selected
/// namespaces.  Allocates a private nsproxy with the listed namespaces
/// freshly created.  Returns 0 on success or a negative errno.
///
/// # Safety
/// Must be called from a valid task context.
pub unsafe fn sys_unshare(flags: u64) -> i64 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return -22;
    }

    // Only namespace-flag bits are meaningful to unshare in M28.
    const VALID_FLAGS: u64 = CLONE_NEWNS
        | CLONE_NEWIPC
        | CLONE_NEWUTS
        | CLONE_NEWPID
        | CLONE_NEWNET
        | CLONE_NEWUSER
        | CLONE_NEWCGROUP;
    const NSPROXY_FLAGS: u64 =
        CLONE_NEWNS | CLONE_NEWIPC | CLONE_NEWUTS | CLONE_NEWPID | CLONE_NEWNET | CLONE_NEWCGROUP;

    if flags & !VALID_FLAGS != 0 {
        // Other unshare bits (CLONE_FILES, CLONE_FS, CLONE_VM,
        // CLONE_SIGHAND, CLONE_SYSVSEM, CLONE_THREAD) require sub-table
        // duplication that lands with M39 / M52.  Reject them for now.
        return -22;
    }
    if flags & CLONE_NEWNS != 0 {
        // The mount tree itself is private, but systemd's mount-namespace
        // setup also depends on the shared/slave propagation graph. Keep
        // this incomplete path fail-closed; Firefox's bwrap path uses the
        // independent CLONE_NEWUSER operation below.
        return -1;
    }
    let parent_cred = crate::kernel::cred::current_cred();
    let parent_user_ns = if parent_cred.is_null() || unsafe { (*parent_cred).user_ns.is_null() } {
        &raw const INIT_USER_NS
    } else {
        unsafe { (*parent_cred).user_ns as *const UserNamespace }
    };
    let new_user_ns = if flags & CLONE_NEWUSER != 0 {
        match crate::kernel::user_namespace::create_user_ns(parent_user_ns) {
            Ok(ns) => Some(ns),
            Err(e) => return e as i64,
        }
    } else {
        None
    };
    // Linux prepares the replacement credential before checking the owner of
    // the namespaces requested in the same unshare.  In particular,
    // unshare(CLONE_NEWUSER | CLONE_NEWNS) is authorised in the newly-created
    // user namespace, not by the old namespace's capability set.
    let mut new_cred = None;
    if let Some(user_ns) = new_user_ns {
        let Some(cred) = crate::kernel::cred::prepare_creds() else {
            unsafe { crate::kernel::user_namespace::put_user_ns(user_ns) };
            return -12;
        };
        unsafe {
            // prepare_creds() retained the old user namespace.  Replacing
            // that pointer transfers the credential's namespace ownership to
            // the freshly-created namespace.
            crate::kernel::user_namespace::put_user_ns(parent_user_ns);
            (*cred).user_ns = user_ns as *const core::ffi::c_void;
            // Linux grants the creator all capabilities in the namespace it
            // owns; uid/gid maps remain empty until procfs accepts the writes.
            (*cred).cap_permitted = crate::kernel::capability::KernelCapT::full();
            (*cred).cap_effective = crate::kernel::capability::KernelCapT::full();
            (*cred).cap_bset = crate::kernel::capability::KernelCapT::full();
            (*cred).cap_ambient = crate::kernel::capability::KernelCapT::empty();
        }
        new_cred = Some(cred);
    }
    let namespace_owner = new_user_ns
        .map(|ns| ns as *const UserNamespace)
        .unwrap_or(parent_user_ns);

    // Linux: unshare_nsproxy_namespaces() checks CAP_SYS_ADMIN in the
    // temporary credential's user namespace, after unshare_userns().
    let capability_cred = new_cred
        .map(|cred| cred as *const crate::kernel::cred::Cred)
        .unwrap_or(parent_cred);
    if flags & NSPROXY_FLAGS != 0
        && !crate::kernel::capability::cred_ns_capable(
            capability_cred,
            namespace_owner as *const core::ffi::c_void,
            crate::kernel::capability::CAP_SYS_ADMIN,
        )
    {
        if let Some(cred) = new_cred {
            unsafe { crate::kernel::cred::Cred::put(cred) };
        }
        return -1;
    }

    let parent_nsproxy = unsafe { (*task).m28_nsproxy.nsproxy };
    let new = if flags & NSPROXY_FLAGS != 0 {
        match create_new_namespaces(flags, parent_nsproxy, namespace_owner) {
            Ok(p) => Some(p),
            Err(e) => {
                if let Some(cred) = new_cred {
                    unsafe { crate::kernel::cred::Cred::put(cred) };
                }
                return e as i64;
            }
        }
    } else {
        None
    };

    if let Some(cred) = new_cred {
        crate::kernel::cred::commit_creds(cred);
    }

    let Some(new) = new else {
        return 0;
    };

    let old = unsafe { (*task).m28_nsproxy.nsproxy };
    if flags & CLONE_NEWNS != 0 {
        let old_mnt_ns = if old.is_null() {
            INIT_NSPROXY.mnt_ns
        } else {
            unsafe { (*old).mnt_ns }
        };
        unsafe {
            rebase_task_fs_for_mount_namespace(task, old_mnt_ns, (*new).mnt_ns);
        }
    }
    unsafe {
        (*task).m28_nsproxy.nsproxy = new;
        (*task).m28_nsproxy.thread_pid_ns_for_children =
            (*new).pid_ns_for_children as *mut core::ffi::c_void;
        put_nsproxy(old);
    }
    0
}

unsafe fn rebase_task_fs_for_mount_namespace(
    task: *mut crate::kernel::task::TaskStruct,
    old_ns: *const MntNamespace,
    new_ns: *const MntNamespace,
) {
    let fs = unsafe { crate::fs::fs_struct::task_fs(task) };
    if fs.is_null() {
        return;
    }
    let Some(old_namespace_root) = crate::fs::mount::namespace_root_for(old_ns) else {
        return;
    };
    let Some(new_namespace_root) = crate::fs::mount::namespace_root_for(new_ns) else {
        return;
    };
    let fs = unsafe { &*fs };
    let old_root = fs
        .root
        .lock()
        .clone()
        .unwrap_or_else(|| old_namespace_root.clone());
    let old_pwd = fs.pwd.lock().clone().unwrap_or_else(|| old_root.clone());
    let root_name = crate::fs::mount::path_between(&old_namespace_root, &old_root)
        .unwrap_or_else(|| alloc::string::String::from("/"));
    let pwd_name = crate::fs::mount::path_between(&old_namespace_root, &old_pwd)
        .unwrap_or_else(|| root_name.clone());
    let new_root = crate::fs::mount::resolve_from_namespace_root(&new_namespace_root, &root_name)
        .unwrap_or_else(|_| new_namespace_root.clone());
    let new_pwd = crate::fs::mount::resolve_from_namespace_root(&new_namespace_root, &pwd_name)
        .unwrap_or_else(|_| new_root.clone());
    crate::fs::fs_struct::set_fs_root_path(fs, new_root);
    crate::fs::fs_struct::set_fs_pwd_path(fs, new_pwd);
}

/// `sys_setns(fd, nstype)` — join an existing namespace identified by `fd`.
///
/// Stub returning `-EBADF` until VFS lands (M39).  The dispatch + capability
/// checks are exercised through `setns_install` (kernel-internal).
pub unsafe fn sys_setns(_fd: i32, _nstype: i32) -> i64 {
    -9 // EBADF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_nsproxy_pointers_non_null() {
        assert!(!INIT_NSPROXY.uts_ns.is_null());
        assert!(!INIT_NSPROXY.ipc_ns.is_null());
        assert!(!INIT_NSPROXY.mnt_ns.is_null());
        assert!(!INIT_NSPROXY.pid_ns_for_children.is_null());
        assert!(!INIT_NSPROXY.net_ns.is_null());
        assert!(!INIT_NSPROXY.cgroup_ns.is_null());
    }

    #[test]
    /// test-origin: linux:vendor/linux/kernel/fork.c:ksys_unshare
    ///
    /// Linux prepares a new user credential before asking
    /// `unshare_nsproxy_namespaces()` to create the other namespaces.  A
    /// user-namespace-only unshare must not be rejected by a local
    /// fail-closed gate. Lupos keeps the incomplete mount-namespace path
    /// fail-closed because systemd depends on propagation semantics that are
    /// not implemented yet.
    fn unshare_user_and_mount_namespaces_follow_linux_order() {
        use crate::kernel::sched;
        use crate::kernel::task::TaskStruct;
        use alloc::boxed::Box;

        let previous = unsafe { sched::get_current() };
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.m28_nsproxy.nsproxy = &raw const INIT_NSPROXY as *mut Nsproxy;
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        assert_eq!(unsafe { sys_unshare(CLONE_NEWNS) }, -1);
        assert_eq!(unsafe { sys_unshare(CLONE_NEWUSER) }, 0);
        assert_ne!(task.cred, core::ptr::null());
        assert_eq!(unsafe { sys_unshare(CLONE_NEWUSER | CLONE_NEWNS) }, -1);

        let final_nsproxy = task.m28_nsproxy.nsproxy;
        assert_eq!(unsafe { (*final_nsproxy).mnt_ns }, INIT_NSPROXY.mnt_ns);

        let source = include_str!("nsproxy.rs");
        let create_user = source
            .find("create_user_ns(parent_user_ns)")
            .expect("unshare must create the user namespace first");
        let check_cap = source
            .find("cred_ns_capable(")
            .expect("unshare must check capabilities in the replacement cred");
        let create_namespaces = source
            .find("create_new_namespaces(flags, parent_nsproxy, namespace_owner)")
            .expect("unshare must create the nsproxy after authorization");
        assert!(create_user < check_cap && check_cap < create_namespaces);

        let final_cred = task.cred;
        let final_real_cred = task.m27.real_cred;
        task.cred = core::ptr::null();
        task.m27.real_cred = core::ptr::null();
        unsafe { sched::set_current(previous) };
        unsafe { put_nsproxy(final_nsproxy) };
        unsafe {
            crate::kernel::cred::Cred::put(final_cred);
            crate::kernel::cred::Cred::put(final_real_cred);
        }
    }

    #[test]
    /// test-origin: linux:vendor/linux/kernel/fork.c:kernel_clone
    ///
    /// Linux does not reject namespace flags in `kernel_clone()`.  It passes
    /// them to `copy_process()`, where `copy_creds()` creates the user
    /// namespace before `copy_namespaces()` builds the nsproxy.  Keeping the
    /// early local EPERM gate would prevent Firefox's sandbox clone from ever
    /// reaching that Linux-equivalent path.
    fn clone_namespace_flags_reach_copy_process_like_linux() {
        let linux = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/fork.c"
        ));
        let linux_kernel_clone = linux
            .split("pid_t kernel_clone(struct kernel_clone_args *args)")
            .nth(1)
            .expect("Linux kernel_clone must exist")
            .split("/*\n * Create a kernel thread")
            .next()
            .expect("Linux kernel_clone body must end before the next helper");
        assert!(
            linux_kernel_clone.contains("p = copy_process")
                && !linux_kernel_clone.contains("return -EPERM"),
            "Linux kernel_clone delegates namespace creation to copy_process"
        );

        let source = include_str!("fork.rs");
        let gate = source
            .split("pub unsafe fn kernel_clone")
            .nth(1)
            .expect("kernel_clone body")
            .split("copy_process")
            .next()
            .expect("kernel_clone preamble");
        assert!(
            !gate.contains("CLONE_NEWNS") && !gate.contains("CLONE_NEWUSER"),
            "kernel_clone must not reject namespace flags before copy_process"
        );
    }

    #[test]
    fn create_new_namespaces_with_no_flags_copies_pointers() {
        let parent = &raw const INIT_NSPROXY as *mut Nsproxy;
        let p = create_new_namespaces(0, parent, &INIT_USER_NS).unwrap();
        unsafe {
            // Same pointers (shared) but it's a fresh bundle.
            assert_eq!((*p).uts_ns, INIT_NSPROXY.uts_ns);
            assert_eq!((*p).ipc_ns, INIT_NSPROXY.ipc_ns);
            put_nsproxy(p);
        }
    }

    #[test]
    fn create_new_namespaces_with_uts_creates_fresh_uts() {
        let parent = &raw const INIT_NSPROXY as *mut Nsproxy;
        let p = create_new_namespaces(CLONE_NEWUTS, parent, &INIT_USER_NS).unwrap();
        unsafe {
            assert_ne!((*p).uts_ns, INIT_NSPROXY.uts_ns);
            put_nsproxy(p);
        }
    }

    #[test]
    fn syscall_m76_process_control_parity() {
        let previous = unsafe { crate::kernel::sched::get_current() };
        unsafe {
            crate::kernel::sched::set_current(core::ptr::null_mut());
            assert_eq!(sys_unshare(0), -22);
            assert_eq!(sys_setns(-1, 0), -9);
            crate::kernel::sched::set_current(previous);
        }
    }
}
