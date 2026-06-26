//! linux-parity: complete
//! linux-source: vendor/linux/fs/coda/pioctl.c
//! test-origin: linux:vendor/linux/fs/coda/pioctl.c
//! Coda pioctl permission and upcall flow.

use crate::include::uapi::errno::{EACCES, EINVAL};

pub const MAY_EXEC: i32 = 0x0000_0001;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodaPioctlFlow {
    pub result: i32,
    pub path_put: bool,
    pub venus_called: bool,
}

pub const fn coda_ioctl_permission(mask: i32) -> i32 {
    if mask & MAY_EXEC != 0 { -EACCES } else { 0 }
}

pub const fn coda_pioctl_flow(
    copy_from_user_ok: bool,
    user_path_result: i32,
    same_superblock: bool,
    venus_result: i32,
) -> CodaPioctlFlow {
    if !copy_from_user_ok {
        return CodaPioctlFlow {
            result: -EINVAL,
            path_put: false,
            venus_called: false,
        };
    }
    if user_path_result != 0 {
        return CodaPioctlFlow {
            result: user_path_result,
            path_put: false,
            venus_called: false,
        };
    }
    if !same_superblock {
        return CodaPioctlFlow {
            result: -EINVAL,
            path_put: true,
            venus_called: false,
        };
    }
    CodaPioctlFlow {
        result: venus_result,
        path_put: true,
        venus_called: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coda_pioctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/coda/pioctl.c"
        ));
        assert!(source.contains("#include <linux/types.h>"));
        assert!(source.contains("#include <linux/namei.h>"));
        assert!(source.contains("#include <linux/uaccess.h>"));
        assert!(source.contains("#include <linux/coda.h>"));
        assert!(source.contains("#include \"coda_psdev.h\""));
        assert!(source.contains("#include \"coda_linux.h\""));
        assert!(source.contains("const struct inode_operations coda_ioctl_inode_operations"));
        assert!(source.contains(".permission\t= coda_ioctl_permission"));
        assert!(source.contains(".setattr\t= coda_setattr"));
        assert!(source.contains("const struct file_operations coda_ioctl_operations"));
        assert!(source.contains(".unlocked_ioctl\t= coda_pioctl"));
        assert!(source.contains(".llseek\t\t= noop_llseek"));
        assert!(source.contains("return (mask & MAY_EXEC) ? -EACCES : 0;"));
        assert!(source.contains("copy_from_user(&data"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("user_path_at(AT_FDCWD, data.path"));
        assert!(source.contains("data.follow ? LOOKUP_FOLLOW : 0"));
        assert!(source.contains("target_inode->i_sb != inode->i_sb"));
        assert!(source.contains("venus_pioctl(inode->i_sb, &(cnp->c_fid), cmd, &data);"));
        assert!(source.contains("path_put(&path);"));

        assert_eq!(coda_ioctl_permission(0), 0);
        assert_eq!(coda_ioctl_permission(MAY_EXEC), -EACCES);
        assert_eq!(
            coda_pioctl_flow(false, 0, true, 0),
            CodaPioctlFlow {
                result: -EINVAL,
                path_put: false,
                venus_called: false
            }
        );
        assert_eq!(coda_pioctl_flow(true, -2, true, 0).result, -2);
        assert_eq!(
            coda_pioctl_flow(true, 0, false, 0),
            CodaPioctlFlow {
                result: -EINVAL,
                path_put: true,
                venus_called: false
            }
        );
        assert_eq!(
            coda_pioctl_flow(true, 0, true, -5),
            CodaPioctlFlow {
                result: -5,
                path_put: true,
                venus_called: true
            }
        );
    }
}
