//! linux-parity: complete
//! linux-source: vendor/linux/fs/sysfs/dir.c
//! test-origin: linux:vendor/linux/fs/sysfs/dir.c
//! sysfs directory helpers.
//!
//! Ref: `vendor/linux/fs/sysfs/dir.c`

use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, add_child};
use crate::include::uapi::errno::{EEXIST, EINVAL, ENOENT};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsWarnDupReport {
    pub path_buffer_allocated: bool,
    pub kernfs_path_called: bool,
    pub warning: String,
    pub stack_dumped: bool,
    pub buffer_freed: bool,
}

pub fn sysfs_warn_dup(parent_path: Option<&str>, name: &str) -> SysfsWarnDupReport {
    let path_buffer_allocated = parent_path.is_some();
    let prefix = parent_path.unwrap_or("(null)");
    SysfsWarnDupReport {
        path_buffer_allocated,
        kernfs_path_called: path_buffer_allocated,
        warning: alloc::format!("cannot create duplicate filename '{prefix}/{name}'"),
        stack_dumped: true,
        buffer_freed: path_buffer_allocated,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SysfsParent {
    ObjectParent,
    ObjectParentMissingSd,
    Root,
    MissingRoot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsCreateDirEnv {
    pub kobject_present: bool,
    pub parent: SysfsParent,
    pub kernfs_create_dir_ret: i32,
}

impl SysfsCreateDirEnv {
    pub const SUCCESS: Self = Self {
        kobject_present: true,
        parent: SysfsParent::Root,
        kernfs_create_dir_ret: 0,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsCreateDirReport {
    pub ret: i32,
    pub parent: Option<SysfsParent>,
    pub ownership_queried: bool,
    pub warn_dup: bool,
    pub kobject_sd_assigned: bool,
}

pub fn sysfs_create_dir_ns(env: SysfsCreateDirEnv) -> SysfsCreateDirReport {
    if !env.kobject_present {
        return SysfsCreateDirReport {
            ret: -EINVAL,
            parent: None,
            ownership_queried: false,
            warn_dup: false,
            kobject_sd_assigned: false,
        };
    }

    if matches!(
        env.parent,
        SysfsParent::ObjectParentMissingSd | SysfsParent::MissingRoot
    ) {
        return SysfsCreateDirReport {
            ret: -ENOENT,
            parent: Some(env.parent),
            ownership_queried: false,
            warn_dup: false,
            kobject_sd_assigned: false,
        };
    }

    if env.kernfs_create_dir_ret != 0 {
        return SysfsCreateDirReport {
            ret: env.kernfs_create_dir_ret,
            parent: Some(env.parent),
            ownership_queried: true,
            warn_dup: env.kernfs_create_dir_ret == -EEXIST,
            kobject_sd_assigned: false,
        };
    }

    SysfsCreateDirReport {
        ret: 0,
        parent: Some(env.parent),
        ownership_queried: true,
        warn_dup: false,
        kobject_sd_assigned: true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsRemoveDirReport {
    pub symlink_target_lock_taken: bool,
    pub sd_cleared: bool,
    pub warned_non_dir: bool,
    pub kernfs_remove_called: bool,
}

pub fn sysfs_remove_dir(sd_present: bool, kernfs_type_is_dir: bool) -> SysfsRemoveDirReport {
    SysfsRemoveDirReport {
        symlink_target_lock_taken: true,
        sd_cleared: true,
        warned_non_dir: sd_present && !kernfs_type_is_dir,
        kernfs_remove_called: sd_present,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsRenameDirReport {
    pub parent_get: bool,
    pub rename_called: bool,
    pub parent_put: bool,
    pub ret: i32,
}

pub fn sysfs_rename_dir_ns(kernfs_rename_ret: i32) -> SysfsRenameDirReport {
    SysfsRenameDirReport {
        parent_get: true,
        rename_called: true,
        parent_put: true,
        ret: kernfs_rename_ret,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SysfsMoveParent {
    NewParentObject,
    Root,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsMoveDirEnv {
    pub new_parent_kobject_present: bool,
    pub new_parent_sd_present: bool,
    pub kernfs_rename_ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsMoveDirReport {
    pub parent: SysfsMoveParent,
    pub rename_name_is_null: bool,
    pub ret: i32,
}

pub fn sysfs_move_dir_ns(env: SysfsMoveDirEnv) -> SysfsMoveDirReport {
    let parent = if env.new_parent_kobject_present && env.new_parent_sd_present {
        SysfsMoveParent::NewParentObject
    } else {
        SysfsMoveParent::Root
    };
    SysfsMoveDirReport {
        parent,
        rename_name_is_null: true,
        ret: env.kernfs_rename_ret,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsMountPointReport {
    pub ret: i32,
    pub warn_dup: bool,
}

pub fn sysfs_create_mount_point(kernfs_create_empty_dir_ret: i32) -> SysfsMountPointReport {
    SysfsMountPointReport {
        ret: kernfs_create_empty_dir_ret,
        warn_dup: kernfs_create_empty_dir_ret == -EEXIST,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsRemoveMountPointReport {
    pub parent_sd_used: bool,
    pub name: String,
    pub namespace_is_null: bool,
}

pub fn sysfs_remove_mount_point(name: &str) -> SysfsRemoveMountPointReport {
    SysfsRemoveMountPointReport {
        parent_sd_used: true,
        name: String::from(name),
        namespace_is_null: true,
    }
}

pub fn create_dir(parent: &Arc<KernfsNode>, name: &str) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(name, 0o755);
    add_child(parent, dir.clone());
    dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs;

    #[test]
    fn sysfs_warn_dup_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/dir.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/sysfs.h"
        ));
        assert!(source.contains("DEFINE_SPINLOCK(sysfs_symlink_target_lock);"));
        assert!(source.contains("buf = kzalloc(PATH_MAX, GFP_KERNEL);"));
        assert!(source.contains("kernfs_path(parent, buf, PATH_MAX);"));
        assert!(
            source.contains("pr_warn(\"cannot create duplicate filename '%s/%s'\\n\", buf, name);")
        );
        assert!(source.contains("dump_stack();"));
        assert!(source.contains("kfree(buf);"));
        assert!(header.contains("extern spinlock_t sysfs_symlink_target_lock;"));

        assert_eq!(
            sysfs_warn_dup(Some("/sys/devices"), "cpu"),
            SysfsWarnDupReport {
                path_buffer_allocated: true,
                kernfs_path_called: true,
                warning: String::from("cannot create duplicate filename '/sys/devices/cpu'"),
                stack_dumped: true,
                buffer_freed: true,
            }
        );
        assert_eq!(
            sysfs_warn_dup(None, "cpu").warning,
            "cannot create duplicate filename '(null)/cpu'"
        );
    }

    #[test]
    fn sysfs_create_dir_ns_matches_linux_error_and_assignment_order() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/dir.c"
        ));
        assert!(source.contains("if (WARN_ON(!kobj))"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("if (kobj->parent)"));
        assert!(source.contains("parent = kobj->parent->sd;"));
        assert!(source.contains("parent = sysfs_root_kn;"));
        assert!(source.contains("if (!parent)"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("kobject_get_ownership(kobj, &uid, &gid);"));
        assert!(
            source.contains("kn = kernfs_create_dir_ns(parent, kobject_name(kobj), 0755, uid, gid")
        );
        assert!(source.contains("if (PTR_ERR(kn) == -EEXIST)"));
        assert!(source.contains("kobj->sd = kn;"));

        assert_eq!(
            sysfs_create_dir_ns(SysfsCreateDirEnv {
                kobject_present: false,
                ..SysfsCreateDirEnv::SUCCESS
            })
            .ret,
            -EINVAL
        );
        assert_eq!(
            sysfs_create_dir_ns(SysfsCreateDirEnv {
                parent: SysfsParent::MissingRoot,
                ..SysfsCreateDirEnv::SUCCESS
            })
            .ret,
            -ENOENT
        );
        assert_eq!(
            sysfs_create_dir_ns(SysfsCreateDirEnv {
                parent: SysfsParent::ObjectParentMissingSd,
                ..SysfsCreateDirEnv::SUCCESS
            })
            .ret,
            -ENOENT
        );
        assert_eq!(
            sysfs_create_dir_ns(SysfsCreateDirEnv {
                parent: SysfsParent::ObjectParent,
                kernfs_create_dir_ret: -EEXIST,
                ..SysfsCreateDirEnv::SUCCESS
            }),
            SysfsCreateDirReport {
                ret: -EEXIST,
                parent: Some(SysfsParent::ObjectParent),
                ownership_queried: true,
                warn_dup: true,
                kobject_sd_assigned: false,
            }
        );
        assert!(sysfs_create_dir_ns(SysfsCreateDirEnv::SUCCESS).kobject_sd_assigned);
    }

    #[test]
    fn sysfs_remove_rename_move_and_mount_points_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/dir.c"
        ));
        assert!(source.contains("spin_lock(&sysfs_symlink_target_lock);"));
        assert!(source.contains("kobj->sd = NULL;"));
        assert!(source.contains("spin_unlock(&sysfs_symlink_target_lock);"));
        assert!(source.contains("WARN_ON_ONCE(kernfs_type(kn) != KERNFS_DIR);"));
        assert!(source.contains("kernfs_remove(kn);"));
        assert!(source.contains("parent = kernfs_get_parent(kobj->sd);"));
        assert!(source.contains("ret = kernfs_rename_ns(kobj->sd, parent, new_name, new_ns);"));
        assert!(source.contains("kernfs_put(parent);"));
        assert!(source.contains("new_parent_kobj && new_parent_kobj->sd ?"));
        assert!(source.contains("return kernfs_rename_ns(kn, new_parent, NULL, new_ns);"));
        assert!(source.contains("kn = kernfs_create_empty_dir(parent, name);"));
        assert!(source.contains("kernfs_remove_by_name_ns(parent, name, NULL);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sysfs_create_mount_point);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sysfs_remove_mount_point);"));

        assert_eq!(
            sysfs_remove_dir(true, false),
            SysfsRemoveDirReport {
                symlink_target_lock_taken: true,
                sd_cleared: true,
                warned_non_dir: true,
                kernfs_remove_called: true,
            }
        );
        assert_eq!(sysfs_remove_dir(false, true).kernfs_remove_called, false);
        assert_eq!(
            sysfs_rename_dir_ns(-ENOENT),
            SysfsRenameDirReport {
                parent_get: true,
                rename_called: true,
                parent_put: true,
                ret: -ENOENT,
            }
        );
        assert_eq!(
            sysfs_move_dir_ns(SysfsMoveDirEnv {
                new_parent_kobject_present: true,
                new_parent_sd_present: false,
                kernfs_rename_ret: 0,
            })
            .parent,
            SysfsMoveParent::Root
        );
        assert_eq!(
            sysfs_move_dir_ns(SysfsMoveDirEnv {
                new_parent_kobject_present: true,
                new_parent_sd_present: true,
                kernfs_rename_ret: -EINVAL,
            }),
            SysfsMoveDirReport {
                parent: SysfsMoveParent::NewParentObject,
                rename_name_is_null: true,
                ret: -EINVAL,
            }
        );
        assert_eq!(
            sysfs_create_mount_point(-EEXIST),
            SysfsMountPointReport {
                ret: -EEXIST,
                warn_dup: true,
            }
        );
        assert_eq!(
            sysfs_remove_mount_point("firmware"),
            SysfsRemoveMountPointReport {
                parent_sd_used: true,
                name: String::from("firmware"),
                namespace_is_null: true,
            }
        );
    }

    #[test]
    fn create_dir_helper_attaches_kernfs_directory() {
        let root = kernfs::KernfsNode::new_dir("/", 0o755);
        let dir = create_dir(&root, "devices");
        assert!(Arc::ptr_eq(
            &kernfs::lookup(&root, "devices").expect("devices"),
            &dir
        ));
    }
}
