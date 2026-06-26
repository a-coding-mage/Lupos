//! linux-parity: complete
//! linux-source: vendor/linux/fs/gfs2/dentry.c
//! test-origin: linux:vendor/linux/fs/gfs2/dentry.c
//! GFS2 dentry operation decisions.

use crate::include::uapi::errno::{ECHILD, ENOENT};

pub const LOOKUP_RCU: u32 = 1 << 8;
pub const LM_ST_SHARED: u32 = 3;
pub const GLF_DEMOTE: u32 = 3;

pub const fn gfs2_drevalidate_result(
    flags: u32,
    inode_present: bool,
    inode_bad: bool,
    lm_mount_present: bool,
    had_lock: bool,
    lock_error: i32,
    dir_check_error: i32,
) -> i32 {
    if flags & LOOKUP_RCU != 0 {
        return -ECHILD;
    }
    if inode_present && inode_bad {
        return 0;
    }
    if !lm_mount_present {
        return 1;
    }
    if !had_lock && lock_error != 0 {
        return 0;
    }
    if inode_present {
        (dir_check_error == 0) as i32
    } else {
        (dir_check_error == -ENOENT) as i32
    }
}

pub const fn gfs2_dentry_delete(negative: bool, holder_initialized: bool, demote: bool) -> i32 {
    if negative || !holder_initialized {
        return 0;
    }
    demote as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gfs2_dentry_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/gfs2/dentry.c"
        ));
        assert!(source.contains("#include <linux/namei.h>"));
        assert!(source.contains("#include <linux/crc32.h>"));
        assert!(source.contains("static int gfs2_drevalidate"));
        assert!(source.contains("if (flags & LOOKUP_RCU)"));
        assert!(source.contains("return -ECHILD;"));
        assert!(source.contains("if (is_bad_inode(inode))"));
        assert!(source.contains("if (sdp->sd_lockstruct.ls_ops->lm_mount == NULL)"));
        assert!(source.contains("gfs2_glock_is_locked_by_me(dip->i_gl)"));
        assert!(source.contains("gfs2_glock_nq_init(dip->i_gl, LM_ST_SHARED, 0, &d_gh);"));
        assert!(source.contains("valid = inode ? !error : (error == -ENOENT);"));
        assert!(source.contains("str->hash = gfs2_disk_hash(str->name, str->len);"));
        assert!(source.contains("if (d_really_is_negative(dentry))"));
        assert!(source.contains("if (!gfs2_holder_initialized(&ginode->i_iopen_gh))"));
        assert!(source.contains("if (test_bit(GLF_DEMOTE, &ginode->i_iopen_gh.gh_gl->gl_flags))"));
        assert!(source.contains("const struct dentry_operations gfs2_dops"));

        assert_eq!(
            gfs2_drevalidate_result(LOOKUP_RCU, false, false, true, true, 0, 0),
            -ECHILD
        );
        assert_eq!(gfs2_drevalidate_result(0, true, true, true, true, 0, 0), 0);
        assert_eq!(
            gfs2_drevalidate_result(0, true, false, false, false, 0, 0),
            1
        );
        assert_eq!(
            gfs2_drevalidate_result(0, true, false, true, false, -5, 0),
            0
        );
        assert_eq!(
            gfs2_drevalidate_result(0, true, false, true, false, 0, 0),
            1
        );
        assert_eq!(
            gfs2_drevalidate_result(0, false, false, true, true, 0, -ENOENT),
            1
        );
        assert_eq!(
            gfs2_drevalidate_result(0, false, false, true, true, 0, 0),
            0
        );
        assert_eq!(gfs2_dentry_delete(true, true, true), 0);
        assert_eq!(gfs2_dentry_delete(false, false, true), 0);
        assert_eq!(gfs2_dentry_delete(false, true, true), 1);
        assert_eq!(gfs2_dentry_delete(false, true, false), 0);
    }
}
