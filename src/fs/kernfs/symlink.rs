//! linux-parity: complete
//! linux-source: vendor/linux/fs/kernfs/symlink.c
//! test-origin: linux:vendor/linux/fs/kernfs/symlink.c
//! kernfs symlink helpers.
//!
//! Ref: `vendor/linux/fs/kernfs/symlink.c`

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::include::uapi::errno::{ECHILD, EINVAL, ENAMETOOLONG, ENOMEM};
use crate::include::uapi::stat::S_IFLNK;

use super::{KernfsNode, add_child};

pub const KERNFS_LINK_MODE: u32 = S_IFLNK | 0o777;
pub const KERNFS_PATH_MAX: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernfsOwner {
    pub uid: u32,
    pub gid: u32,
}

pub const GLOBAL_ROOT_OWNER: KernfsOwner = KernfsOwner { uid: 0, gid: 0 };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernfsCreateLinkEnv {
    pub target_owner: Option<KernfsOwner>,
    pub parent_ns_enabled: bool,
    pub target_ns: Option<u64>,
    pub node_allocated: bool,
    pub add_one_ret: i32,
}

impl KernfsCreateLinkEnv {
    pub const SUCCESS: Self = Self {
        target_owner: None,
        parent_ns_enabled: false,
        target_ns: None,
        node_allocated: true,
        add_one_ret: 0,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernfsCreateLinkPlan {
    pub name: String,
    pub mode: u32,
    pub owner: KernfsOwner,
    pub ns: Option<u64>,
    pub target_ref_taken: bool,
    pub added: bool,
    pub put_on_error: bool,
}

pub fn kernfs_create_link_plan(
    name: &str,
    env: KernfsCreateLinkEnv,
) -> Result<KernfsCreateLinkPlan, i32> {
    if !env.node_allocated {
        return Err(-ENOMEM);
    }

    let owner = env.target_owner.unwrap_or(GLOBAL_ROOT_OWNER);
    let ns = if env.parent_ns_enabled {
        env.target_ns
    } else {
        None
    };

    if env.add_one_ret != 0 {
        return Err(env.add_one_ret);
    }

    Ok(KernfsCreateLinkPlan {
        name: String::from(name),
        mode: KERNFS_LINK_MODE,
        owner,
        ns,
        target_ref_taken: true,
        added: true,
        put_on_error: false,
    })
}

pub fn kernfs_create_link_puts_node_on_add_error(env: KernfsCreateLinkEnv) -> bool {
    env.node_allocated && env.add_one_ret != 0
}

pub fn kernfs_create_link(parent: &Arc<KernfsNode>, name: &str, target: &str) -> Arc<KernfsNode> {
    let link = KernfsNode::new_symlink(name, target);
    add_child(parent, link.clone());
    link
}

pub fn kernfs_get_target_path(parent_path: &[&str], target_path: &[&str]) -> Result<String, i32> {
    kernfs_get_target_path_with_limit(parent_path, target_path, KERNFS_PATH_MAX)
}

pub fn kernfs_get_target_path_with_limit(
    parent_path: &[&str],
    target_path: &[&str],
    path_max: usize,
) -> Result<String, i32> {
    let common = common_prefix_len(parent_path, target_path);
    let mut path = String::new();

    for _ in common..parent_path.len() {
        if path.len() + 3 >= path_max {
            return Err(-ENAMETOOLONG);
        }
        path.push_str("../");
    }

    let target_tail = &target_path[common..];
    let raw_tail_len = target_tail
        .iter()
        .fold(0usize, |len, component| len + component.len() + 1);
    if raw_tail_len < 2 {
        return Err(-EINVAL);
    }
    let tail_len = raw_tail_len - 1;
    if path.len() + tail_len >= path_max {
        return Err(-ENAMETOOLONG);
    }

    let mut first = true;
    for component in target_tail {
        if !first {
            path.push('/');
        }
        path.push_str(component);
        first = false;
    }

    Ok(path)
}

fn common_prefix_len(left: &[&str], right: &[&str]) -> usize {
    let mut idx = 0;
    while idx < left.len() && idx < right.len() && left[idx].as_bytes() == right[idx].as_bytes() {
        idx += 1;
    }
    idx
}

pub fn kernfs_node_path_from_root(node: &Arc<KernfsNode>) -> Vec<String> {
    let mut components = Vec::new();
    let mut current = Some(node.clone());
    while let Some(kn) = current {
        let parent = kn.parent.lock().upgrade();
        if parent.is_some() {
            components.push(kn.name.clone());
        }
        current = parent;
    }
    components.reverse();
    components
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernfsGetLinkEnv {
    pub dentry_present: bool,
    pub body_allocated: bool,
    pub getlink_ret: i32,
}

impl KernfsGetLinkEnv {
    pub const SUCCESS: Self = Self {
        dentry_present: true,
        body_allocated: true,
        getlink_ret: 0,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernfsGetLinkResult {
    pub ret: i32,
    pub body_returned: bool,
    pub body_freed: bool,
    pub delayed_call_set: bool,
}

pub fn kernfs_iop_get_link(env: KernfsGetLinkEnv) -> KernfsGetLinkResult {
    if !env.dentry_present {
        return KernfsGetLinkResult {
            ret: -ECHILD,
            body_returned: false,
            body_freed: false,
            delayed_call_set: false,
        };
    }
    if !env.body_allocated {
        return KernfsGetLinkResult {
            ret: -ENOMEM,
            body_returned: false,
            body_freed: false,
            delayed_call_set: false,
        };
    }
    if env.getlink_ret < 0 {
        return KernfsGetLinkResult {
            ret: env.getlink_ret,
            body_returned: false,
            body_freed: true,
            delayed_call_set: false,
        };
    }
    KernfsGetLinkResult {
        ret: 0,
        body_returned: true,
        body_freed: false,
        delayed_call_set: true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernfsSymlinkIops {
    pub listxattr: &'static str,
    pub get_link: &'static str,
    pub setattr: &'static str,
    pub getattr: &'static str,
    pub permission: &'static str,
}

pub const KERNFS_SYMLINK_IOPS: KernfsSymlinkIops = KernfsSymlinkIops {
    listxattr: "kernfs_iop_listxattr",
    get_link: "kernfs_iop_get_link",
    setattr: "kernfs_iop_setattr",
    getattr: "kernfs_iop_getattr",
    permission: "kernfs_iop_permission",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::lookup;

    #[test]
    fn kernfs_create_link_matches_linux_source_metadata() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/kernfs/symlink.c"
        ));
        assert!(source.contains("kuid_t uid = GLOBAL_ROOT_UID;"));
        assert!(source.contains("kgid_t gid = GLOBAL_ROOT_GID;"));
        assert!(source.contains("if (target->iattr)"));
        assert!(source.contains("kn = kernfs_new_node(parent, name, S_IFLNK|0777"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("if (kernfs_ns_enabled(parent))"));
        assert!(source.contains("kn->ns = target->ns;"));
        assert!(source.contains("kn->symlink.target_kn = target;"));
        assert!(source.contains("kernfs_get(target);"));
        assert!(source.contains("error = kernfs_add_one(kn);"));
        assert!(source.contains("kernfs_put(kn);"));

        assert_eq!(
            kernfs_create_link_plan(
                "target",
                KernfsCreateLinkEnv {
                    target_owner: Some(KernfsOwner {
                        uid: 1000,
                        gid: 100
                    }),
                    parent_ns_enabled: true,
                    target_ns: Some(42),
                    ..KernfsCreateLinkEnv::SUCCESS
                }
            )
            .unwrap(),
            KernfsCreateLinkPlan {
                name: String::from("target"),
                mode: KERNFS_LINK_MODE,
                owner: KernfsOwner {
                    uid: 1000,
                    gid: 100
                },
                ns: Some(42),
                target_ref_taken: true,
                added: true,
                put_on_error: false,
            }
        );
        assert_eq!(
            kernfs_create_link_plan(
                "target",
                KernfsCreateLinkEnv {
                    node_allocated: false,
                    ..KernfsCreateLinkEnv::SUCCESS
                }
            ),
            Err(-ENOMEM)
        );
        assert_eq!(
            kernfs_create_link_plan(
                "target",
                KernfsCreateLinkEnv {
                    add_one_ret: -EINVAL,
                    ..KernfsCreateLinkEnv::SUCCESS
                }
            ),
            Err(-EINVAL)
        );
        assert!(kernfs_create_link_puts_node_on_add_error(
            KernfsCreateLinkEnv {
                add_one_ret: -EINVAL,
                ..KernfsCreateLinkEnv::SUCCESS
            }
        ));
    }

    #[test]
    fn kernfs_get_target_path_matches_linux_relative_algorithm() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/kernfs/symlink.c"
        ));
        assert!(source.contains("base = parent;"));
        assert!(source.contains("while (kernfs_parent(base))"));
        assert!(source.contains("strcpy(s, \"../\");"));
        assert!(source.contains("len += strlen(kernfs_rcu_name(kn)) + 1;"));
        assert!(source.contains("if (len < 2)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("if ((s - path) + len >= PATH_MAX)"));
        assert!(source.contains("memcpy(s + len, name, slen);"));
        assert!(source.contains("s[--len] = '/';"));

        assert_eq!(
            kernfs_get_target_path(
                &["sys", "devices", "pci0000:00"],
                &["sys", "devices", "system", "cpu", "cpu0"]
            ),
            Ok(String::from("../system/cpu/cpu0"))
        );
        assert_eq!(
            kernfs_get_target_path(&["sys", "kernel"], &["sys", "kernel", "debug"]),
            Ok(String::from("debug"))
        );
        assert_eq!(
            kernfs_get_target_path(&["sys", "kernel"], &["sys", "kernel"]),
            Err(-EINVAL)
        );
        assert_eq!(
            kernfs_get_target_path_with_limit(&["a", "b"], &["a", "c"], 4),
            Err(-ENAMETOOLONG)
        );
        assert_eq!(
            kernfs_get_target_path_with_limit(&["a"], &["a", "toolong"], 7),
            Err(-ENAMETOOLONG)
        );
    }

    #[test]
    fn kernfs_iop_get_link_matches_linux_error_cleanup_order() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/kernfs/symlink.c"
        ));
        assert!(source.contains("if (!dentry)"));
        assert!(source.contains("return ERR_PTR(-ECHILD);"));
        assert!(source.contains("body = kzalloc(PAGE_SIZE, GFP_KERNEL);"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("error = kernfs_getlink(inode, body);"));
        assert!(source.contains("kfree(body);"));
        assert!(source.contains("set_delayed_call(done, kfree_link, body);"));
        assert!(source.contains(".get_link\t= kernfs_iop_get_link"));

        assert_eq!(
            kernfs_iop_get_link(KernfsGetLinkEnv {
                dentry_present: false,
                ..KernfsGetLinkEnv::SUCCESS
            })
            .ret,
            -ECHILD
        );
        assert_eq!(
            kernfs_iop_get_link(KernfsGetLinkEnv {
                body_allocated: false,
                ..KernfsGetLinkEnv::SUCCESS
            })
            .ret,
            -ENOMEM
        );
        assert_eq!(
            kernfs_iop_get_link(KernfsGetLinkEnv {
                getlink_ret: -EINVAL,
                ..KernfsGetLinkEnv::SUCCESS
            }),
            KernfsGetLinkResult {
                ret: -EINVAL,
                body_returned: false,
                body_freed: true,
                delayed_call_set: false,
            }
        );
        assert_eq!(
            kernfs_iop_get_link(KernfsGetLinkEnv::SUCCESS),
            KernfsGetLinkResult {
                ret: 0,
                body_returned: true,
                body_freed: false,
                delayed_call_set: true,
            }
        );
        assert_eq!(KERNFS_SYMLINK_IOPS.get_link, "kernfs_iop_get_link");
        assert_eq!(KERNFS_SYMLINK_IOPS.permission, "kernfs_iop_permission");
    }

    #[test]
    fn kernfs_create_link_helper_adds_symlink_node() {
        let root = KernfsNode::new_dir("/", 0o755);
        let link = kernfs_create_link(&root, "driver", "../bus/pci/drivers/foo");
        assert!(Arc::ptr_eq(
            &lookup(&root, "driver").expect("created link"),
            &link
        ));
    }

    #[test]
    fn kernfs_node_path_from_root_follows_parent_links() {
        let root = KernfsNode::new_dir("/", 0o755);
        let sys = KernfsNode::new_dir("sys", 0o755);
        let kernel = KernfsNode::new_dir("kernel", 0o755);
        add_child(&root, sys.clone());
        add_child(&sys, kernel.clone());

        assert_eq!(
            kernfs_node_path_from_root(&kernel),
            alloc::vec![String::from("sys"), String::from("kernel")]
        );
    }
}
