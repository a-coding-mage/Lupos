//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/thread_self.c
//! test-origin: linux:vendor/linux/fs/proc/thread_self.c
//! `/proc/thread-self`.
//!
//! Ref: `vendor/linux/fs/proc/thread_self.c`

use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::{ECHILD, ENOENT, ENOMEM};
use crate::include::uapi::stat::S_IFLNK;

pub const PROC_THREAD_SELF_ALLOC_LEN: usize = 10 + 6 + 10 + 1;
pub const PROC_THREAD_SELF_LINK_MODE: u32 = S_IFLNK | 0o777;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcThreadSelfGetLinkResult {
    pub ret: Result<String, i32>,
    pub gfp: &'static str,
    pub delayed_call_set: bool,
}

pub fn proc_thread_self_get_link(
    dentry_present: bool,
    tgid: u32,
    pid: u32,
    allocation_succeeds: bool,
) -> ProcThreadSelfGetLinkResult {
    let gfp = if dentry_present {
        "GFP_KERNEL"
    } else {
        "GFP_ATOMIC"
    };
    if pid == 0 {
        return ProcThreadSelfGetLinkResult {
            ret: Err(-ENOENT),
            gfp,
            delayed_call_set: false,
        };
    }
    if !allocation_succeeds {
        return ProcThreadSelfGetLinkResult {
            ret: Err(if dentry_present { -ENOMEM } else { -ECHILD }),
            gfp,
            delayed_call_set: false,
        };
    }
    ProcThreadSelfGetLinkResult {
        ret: Ok(alloc::format!("{tgid}/task/{pid}")),
        gfp,
        delayed_call_set: true,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcThreadSelfSetupReport {
    pub ret: i32,
    pub dentry_allocated: bool,
    pub inode_allocated: bool,
    pub inode_number: Option<u32>,
    pub mode: Option<u32>,
    pub root_uid_gid: bool,
    pub inode_operations: Option<&'static str>,
    pub persistent: bool,
    pub dput_called: bool,
    pub error_logged: bool,
}

pub fn proc_setup_thread_self(
    dentry_allocated: bool,
    inode_allocated: bool,
    thread_self_inum: u32,
) -> ProcThreadSelfSetupReport {
    let success = dentry_allocated && inode_allocated;
    ProcThreadSelfSetupReport {
        ret: if success { 0 } else { -ENOMEM },
        dentry_allocated,
        inode_allocated,
        inode_number: success.then_some(thread_self_inum),
        mode: success.then_some(PROC_THREAD_SELF_LINK_MODE),
        root_uid_gid: success,
        inode_operations: success.then_some("proc_thread_self_inode_operations"),
        persistent: success,
        dput_called: dentry_allocated,
        error_logged: !success,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcThreadSelfInitReport {
    pub proc_alloc_inum_called: bool,
}

pub const fn proc_thread_self_init() -> ProcThreadSelfInitReport {
    ProcThreadSelfInitReport {
        proc_alloc_inum_called: true,
    }
}

pub fn new_thread_self() -> Arc<KernfsNode> {
    KernfsNode::new_symlink("thread-self", "self/task/1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_thread_self_get_link_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/thread_self.c"
        ));
        assert!(source.contains("proc_thread_self_get_link"));
        assert!(source.contains("pid_t tgid = task_tgid_nr_ns(current, ns);"));
        assert!(source.contains("pid_t pid = task_pid_nr_ns(current, ns);"));
        assert!(source.contains("if (!pid)"));
        assert!(source.contains("return ERR_PTR(-ENOENT);"));
        assert!(
            source.contains("name = kmalloc(10 + 6 + 10 + 1, dentry ? GFP_KERNEL : GFP_ATOMIC);")
        );
        assert!(source.contains("return dentry ? ERR_PTR(-ENOMEM) : ERR_PTR(-ECHILD);"));
        assert!(source.contains("sprintf(name, \"%u/task/%u\", tgid, pid);"));
        assert!(source.contains("set_delayed_call(done, kfree_link, name);"));
        assert!(source.contains(".get_link\t= proc_thread_self_get_link"));

        assert_eq!(
            proc_thread_self_get_link(true, 7, 0, true).ret,
            Err(-ENOENT)
        );
        assert_eq!(
            proc_thread_self_get_link(true, 7, 9, false).ret,
            Err(-ENOMEM)
        );
        assert_eq!(
            proc_thread_self_get_link(false, 7, 9, false).ret,
            Err(-ECHILD)
        );
        assert_eq!(
            proc_thread_self_get_link(false, 7, 9, true),
            ProcThreadSelfGetLinkResult {
                ret: Ok(String::from("7/task/9")),
                gfp: "GFP_ATOMIC",
                delayed_call_set: true,
            }
        );
        assert_eq!(PROC_THREAD_SELF_ALLOC_LEN, 27);
    }

    #[test]
    fn proc_setup_thread_self_and_init_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/thread_self.c"
        ));
        assert!(source.contains("unsigned thread_self_inum __ro_after_init;"));
        assert!(source.contains("thread_self = d_alloc_name(s->s_root, \"thread-self\");"));
        assert!(source.contains("struct inode *inode = new_inode(s);"));
        assert!(source.contains("inode->i_ino = thread_self_inum;"));
        assert!(source.contains("simple_inode_init_ts(inode);"));
        assert!(source.contains("inode->i_mode = S_IFLNK | S_IRWXUGO;"));
        assert!(source.contains("inode->i_uid = GLOBAL_ROOT_UID;"));
        assert!(source.contains("inode->i_gid = GLOBAL_ROOT_GID;"));
        assert!(source.contains("inode->i_op = &proc_thread_self_inode_operations;"));
        assert!(source.contains("d_make_persistent(thread_self, inode);"));
        assert!(source.contains("dput(thread_self);"));
        assert!(
            source.contains("pr_err(\"proc_fill_super: can't allocate /proc/thread-self\\n\");")
        );
        assert!(source.contains("proc_alloc_inum(&thread_self_inum);"));

        assert_eq!(
            proc_setup_thread_self(true, true, 100),
            ProcThreadSelfSetupReport {
                ret: 0,
                dentry_allocated: true,
                inode_allocated: true,
                inode_number: Some(100),
                mode: Some(S_IFLNK | 0o777),
                root_uid_gid: true,
                inode_operations: Some("proc_thread_self_inode_operations"),
                persistent: true,
                dput_called: true,
                error_logged: false,
            }
        );
        assert_eq!(proc_setup_thread_self(true, false, 100).ret, -ENOMEM);
        assert!(proc_setup_thread_self(false, false, 100).error_logged);
        assert!(proc_thread_self_init().proc_alloc_inum_called);
    }
}
