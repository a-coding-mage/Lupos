//! linux-parity: complete
//! linux-source: vendor/linux/fs/hfs/sysdep.c
//! test-origin: linux:vendor/linux/fs/hfs/sysdep.c
//! HFS dentry revalidation timezone adjustment.

use crate::include::uapi::errno::ECHILD;

pub const LOOKUP_RCU: u32 = 1 << 8;
pub const HFS_DENTRY_OPERATIONS_SYMBOL: &str = "hfs_dentry_operations";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsInodeTimes {
    pub ctime_sec: i64,
    pub atime_sec: i64,
    pub mtime_sec: i64,
    pub tz_secondswest: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsRevalidateOutcome {
    pub result: i32,
    pub inode: Option<HfsInodeTimes>,
    pub adjusted: bool,
}

pub fn hfs_revalidate_dentry_outcome(
    flags: u32,
    inode: Option<HfsInodeTimes>,
    sys_tz_minuteswest: i32,
) -> HfsRevalidateOutcome {
    if flags & LOOKUP_RCU != 0 {
        return HfsRevalidateOutcome {
            result: -ECHILD,
            inode,
            adjusted: false,
        };
    }

    let Some(mut inode) = inode else {
        return HfsRevalidateOutcome {
            result: 1,
            inode: None,
            adjusted: false,
        };
    };

    let diff = sys_tz_minuteswest * 60 - inode.tz_secondswest;
    if diff != 0 {
        let diff = i64::from(diff);
        inode.ctime_sec += diff;
        inode.atime_sec += diff;
        inode.mtime_sec += diff;
        inode.tz_secondswest += diff as i32;
    }

    HfsRevalidateOutcome {
        result: 1,
        inode: Some(inode),
        adjusted: diff != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hfs_revalidate_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hfs/sysdep.c"
        ));
        assert!(source.contains("#include <linux/namei.h>"));
        assert!(source.contains("#include \"hfs_fs.h\""));
        assert!(source.contains("static int hfs_revalidate_dentry"));
        assert!(source.contains("flags & LOOKUP_RCU"));
        assert!(source.contains("return -ECHILD;"));
        assert!(source.contains("inode = d_inode(dentry);"));
        assert!(source.contains("sys_tz.tz_minuteswest * 60"));
        assert!(source.contains("inode_set_ctime(inode, ts.tv_sec + diff"));
        assert!(source.contains("inode_set_atime(inode, ts.tv_sec + diff"));
        assert!(source.contains("inode_set_mtime(inode, ts.tv_sec + diff"));
        assert!(source.contains("HFS_I(inode)->tz_secondswest += diff;"));
        assert!(source.contains(HFS_DENTRY_OPERATIONS_SYMBOL));
        assert!(source.contains(".d_hash"));
        assert!(source.contains(".d_compare"));

        assert_eq!(
            hfs_revalidate_dentry_outcome(LOOKUP_RCU, None, 0).result,
            -ECHILD
        );
        assert_eq!(
            hfs_revalidate_dentry_outcome(0, None, 0),
            HfsRevalidateOutcome {
                result: 1,
                inode: None,
                adjusted: false,
            }
        );
        let inode = HfsInodeTimes {
            ctime_sec: 10,
            atime_sec: 20,
            mtime_sec: 30,
            tz_secondswest: 0,
        };
        let adjusted = hfs_revalidate_dentry_outcome(0, Some(inode), 60);
        assert_eq!(adjusted.inode.unwrap().ctime_sec, 3610);
        assert_eq!(adjusted.inode.unwrap().tz_secondswest, 3600);
        assert!(adjusted.adjusted);
    }
}
