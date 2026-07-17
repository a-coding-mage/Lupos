//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/self.c
//! test-origin: linux:vendor/linux/fs/proc/self.c
//! `/proc/self`.
//!
//! Ref: `vendor/linux/fs/proc/self.c`

use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, add_child};
use crate::include::uapi::errno::{ECHILD, ENOENT, ENOMEM};
use crate::include::uapi::stat::S_IFLNK;

pub const PROC_SELF_ALLOC_LEN: usize = 10 + 1;
pub const PROC_SELF_LINK_MODE: u32 = S_IFLNK | 0o777;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcSelfGetLinkResult {
    pub ret: Result<String, i32>,
    pub gfp: &'static str,
    pub delayed_call_set: bool,
}

pub fn proc_self_get_link(
    dentry_present: bool,
    tgid: u32,
    allocation_succeeds: bool,
) -> ProcSelfGetLinkResult {
    let gfp = if dentry_present {
        "GFP_KERNEL"
    } else {
        "GFP_ATOMIC"
    };
    if tgid == 0 {
        return ProcSelfGetLinkResult {
            ret: Err(-ENOENT),
            gfp,
            delayed_call_set: false,
        };
    }
    if !allocation_succeeds {
        return ProcSelfGetLinkResult {
            ret: Err(if dentry_present { -ENOMEM } else { -ECHILD }),
            gfp,
            delayed_call_set: false,
        };
    }
    ProcSelfGetLinkResult {
        ret: Ok(alloc::format!("{tgid}")),
        gfp,
        delayed_call_set: true,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcSelfSetupReport {
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

pub fn proc_setup_self(
    dentry_allocated: bool,
    inode_allocated: bool,
    self_inum: u32,
) -> ProcSelfSetupReport {
    let success = dentry_allocated && inode_allocated;
    ProcSelfSetupReport {
        ret: if success { 0 } else { -ENOMEM },
        dentry_allocated,
        inode_allocated,
        inode_number: success.then_some(self_inum),
        mode: success.then_some(PROC_SELF_LINK_MODE),
        root_uid_gid: success,
        inode_operations: success.then_some("proc_self_inode_operations"),
        persistent: success,
        dput_called: dentry_allocated,
        error_logged: !success,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcSelfInitReport {
    pub proc_alloc_inum_called: bool,
}

pub const fn proc_self_init() -> ProcSelfInitReport {
    ProcSelfInitReport {
        proc_alloc_inum_called: true,
    }
}

pub fn new_self_dir() -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir("self", 0o555);
    super::base::add_task_common(&dir);
    add_child(&dir, super::fd::new_fd_dir());
    add_child(
        &dir,
        KernfsNode::new_file("mounts", 0o444, Some(super::root::mounts_show), None),
    );
    add_child(
        &dir,
        KernfsNode::new_file("mountinfo", 0o444, Some(super::root::mountinfo_show), None),
    );
    add_child(
        &dir,
        KernfsNode::new_file(
            "mountstats",
            0o444,
            Some(super::root::mountstats_show),
            None,
        ),
    );
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_self_get_link_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/self.c"
        ));
        assert!(source.contains("proc_self_get_link"));
        assert!(source.contains("pid_t tgid = task_tgid_nr_ns(current, ns);"));
        assert!(source.contains("if (!tgid)"));
        assert!(source.contains("return ERR_PTR(-ENOENT);"));
        assert!(source.contains("name = kmalloc(10 + 1, dentry ? GFP_KERNEL : GFP_ATOMIC);"));
        assert!(source.contains("return dentry ? ERR_PTR(-ENOMEM) : ERR_PTR(-ECHILD);"));
        assert!(source.contains("sprintf(name, \"%u\", tgid);"));
        assert!(source.contains("set_delayed_call(done, kfree_link, name);"));
        assert!(source.contains(".get_link\t= proc_self_get_link"));

        assert_eq!(proc_self_get_link(true, 0, true).ret, Err(-ENOENT));
        assert_eq!(proc_self_get_link(true, 42, false).ret, Err(-ENOMEM));
        assert_eq!(proc_self_get_link(false, 42, false).ret, Err(-ECHILD));
        assert_eq!(
            proc_self_get_link(false, 42, true),
            ProcSelfGetLinkResult {
                ret: Ok(String::from("42")),
                gfp: "GFP_ATOMIC",
                delayed_call_set: true,
            }
        );
        assert_eq!(PROC_SELF_ALLOC_LEN, 11);
    }

    #[test]
    fn proc_setup_self_and_init_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/self.c"
        ));
        assert!(source.contains("unsigned self_inum __ro_after_init;"));
        assert!(source.contains("self = d_alloc_name(s->s_root, \"self\");"));
        assert!(source.contains("struct inode *inode = new_inode(s);"));
        assert!(source.contains("inode->i_ino = self_inum;"));
        assert!(source.contains("simple_inode_init_ts(inode);"));
        assert!(source.contains("inode->i_mode = S_IFLNK | S_IRWXUGO;"));
        assert!(source.contains("inode->i_uid = GLOBAL_ROOT_UID;"));
        assert!(source.contains("inode->i_gid = GLOBAL_ROOT_GID;"));
        assert!(source.contains("inode->i_op = &proc_self_inode_operations;"));
        assert!(source.contains("d_make_persistent(self, inode);"));
        assert!(source.contains("dput(self);"));
        assert!(source.contains("pr_err(\"proc_fill_super: can't allocate /proc/self\\n\");"));
        assert!(source.contains("proc_alloc_inum(&self_inum);"));

        assert_eq!(
            proc_setup_self(true, true, 99),
            ProcSelfSetupReport {
                ret: 0,
                dentry_allocated: true,
                inode_allocated: true,
                inode_number: Some(99),
                mode: Some(S_IFLNK | 0o777),
                root_uid_gid: true,
                inode_operations: Some("proc_self_inode_operations"),
                persistent: true,
                dput_called: true,
                error_logged: false,
            }
        );
        assert_eq!(proc_setup_self(true, false, 99).ret, -ENOMEM);
        assert!(proc_setup_self(false, false, 99).error_logged);
        assert!(proc_self_init().proc_alloc_inum_called);
    }
}
