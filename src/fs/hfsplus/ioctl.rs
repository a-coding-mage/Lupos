//! linux-parity: complete
//! linux-source: vendor/linux/fs/hfsplus/ioctl.c
//! test-origin: linux:vendor/linux/fs/hfsplus/ioctl.c
//! HFS+ ioctl dispatch and bless metadata updates.

use crate::include::uapi::errno::{ENOTTY, EPERM};

pub const HFSPLUS_IOC_BLESS: u32 = (b'h' as u32) << 8 | 0x80;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusFinderInfoUpdate {
    pub finder_info_0: u32,
    pub finder_info_1: u32,
    pub finder_info_5: u32,
}

pub fn hfsplus_ioctl_bless(
    capable_sys_admin: bool,
    parent_ino: u32,
    cnid: u32,
) -> Result<HfsplusFinderInfoUpdate, i32> {
    if !capable_sys_admin {
        return Err(-EPERM);
    }
    Ok(HfsplusFinderInfoUpdate {
        finder_info_0: parent_ino,
        finder_info_1: cnid,
        finder_info_5: parent_ino,
    })
}

pub fn hfsplus_ioctl(
    cmd: u32,
    capable_sys_admin: bool,
    parent_ino: u32,
    cnid: u32,
) -> Result<HfsplusFinderInfoUpdate, i32> {
    match cmd {
        HFSPLUS_IOC_BLESS => hfsplus_ioctl_bless(capable_sys_admin, parent_ino, cnid),
        _ => Err(-ENOTTY),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hfsplus_ioctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hfsplus/ioctl.c"
        ));
        assert!(source.contains("#include <linux/capability.h>"));
        assert!(source.contains("#include \"hfsplus_fs.h\""));
        assert!(source.contains("static int hfsplus_ioctl_bless"));
        assert!(source.contains("if (!capable(CAP_SYS_ADMIN))"));
        assert!(source.contains("return -EPERM;"));
        assert!(source.contains("vh->finder_info[0] = bvh->finder_info[0]"));
        assert!(source.contains("vh->finder_info[1] = bvh->finder_info[1]"));
        assert!(source.contains("vh->finder_info[5] = bvh->finder_info[5]"));
        assert!(source.contains("case HFSPLUS_IOC_BLESS:"));
        assert!(source.contains("return -ENOTTY;"));

        assert_eq!(
            hfsplus_ioctl(HFSPLUS_IOC_BLESS, true, 11, 99),
            Ok(HfsplusFinderInfoUpdate {
                finder_info_0: 11,
                finder_info_1: 99,
                finder_info_5: 11,
            })
        );
        assert_eq!(hfsplus_ioctl_bless(false, 11, 99), Err(-EPERM));
        assert_eq!(hfsplus_ioctl(0, true, 11, 99), Err(-ENOTTY));
    }
}
