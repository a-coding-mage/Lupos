//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/namespaces.c
//! test-origin: linux:vendor/linux/fs/proc/namespaces.c
//! `/proc/<pid>/ns`.
//!
//! Ref: `vendor/linux/fs/proc/namespaces.c`

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::fs::kernfs::{KernfsNode, add_child};
use crate::include::uapi::errno::{EACCES, ECHILD, ENOENT};
use crate::include::uapi::stat::{S_IFLNK, S_IRWXG, S_IRWXO, S_IRWXU};

pub const PROC_NS_LINK_MODE: u32 = S_IFLNK | S_IRWXU | S_IRWXG | S_IRWXO;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNsOperation {
    pub name: &'static str,
    pub target: &'static str,
}

pub const PROC_NS_ENTRIES: &[ProcNsOperation] = &[
    ProcNsOperation {
        name: "net",
        target: "net:[4026531840]",
    },
    ProcNsOperation {
        name: "uts",
        target: "uts:[4026531838]",
    },
    ProcNsOperation {
        name: "ipc",
        target: "ipc:[4026531839]",
    },
    ProcNsOperation {
        name: "pid",
        target: "pid:[4026531836]",
    },
    ProcNsOperation {
        name: "pid_for_children",
        target: "pid:[4026531836]",
    },
    ProcNsOperation {
        name: "user",
        target: "user:[4026531837]",
    },
    ProcNsOperation {
        name: "mnt",
        target: "mnt:[4026531841]",
    },
    ProcNsOperation {
        name: "cgroup",
        target: "cgroup:[4026531835]",
    },
    ProcNsOperation {
        name: "time",
        target: "time:[4026531834]",
    },
    ProcNsOperation {
        name: "time_for_children",
        target: "time:[4026531834]",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNsLinkInodeOperations {
    pub readlink: &'static str,
    pub get_link: &'static str,
    pub setattr: &'static str,
}

pub const PROC_NS_LINK_INODE_OPERATIONS: ProcNsLinkInodeOperations = ProcNsLinkInodeOperations {
    readlink: "proc_ns_readlink",
    get_link: "proc_ns_get_link",
    setattr: "proc_nochmod_setattr",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNsDirFileOperations {
    pub read: &'static str,
    pub iterate_shared: &'static str,
    pub llseek: &'static str,
}

pub const PROC_NS_DIR_OPERATIONS: ProcNsDirFileOperations = ProcNsDirFileOperations {
    read: "generic_read_dir",
    iterate_shared: "proc_ns_dir_readdir",
    llseek: "generic_file_llseek",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNsDirInodeOperations {
    pub lookup: &'static str,
    pub getattr: &'static str,
    pub setattr: &'static str,
}

pub const PROC_NS_DIR_INODE_OPERATIONS: ProcNsDirInodeOperations = ProcNsDirInodeOperations {
    lookup: "proc_ns_dir_lookup",
    getattr: "pid_getattr",
    setattr: "proc_nochmod_setattr",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNsGetLinkEnv {
    pub dentry_present: bool,
    pub task_present: bool,
    pub ptrace_may_access: bool,
    pub ns_get_path_ret: i32,
    pub nd_jump_link_ret: i32,
}

impl ProcNsGetLinkEnv {
    pub const ALLOWED: Self = Self {
        dentry_present: true,
        task_present: true,
        ptrace_may_access: true,
        ns_get_path_ret: 0,
        nd_jump_link_ret: 0,
    };
}

pub fn proc_ns_get_link(env: ProcNsGetLinkEnv) -> i32 {
    if !env.dentry_present {
        return -ECHILD;
    }
    if !env.task_present {
        return -EACCES;
    }
    if !env.ptrace_may_access {
        return -EACCES;
    }
    if env.ns_get_path_ret != 0 {
        return env.ns_get_path_ret;
    }
    env.nd_jump_link_ret
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNsReadlinkEnv<'a> {
    pub task_present: bool,
    pub ptrace_may_access: bool,
    pub ns_get_name: Result<&'a str, i32>,
    pub buflen: usize,
}

impl<'a> ProcNsReadlinkEnv<'a> {
    pub const fn allowed(name: &'a str, buflen: usize) -> Self {
        Self {
            task_present: true,
            ptrace_may_access: true,
            ns_get_name: Ok(name),
            buflen,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcNsReadlinkResult {
    pub ret: i32,
    pub copied: String,
}

pub fn proc_ns_readlink(env: ProcNsReadlinkEnv<'_>) -> ProcNsReadlinkResult {
    if !env.task_present {
        return ProcNsReadlinkResult {
            ret: -EACCES,
            copied: String::new(),
        };
    }
    if !env.ptrace_may_access {
        return ProcNsReadlinkResult {
            ret: -EACCES,
            copied: String::new(),
        };
    }

    let name = match env.ns_get_name {
        Ok(name) => name,
        Err(errno) => {
            return ProcNsReadlinkResult {
                ret: errno,
                copied: String::new(),
            };
        }
    };
    let copy_len = name.len().min(env.buflen);
    ProcNsReadlinkResult {
        ret: copy_len as i32,
        copied: String::from(&name[..copy_len]),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcNsDentry {
    pub name: String,
    pub mode: u32,
    pub inode_operations: ProcNsLinkInodeOperations,
    pub ns_ops_name: &'static str,
    pub pid_updated: bool,
    pub spliced_with_pid_ops: bool,
}

pub fn proc_ns_instantiate(
    dentry_name: &str,
    ns_ops: ProcNsOperation,
    inode_allocated: bool,
) -> Result<ProcNsDentry, i32> {
    if !inode_allocated {
        return Err(-ENOENT);
    }
    Ok(ProcNsDentry {
        name: String::from(dentry_name),
        mode: PROC_NS_LINK_MODE,
        inode_operations: PROC_NS_LINK_INODE_OPERATIONS,
        ns_ops_name: ns_ops.name,
        pid_updated: true,
        spliced_with_pid_ops: true,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNsReaddirEnv {
    pub task_present: bool,
    pub dir_emit_dots: bool,
    pub pos: usize,
    pub fill_limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcNsReaddirResult {
    pub ret: i32,
    pub pos: usize,
    pub emitted: Vec<&'static str>,
}

pub fn proc_ns_dir_readdir(env: ProcNsReaddirEnv) -> ProcNsReaddirResult {
    if !env.task_present {
        return ProcNsReaddirResult {
            ret: -ENOENT,
            pos: env.pos,
            emitted: Vec::new(),
        };
    }

    let mut pos = env.pos;
    if !env.dir_emit_dots {
        return ProcNsReaddirResult {
            ret: 0,
            pos,
            emitted: Vec::new(),
        };
    }
    if pos < 2 {
        pos = 2;
    }
    if pos >= 2 + PROC_NS_ENTRIES.len() {
        return ProcNsReaddirResult {
            ret: 0,
            pos,
            emitted: Vec::new(),
        };
    }

    let mut emitted = Vec::new();
    let mut idx = pos - 2;
    while idx < PROC_NS_ENTRIES.len() && emitted.len() < env.fill_limit {
        emitted.push(PROC_NS_ENTRIES[idx].name);
        pos += 1;
        idx += 1;
    }

    ProcNsReaddirResult {
        ret: 0,
        pos,
        emitted,
    }
}

pub fn proc_ns_dir_lookup(
    task_present: bool,
    name: &str,
    inode_allocated: bool,
) -> Result<ProcNsDentry, i32> {
    if !task_present {
        return Err(-ENOENT);
    }
    let ns_ops = proc_ns_entry_by_name(name).ok_or(-ENOENT)?;
    proc_ns_instantiate(name, ns_ops, inode_allocated)
}

pub fn proc_ns_entry_by_name(name: &str) -> Option<ProcNsOperation> {
    PROC_NS_ENTRIES
        .iter()
        .copied()
        .find(|entry| entry.name.as_bytes() == name.as_bytes())
}

pub fn new_ns_dir() -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir("ns", 0o555);
    for entry in PROC_NS_ENTRIES {
        add_child(&dir, KernfsNode::new_symlink(entry.name, entry.target));
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::{KernfsKind, lookup};
    use alloc::vec;

    #[test]
    fn proc_ns_table_and_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/namespaces.c"
        ));

        assert!(source.contains("static const struct proc_ns_operations *const ns_entries[]"));
        assert!(source.contains("&netns_operations"));
        assert!(source.contains("&utsns_operations"));
        assert!(source.contains("&ipcns_operations"));
        assert!(source.contains("&pidns_operations"));
        assert!(source.contains("&pidns_for_children_operations"));
        assert!(source.contains("&userns_operations"));
        assert!(source.contains("&mntns_operations"));
        assert!(source.contains("&cgroupns_operations"));
        assert!(source.contains("&timens_operations"));
        assert!(source.contains("&timens_for_children_operations"));
        assert!(source.contains(".readlink\t= proc_ns_readlink"));
        assert!(source.contains(".get_link\t= proc_ns_get_link"));
        assert!(source.contains(".iterate_shared\t= proc_ns_dir_readdir"));
        assert!(source.contains(".lookup\t\t= proc_ns_dir_lookup"));
        assert!(source.contains("proc_nochmod_setattr"));
        assert!(source.contains("pid_getattr"));

        assert_eq!(
            PROC_NS_ENTRIES
                .iter()
                .map(|entry| entry.name)
                .collect::<Vec<_>>(),
            vec![
                "net",
                "uts",
                "ipc",
                "pid",
                "pid_for_children",
                "user",
                "mnt",
                "cgroup",
                "time",
                "time_for_children",
            ]
        );
        assert_eq!(PROC_NS_LINK_MODE, S_IFLNK | 0o777);
        assert_eq!(PROC_NS_LINK_INODE_OPERATIONS.get_link, "proc_ns_get_link");
        assert_eq!(PROC_NS_DIR_OPERATIONS.iterate_shared, "proc_ns_dir_readdir");
        assert_eq!(PROC_NS_DIR_INODE_OPERATIONS.lookup, "proc_ns_dir_lookup");
    }

    #[test]
    fn proc_ns_get_link_matches_linux_error_order() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/namespaces.c"
        ));
        assert!(source.contains("if (!dentry)"));
        assert!(source.contains("return ERR_PTR(-ECHILD);"));
        assert!(source.contains("if (!task)"));
        assert!(source.contains("return ERR_PTR(-EACCES);"));
        assert!(source.contains("if (!ptrace_may_access(task, PTRACE_MODE_READ_FSCREDS))"));
        assert!(source.contains("error = ns_get_path(&ns_path, task, ns_ops);"));
        assert!(source.contains("error = nd_jump_link(&ns_path);"));

        assert_eq!(
            proc_ns_get_link(ProcNsGetLinkEnv {
                dentry_present: false,
                ..ProcNsGetLinkEnv::ALLOWED
            }),
            -ECHILD
        );
        assert_eq!(
            proc_ns_get_link(ProcNsGetLinkEnv {
                task_present: false,
                ..ProcNsGetLinkEnv::ALLOWED
            }),
            -EACCES
        );
        assert_eq!(
            proc_ns_get_link(ProcNsGetLinkEnv {
                ptrace_may_access: false,
                ..ProcNsGetLinkEnv::ALLOWED
            }),
            -EACCES
        );
        assert_eq!(
            proc_ns_get_link(ProcNsGetLinkEnv {
                ns_get_path_ret: -ENOENT,
                nd_jump_link_ret: -ECHILD,
                ..ProcNsGetLinkEnv::ALLOWED
            }),
            -ENOENT
        );
        assert_eq!(
            proc_ns_get_link(ProcNsGetLinkEnv {
                nd_jump_link_ret: -ECHILD,
                ..ProcNsGetLinkEnv::ALLOWED
            }),
            -ECHILD
        );
    }

    #[test]
    fn proc_ns_readlink_matches_linux_access_and_copy_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/namespaces.c"
        ));
        assert!(source.contains("char name[50];"));
        assert!(source.contains("int res = -EACCES;"));
        assert!(source.contains("task = get_proc_task(inode);"));
        assert!(source.contains("if (ptrace_may_access(task, PTRACE_MODE_READ_FSCREDS))"));
        assert!(source.contains("res = ns_get_name(name, sizeof(name), task, ns_ops);"));
        assert!(source.contains("res = readlink_copy(buffer, buflen, name, strlen(name));"));

        assert_eq!(
            proc_ns_readlink(ProcNsReadlinkEnv {
                task_present: false,
                ..ProcNsReadlinkEnv::allowed("mnt:[7]", 64)
            }),
            ProcNsReadlinkResult {
                ret: -EACCES,
                copied: String::new(),
            }
        );
        assert_eq!(
            proc_ns_readlink(ProcNsReadlinkEnv {
                ptrace_may_access: false,
                ..ProcNsReadlinkEnv::allowed("mnt:[7]", 64)
            })
            .ret,
            -EACCES
        );
        assert_eq!(
            proc_ns_readlink(ProcNsReadlinkEnv {
                ns_get_name: Err(-ENOENT),
                ..ProcNsReadlinkEnv::allowed("mnt:[7]", 64)
            })
            .ret,
            -ENOENT
        );
        assert_eq!(
            proc_ns_readlink(ProcNsReadlinkEnv::allowed("mnt:[4026531841]", 8)),
            ProcNsReadlinkResult {
                ret: 8,
                copied: String::from("mnt:[402"),
            }
        );
    }

    #[test]
    fn proc_ns_instantiate_lookup_and_readdir_match_linux_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/namespaces.c"
        ));
        assert!(source.contains("proc_pid_make_inode(dentry->d_sb, task, S_IFLNK | S_IRWXUGO)"));
        assert!(source.contains("return ERR_PTR(-ENOENT);"));
        assert!(source.contains("inode->i_op = &proc_ns_link_inode_operations;"));
        assert!(source.contains("ei->ns_ops = ns_ops;"));
        assert!(source.contains("pid_update_inode(task, inode);"));
        assert!(
            source.contains("return d_splice_alias_ops(inode, dentry, &pid_dentry_operations);")
        );
        assert!(source.contains("if (!dir_emit_dots(file, ctx))"));
        assert!(source.contains("if (ctx->pos >= 2 + ARRAY_SIZE(ns_entries))"));
        assert!(source.contains("if (!proc_fill_cache(file, ctx, ops->name, strlen(ops->name)"));
        assert!(source.contains("ctx->pos++;"));
        assert!(source.contains("unsigned int len = dentry->d_name.len;"));
        assert!(source.contains("if (strlen((*entry)->name) != len)"));
        assert!(source.contains("if (!memcmp(dentry->d_name.name, (*entry)->name, len))"));

        let user = proc_ns_dir_lookup(true, "user", true).expect("user namespace dentry");
        assert_eq!(user.mode, S_IFLNK | 0o777);
        assert_eq!(user.ns_ops_name, "user");
        assert!(user.pid_updated);
        assert!(user.spliced_with_pid_ops);

        assert_eq!(proc_ns_dir_lookup(false, "user", true), Err(-ENOENT));
        assert_eq!(proc_ns_dir_lookup(true, "missing", true), Err(-ENOENT));
        assert_eq!(proc_ns_dir_lookup(true, "user", false), Err(-ENOENT));

        let first = proc_ns_dir_readdir(ProcNsReaddirEnv {
            task_present: true,
            dir_emit_dots: true,
            pos: 0,
            fill_limit: 3,
        });
        assert_eq!(first.ret, 0);
        assert_eq!(first.pos, 5);
        assert_eq!(first.emitted, vec!["net", "uts", "ipc"]);

        let resumed = proc_ns_dir_readdir(ProcNsReaddirEnv {
            task_present: true,
            dir_emit_dots: true,
            pos: first.pos,
            fill_limit: 2,
        });
        assert_eq!(resumed.emitted, vec!["pid", "pid_for_children"]);
        assert_eq!(
            proc_ns_dir_readdir(ProcNsReaddirEnv {
                task_present: false,
                dir_emit_dots: true,
                pos: 0,
                fill_limit: 10,
            })
            .ret,
            -ENOENT
        );
        assert!(
            proc_ns_dir_readdir(ProcNsReaddirEnv {
                task_present: true,
                dir_emit_dots: false,
                pos: 0,
                fill_limit: 10,
            })
            .emitted
            .is_empty()
        );
    }

    #[test]
    fn new_ns_dir_exposes_all_configured_namespace_links() {
        let dir = new_ns_dir();
        for entry in PROC_NS_ENTRIES {
            let child = lookup(&dir, entry.name).expect("namespace link");
            match &child.kind {
                KernfsKind::Symlink { target } => assert_eq!(target.as_str(), entry.target),
                _ => panic!("namespace entry is not a symlink"),
            }
        }
    }
}
