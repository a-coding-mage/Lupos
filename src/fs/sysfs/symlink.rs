//! linux-parity: complete
//! linux-source: vendor/linux/fs/sysfs/symlink.c
//! test-origin: linux:vendor/linux/fs/sysfs/symlink.c
//! sysfs symlink helpers.
//!
//! Ref: `vendor/linux/fs/sysfs/symlink.c`

use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, add_child};
use crate::include::uapi::errno::{EEXIST, EFAULT, EINVAL, ENOENT};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsCreateLinkSdEnv {
    pub name_present: bool,
    pub parent_present: bool,
    pub target_sd_present: bool,
    pub kernfs_create_link_ret: i32,
    pub warn: bool,
}

impl SysfsCreateLinkSdEnv {
    pub const SUCCESS: Self = Self {
        name_present: true,
        parent_present: true,
        target_sd_present: true,
        kernfs_create_link_ret: 0,
        warn: true,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsCreateLinkReport {
    pub ret: i32,
    pub symlink_target_lock_taken: bool,
    pub target_ref_taken: bool,
    pub target_ref_put: bool,
    pub warn_dup: bool,
}

pub fn sysfs_do_create_link_sd(env: SysfsCreateLinkSdEnv) -> SysfsCreateLinkReport {
    if !env.name_present || !env.parent_present {
        return SysfsCreateLinkReport {
            ret: -EINVAL,
            symlink_target_lock_taken: false,
            target_ref_taken: false,
            target_ref_put: false,
            warn_dup: false,
        };
    }

    if !env.target_sd_present {
        return SysfsCreateLinkReport {
            ret: -ENOENT,
            symlink_target_lock_taken: true,
            target_ref_taken: false,
            target_ref_put: false,
            warn_dup: false,
        };
    }

    SysfsCreateLinkReport {
        ret: env.kernfs_create_link_ret,
        symlink_target_lock_taken: true,
        target_ref_taken: true,
        target_ref_put: true,
        warn_dup: env.warn && env.kernfs_create_link_ret == -EEXIST,
    }
}

pub fn sysfs_create_link_sd(env: SysfsCreateLinkSdEnv) -> SysfsCreateLinkReport {
    sysfs_do_create_link_sd(SysfsCreateLinkSdEnv { warn: true, ..env })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SysfsLinkParent {
    KobjectSd,
    Root,
    Missing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsCreateLinkEnv {
    pub kobject_present: bool,
    pub kobject_sd_present: bool,
    pub root_present: bool,
    pub name_present: bool,
    pub target_sd_present: bool,
    pub kernfs_create_link_ret: i32,
    pub warn: bool,
}

impl SysfsCreateLinkEnv {
    pub const SUCCESS: Self = Self {
        kobject_present: true,
        kobject_sd_present: true,
        root_present: true,
        name_present: true,
        target_sd_present: true,
        kernfs_create_link_ret: 0,
        warn: true,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsCreateLinkWrapperReport {
    pub parent: SysfsLinkParent,
    pub create: Option<SysfsCreateLinkReport>,
    pub ret: i32,
}

pub fn sysfs_do_create_link(env: SysfsCreateLinkEnv) -> SysfsCreateLinkWrapperReport {
    let parent = if env.kobject_present {
        if env.kobject_sd_present {
            SysfsLinkParent::KobjectSd
        } else {
            SysfsLinkParent::Missing
        }
    } else if env.root_present {
        SysfsLinkParent::Root
    } else {
        SysfsLinkParent::Missing
    };

    if parent == SysfsLinkParent::Missing {
        return SysfsCreateLinkWrapperReport {
            parent,
            create: None,
            ret: -EFAULT,
        };
    }

    let create = sysfs_do_create_link_sd(SysfsCreateLinkSdEnv {
        name_present: env.name_present,
        parent_present: true,
        target_sd_present: env.target_sd_present,
        kernfs_create_link_ret: env.kernfs_create_link_ret,
        warn: env.warn,
    });
    SysfsCreateLinkWrapperReport {
        parent,
        ret: create.ret,
        create: Some(create),
    }
}

pub fn sysfs_create_link(env: SysfsCreateLinkEnv) -> SysfsCreateLinkWrapperReport {
    sysfs_do_create_link(SysfsCreateLinkEnv { warn: true, ..env })
}

pub fn sysfs_create_link_nowarn(env: SysfsCreateLinkEnv) -> SysfsCreateLinkWrapperReport {
    sysfs_do_create_link(SysfsCreateLinkEnv { warn: false, ..env })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsDeleteLinkReport {
    pub symlink_target_lock_taken: bool,
    pub namespace_from_target: bool,
    pub remove_by_name_ns_called: bool,
    pub name: String,
}

pub fn sysfs_delete_link(
    target_sd_present: bool,
    parent_ns_enabled: bool,
    name: &str,
) -> SysfsDeleteLinkReport {
    SysfsDeleteLinkReport {
        symlink_target_lock_taken: true,
        namespace_from_target: target_sd_present && parent_ns_enabled,
        remove_by_name_ns_called: true,
        name: String::from(name),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsRemoveLinkReport {
    pub parent: SysfsLinkParent,
    pub remove_by_name_called: bool,
    pub name: String,
}

pub fn sysfs_remove_link(
    kobject_present: bool,
    kobject_sd_present: bool,
    root_present: bool,
    name: &str,
) -> SysfsRemoveLinkReport {
    let parent = if kobject_present {
        if kobject_sd_present {
            SysfsLinkParent::KobjectSd
        } else {
            SysfsLinkParent::Missing
        }
    } else if root_present {
        SysfsLinkParent::Root
    } else {
        SysfsLinkParent::Missing
    };
    SysfsRemoveLinkReport {
        parent,
        remove_by_name_called: true,
        name: String::from(name),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SysfsRenameLinkEnv {
    pub kobject_present: bool,
    pub kobject_sd_present: bool,
    pub root_present: bool,
    pub target_sd_present: bool,
    pub found_link: bool,
    pub found_type_is_link: bool,
    pub link_points_to_target: bool,
    pub kernfs_rename_ret: i32,
}

impl SysfsRenameLinkEnv {
    pub const SUCCESS: Self = Self {
        kobject_present: true,
        kobject_sd_present: true,
        root_present: true,
        target_sd_present: true,
        found_link: true,
        found_type_is_link: true,
        link_points_to_target: true,
        kernfs_rename_ret: 0,
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SysfsRenameLinkReport {
    pub parent: SysfsLinkParent,
    pub old_namespace_from_target: bool,
    pub find_and_get_called: bool,
    pub kernfs_put_called: bool,
    pub rename_called: bool,
    pub ret: i32,
}

pub fn sysfs_rename_link_ns(env: SysfsRenameLinkEnv) -> SysfsRenameLinkReport {
    let parent = if env.kobject_present {
        if env.kobject_sd_present {
            SysfsLinkParent::KobjectSd
        } else {
            SysfsLinkParent::Missing
        }
    } else if env.root_present {
        SysfsLinkParent::Root
    } else {
        SysfsLinkParent::Missing
    };

    if !env.found_link {
        return SysfsRenameLinkReport {
            parent,
            old_namespace_from_target: env.target_sd_present,
            find_and_get_called: true,
            kernfs_put_called: false,
            rename_called: false,
            ret: -ENOENT,
        };
    }
    if !env.found_type_is_link || !env.link_points_to_target {
        return SysfsRenameLinkReport {
            parent,
            old_namespace_from_target: env.target_sd_present,
            find_and_get_called: true,
            kernfs_put_called: true,
            rename_called: false,
            ret: -EINVAL,
        };
    }
    SysfsRenameLinkReport {
        parent,
        old_namespace_from_target: env.target_sd_present,
        find_and_get_called: true,
        kernfs_put_called: true,
        rename_called: true,
        ret: env.kernfs_rename_ret,
    }
}

pub fn create_link(parent: &Arc<KernfsNode>, name: &str, target: &str) -> Arc<KernfsNode> {
    let link = KernfsNode::new_symlink(name, target);
    add_child(parent, link.clone());
    link
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs;

    #[test]
    fn sysfs_create_link_sd_matches_linux_lock_ref_and_warn_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/symlink.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/sysfs.h"
        ));
        assert!(source.contains("static int sysfs_do_create_link_sd"));
        assert!(source.contains("if (WARN_ON(!name || !parent))"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("spin_lock(&sysfs_symlink_target_lock);"));
        assert!(source.contains("if (target_kobj->sd)"));
        assert!(source.contains("kernfs_get(target);"));
        assert!(source.contains("spin_unlock(&sysfs_symlink_target_lock);"));
        assert!(source.contains("if (!target)"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("kn = kernfs_create_link(parent, name, target);"));
        assert!(source.contains("kernfs_put(target);"));
        assert!(source.contains("if (warn && PTR_ERR(kn) == -EEXIST)"));
        assert!(source.contains("sysfs_warn_dup(parent, name);"));
        assert!(header.contains("int sysfs_create_link_sd(struct kernfs_node *kn"));

        assert_eq!(
            sysfs_do_create_link_sd(SysfsCreateLinkSdEnv {
                name_present: false,
                ..SysfsCreateLinkSdEnv::SUCCESS
            })
            .ret,
            -EINVAL
        );
        assert_eq!(
            sysfs_do_create_link_sd(SysfsCreateLinkSdEnv {
                parent_present: false,
                ..SysfsCreateLinkSdEnv::SUCCESS
            })
            .ret,
            -EINVAL
        );
        assert_eq!(
            sysfs_do_create_link_sd(SysfsCreateLinkSdEnv {
                target_sd_present: false,
                ..SysfsCreateLinkSdEnv::SUCCESS
            }),
            SysfsCreateLinkReport {
                ret: -ENOENT,
                symlink_target_lock_taken: true,
                target_ref_taken: false,
                target_ref_put: false,
                warn_dup: false,
            }
        );
        assert_eq!(
            sysfs_do_create_link_sd(SysfsCreateLinkSdEnv {
                kernfs_create_link_ret: -EEXIST,
                ..SysfsCreateLinkSdEnv::SUCCESS
            })
            .warn_dup,
            true
        );
        assert_eq!(
            sysfs_create_link_sd(SysfsCreateLinkSdEnv {
                warn: false,
                kernfs_create_link_ret: -EEXIST,
                ..SysfsCreateLinkSdEnv::SUCCESS
            })
            .warn_dup,
            true
        );
    }

    #[test]
    fn public_create_link_wrappers_match_parent_and_warning_rules() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/symlink.c"
        ));
        assert!(source.contains("if (!kobj)"));
        assert!(source.contains("parent = sysfs_root_kn;"));
        assert!(source.contains("parent = kobj->sd;"));
        assert!(source.contains("if (!parent)"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("return sysfs_do_create_link(kobj, target, name, 1);"));
        assert!(source.contains("return sysfs_do_create_link(kobj, target, name, 0);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sysfs_create_link);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sysfs_create_link_nowarn);"));

        assert_eq!(
            sysfs_do_create_link(SysfsCreateLinkEnv {
                kobject_present: false,
                root_present: false,
                ..SysfsCreateLinkEnv::SUCCESS
            })
            .ret,
            -EFAULT
        );
        assert_eq!(
            sysfs_create_link(SysfsCreateLinkEnv {
                kobject_present: false,
                ..SysfsCreateLinkEnv::SUCCESS
            })
            .parent,
            SysfsLinkParent::Root
        );
        assert_eq!(
            sysfs_create_link_nowarn(SysfsCreateLinkEnv {
                kernfs_create_link_ret: -EEXIST,
                ..SysfsCreateLinkEnv::SUCCESS
            })
            .create
            .unwrap()
            .warn_dup,
            false
        );
    }

    #[test]
    fn delete_remove_and_rename_link_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysfs/symlink.c"
        ));
        assert!(source.contains("void sysfs_delete_link"));
        assert!(source.contains("if (targ->sd && kernfs_ns_enabled(kobj->sd))"));
        assert!(source.contains("ns = targ->sd->ns;"));
        assert!(source.contains("kernfs_remove_by_name_ns(kobj->sd, name, ns);"));
        assert!(source.contains("void sysfs_remove_link"));
        assert!(source.contains("kernfs_remove_by_name(parent, name);"));
        assert!(source.contains("int sysfs_rename_link_ns"));
        assert!(source.contains("if (targ->sd)"));
        assert!(source.contains("old_ns = targ->sd->ns;"));
        assert!(source.contains("result = -ENOENT;"));
        assert!(source.contains("kn = kernfs_find_and_get_ns(parent, old, old_ns);"));
        assert!(source.contains("result = -EINVAL;"));
        assert!(source.contains("if (kernfs_type(kn) != KERNFS_LINK)"));
        assert!(source.contains("if (kn->symlink.target_kn->priv != targ)"));
        assert!(source.contains("result = kernfs_rename_ns(kn, parent, new, new_ns);"));
        assert!(source.contains("kernfs_put(kn);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sysfs_rename_link_ns);"));

        assert_eq!(
            sysfs_delete_link(true, true, "device"),
            SysfsDeleteLinkReport {
                symlink_target_lock_taken: true,
                namespace_from_target: true,
                remove_by_name_ns_called: true,
                name: String::from("device"),
            }
        );
        assert_eq!(
            sysfs_remove_link(false, false, true, "device").parent,
            SysfsLinkParent::Root
        );
        assert_eq!(
            sysfs_rename_link_ns(SysfsRenameLinkEnv {
                found_link: false,
                ..SysfsRenameLinkEnv::SUCCESS
            })
            .ret,
            -ENOENT
        );
        assert_eq!(
            sysfs_rename_link_ns(SysfsRenameLinkEnv {
                found_type_is_link: false,
                ..SysfsRenameLinkEnv::SUCCESS
            })
            .ret,
            -EINVAL
        );
        assert_eq!(
            sysfs_rename_link_ns(SysfsRenameLinkEnv {
                link_points_to_target: false,
                ..SysfsRenameLinkEnv::SUCCESS
            })
            .ret,
            -EINVAL
        );
        assert_eq!(
            sysfs_rename_link_ns(SysfsRenameLinkEnv {
                kernfs_rename_ret: -EEXIST,
                ..SysfsRenameLinkEnv::SUCCESS
            }),
            SysfsRenameLinkReport {
                parent: SysfsLinkParent::KobjectSd,
                old_namespace_from_target: true,
                find_and_get_called: true,
                kernfs_put_called: true,
                rename_called: true,
                ret: -EEXIST,
            }
        );
    }

    #[test]
    fn create_link_helper_attaches_kernfs_symlink() {
        let root = kernfs::KernfsNode::new_dir("/", 0o755);
        let link = create_link(&root, "device", "../devices/pci0000:00");
        assert!(Arc::ptr_eq(
            &kernfs::lookup(&root, "device").expect("device link"),
            &link
        ));
    }
}
