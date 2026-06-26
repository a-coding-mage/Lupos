//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2/ioctl.c
//! test-origin: linux:vendor/linux/fs/jffs2/ioctl.c
//! JFFS2 ioctl fallback.

use crate::include::uapi::errno::ENOTTY;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2File {
    pub inode: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2IoctlRequest {
    pub file: Option<Jffs2File>,
    pub cmd: u32,
    pub arg: usize,
}

pub const JFFS2_IOCTL_FUTURE_USE: &str = "lsattr.jffs2/chattr.jffs2 compression support";

pub const fn jffs2_ioctl(_request: Jffs2IoctlRequest) -> i32 {
    -ENOTTY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jffs2_ioctl_matches_linux_enotty_stub() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jffs2/ioctl.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include \"nodelist.h\""));
        assert!(
            source.contains(
                "long jffs2_ioctl(struct file *filp, unsigned int cmd, unsigned long arg)"
            )
        );
        assert!(source.contains("lsattr.jffs2 and chattr.jffs2"));
        assert!(source.contains("compression support"));
        assert!(source.contains("return -ENOTTY;"));

        let request = Jffs2IoctlRequest {
            file: Some(Jffs2File { inode: 7 }),
            cmd: 0x1234,
            arg: 0xfeed,
        };
        assert_eq!(
            JFFS2_IOCTL_FUTURE_USE,
            "lsattr.jffs2/chattr.jffs2 compression support"
        );
        assert_eq!(jffs2_ioctl(request), -ENOTTY);
        assert_eq!(
            jffs2_ioctl(Jffs2IoctlRequest {
                file: None,
                cmd: 0,
                arg: 0,
            }),
            -ENOTTY
        );
    }
}
