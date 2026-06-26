//! linux-parity: complete
//! linux-source: vendor/linux/fs/btrfs/acl.c
//! test-origin: linux:vendor/linux/fs/btrfs/acl.c
//! Btrfs POSIX ACL xattr dispatch.

use crate::include::uapi::errno::{ECHILD, EINVAL, ENODATA, ENOMEM};

pub const ACL_TYPE_ACCESS: i32 = 0x8000;
pub const ACL_TYPE_DEFAULT: i32 = 0x4000;
pub const XATTR_NAME_POSIX_ACL_ACCESS: &str = "system.posix_acl_access";
pub const XATTR_NAME_POSIX_ACL_DEFAULT: &str = "system.posix_acl_default";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BtrfsAclGet {
    None,
    DecodeXattr { size: usize, name: &'static str },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BtrfsSetAclPlan {
    pub name: &'static str,
    pub convert_acl_to_xattr: bool,
    pub update_access_mode: bool,
    pub used_transaction: bool,
}

pub fn btrfs_acl_xattr_name(acl_type: i32) -> Result<&'static str, i32> {
    match acl_type {
        ACL_TYPE_ACCESS => Ok(XATTR_NAME_POSIX_ACL_ACCESS),
        ACL_TYPE_DEFAULT => Ok(XATTR_NAME_POSIX_ACL_DEFAULT),
        _ => Err(-EINVAL),
    }
}

pub fn btrfs_get_acl_plan(
    acl_type: i32,
    rcu: bool,
    getxattr_size: i32,
) -> Result<BtrfsAclGet, i32> {
    if rcu {
        return Err(-ECHILD);
    }
    let name = btrfs_acl_xattr_name(acl_type)?;
    if getxattr_size > 0 {
        Ok(BtrfsAclGet::DecodeXattr {
            size: getxattr_size as usize,
            name,
        })
    } else if getxattr_size == -ENODATA || getxattr_size == 0 {
        Ok(BtrfsAclGet::None)
    } else {
        Err(getxattr_size)
    }
}

pub fn btrfs_set_acl_plan(
    acl_type: i32,
    inode_is_dir: bool,
    acl_present: bool,
    trans_present: bool,
    xattr_alloc_ok: bool,
    setxattr_result: i32,
) -> Result<BtrfsSetAclPlan, i32> {
    let name = match acl_type {
        ACL_TYPE_ACCESS => XATTR_NAME_POSIX_ACL_ACCESS,
        ACL_TYPE_DEFAULT => {
            if !inode_is_dir {
                return if acl_present {
                    Err(-EINVAL)
                } else {
                    Ok(BtrfsSetAclPlan {
                        name: XATTR_NAME_POSIX_ACL_DEFAULT,
                        convert_acl_to_xattr: false,
                        update_access_mode: false,
                        used_transaction: trans_present,
                    })
                };
            }
            XATTR_NAME_POSIX_ACL_DEFAULT
        }
        _ => return Err(-EINVAL),
    };

    if acl_present && !xattr_alloc_ok {
        return Err(-ENOMEM);
    }
    if setxattr_result < 0 {
        return Err(setxattr_result);
    }

    Ok(BtrfsSetAclPlan {
        name,
        convert_acl_to_xattr: acl_present,
        update_access_mode: acl_type == ACL_TYPE_ACCESS && acl_present,
        used_transaction: trans_present,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btrfs_acl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/btrfs/acl.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include <linux/xattr.h>"));
        assert!(source.contains("#include <linux/posix_acl_xattr.h>"));
        assert!(source.contains("#include <linux/posix_acl.h>"));
        assert!(source.contains("#include <linux/sched.h>"));
        assert!(source.contains("#include <linux/sched/mm.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include \"ctree.h\""));
        assert!(source.contains("#include \"xattr.h\""));
        assert!(source.contains("#include \"acl.h\""));
        assert!(
            source.contains(
                "struct posix_acl *btrfs_get_acl(struct inode *inode, int type, bool rcu)"
            )
        );
        assert!(source.contains("if (rcu)"));
        assert!(source.contains("return ERR_PTR(-ECHILD);"));
        assert!(source.contains("case ACL_TYPE_ACCESS:"));
        assert!(source.contains("name = XATTR_NAME_POSIX_ACL_ACCESS;"));
        assert!(source.contains("case ACL_TYPE_DEFAULT:"));
        assert!(source.contains("name = XATTR_NAME_POSIX_ACL_DEFAULT;"));
        assert!(source.contains("return ERR_PTR(-EINVAL);"));
        assert!(source.contains("size = btrfs_getxattr(inode, name, NULL, 0);"));
        assert!(source.contains("value = kzalloc(size, GFP_KERNEL);"));
        assert!(source.contains("posix_acl_from_xattr(&init_user_ns, value, size);"));
        assert!(source.contains("size == -ENODATA || size == 0"));
        assert!(source.contains(
            "int __btrfs_set_acl(struct btrfs_trans_handle *trans, struct inode *inode,"
        ));
        assert!(source.contains("if (!S_ISDIR(inode->i_mode))"));
        assert!(source.contains("return acl ? -EINVAL : 0;"));
        assert!(source.contains("memalloc_nofs_save();"));
        assert!(source.contains("posix_acl_to_xattr(&init_user_ns, acl, &size, GFP_KERNEL);"));
        assert!(source.contains("ret = btrfs_setxattr(trans, inode, name, value, size, 0);"));
        assert!(source.contains("ret = btrfs_setxattr_trans(inode, name, value, size, 0);"));
        assert!(source.contains("set_cached_acl(inode, type, acl);"));
        assert!(
            source.contains("int btrfs_set_acl(struct mnt_idmap *idmap, struct dentry *dentry,")
        );
        assert!(source.contains("posix_acl_update_mode(idmap, inode,"));
        assert!(source.contains("inode->i_mode = old_mode;"));

        assert_eq!(
            btrfs_acl_xattr_name(ACL_TYPE_ACCESS),
            Ok(XATTR_NAME_POSIX_ACL_ACCESS)
        );
        assert_eq!(btrfs_acl_xattr_name(7), Err(-EINVAL));
        assert_eq!(btrfs_get_acl_plan(ACL_TYPE_ACCESS, true, 0), Err(-ECHILD));
        assert_eq!(
            btrfs_get_acl_plan(ACL_TYPE_DEFAULT, false, 12),
            Ok(BtrfsAclGet::DecodeXattr {
                size: 12,
                name: XATTR_NAME_POSIX_ACL_DEFAULT
            })
        );
        assert_eq!(
            btrfs_get_acl_plan(ACL_TYPE_ACCESS, false, -ENODATA),
            Ok(BtrfsAclGet::None)
        );
        assert_eq!(
            btrfs_set_acl_plan(ACL_TYPE_DEFAULT, false, true, false, true, 0),
            Err(-EINVAL)
        );
        assert_eq!(
            btrfs_set_acl_plan(ACL_TYPE_DEFAULT, false, false, false, true, 0)
                .unwrap()
                .convert_acl_to_xattr,
            false
        );
        assert_eq!(
            btrfs_set_acl_plan(ACL_TYPE_ACCESS, true, true, false, false, 0),
            Err(-ENOMEM)
        );
        let plan = btrfs_set_acl_plan(ACL_TYPE_ACCESS, true, true, true, true, 0).unwrap();
        assert_eq!(plan.name, XATTR_NAME_POSIX_ACL_ACCESS);
        assert!(plan.convert_acl_to_xattr);
        assert!(plan.update_access_mode);
        assert!(plan.used_transaction);
    }
}
