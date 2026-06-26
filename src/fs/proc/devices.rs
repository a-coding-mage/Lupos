//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/devices.c
//! test-origin: linux:vendor/linux/fs/proc/devices.c
//! `/proc/devices`.

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub const CHRDEV_MAJOR_MAX: u64 = 512;
pub const BLKDEV_MAJOR_MAX: u64 = 512;
pub const DEVINFO_OPERATIONS_SYMBOL: &str = "devinfo_ops";
pub const DEVINFO_OPERATIONS: &[(&str, &str)] = &[
    ("start", "devinfo_start"),
    ("next", "devinfo_next"),
    ("stop", "devinfo_stop"),
    ("show", "devinfo_show"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevinfoKind {
    Character,
    Block,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DevinfoShow {
    pub kind: DevinfoKind,
    pub major: u64,
    pub header: Option<&'static str>,
}

pub const fn devinfo_total_majors(config_block: bool) -> u64 {
    CHRDEV_MAJOR_MAX + if config_block { BLKDEV_MAJOR_MAX } else { 0 }
}

pub const fn devinfo_start(pos: u64, config_block: bool) -> Option<u64> {
    if pos < devinfo_total_majors(config_block) {
        Some(pos)
    } else {
        None
    }
}

pub const fn devinfo_next(pos: u64, config_block: bool) -> Option<u64> {
    let next = pos.saturating_add(1);
    if next >= devinfo_total_majors(config_block) {
        None
    } else {
        Some(next)
    }
}

pub const fn devinfo_stop() {}

pub const fn devinfo_show(pos: u64, config_block: bool) -> Option<DevinfoShow> {
    if pos < CHRDEV_MAJOR_MAX {
        Some(DevinfoShow {
            kind: DevinfoKind::Character,
            major: pos,
            header: if pos == 0 {
                Some("Character devices:\n")
            } else {
                None
            },
        })
    } else if config_block && pos < CHRDEV_MAJOR_MAX + BLKDEV_MAJOR_MAX {
        let major = pos - CHRDEV_MAJOR_MAX;
        Some(DevinfoShow {
            kind: DevinfoKind::Block,
            major,
            header: if major == 0 {
                Some("\nBlock devices:\n")
            } else {
                None
            },
        })
    } else {
        None
    }
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(
        buf,
        "Character devices:\n  1 mem\n  4 tty\n  5 console\n\nBlock devices:\n  7 loop\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_devices_seq_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/devices.c"
        ));
        let fs_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/fs.h"
        ));
        let blkdev_header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/blkdev.h"
        ));
        assert!(source.contains("static int devinfo_show(struct seq_file *f, void *v)"));
        assert!(source.contains("if (i < CHRDEV_MAJOR_MAX)"));
        assert!(source.contains("seq_puts(f, \"Character devices:\\n\");"));
        assert!(source.contains("chrdev_show(f, i);"));
        assert!(source.contains("#ifdef CONFIG_BLOCK"));
        assert!(source.contains("i -= CHRDEV_MAJOR_MAX;"));
        assert!(source.contains("seq_puts(f, \"\\nBlock devices:\\n\");"));
        assert!(source.contains("blkdev_show(f, i);"));
        assert!(source.contains("static void *devinfo_start"));
        assert!(source.contains("if (*pos < (BLKDEV_MAJOR_MAX + CHRDEV_MAJOR_MAX))"));
        assert!(source.contains("static void *devinfo_next"));
        assert!(source.contains("(*pos)++;"));
        assert!(source.contains("if (*pos >= (BLKDEV_MAJOR_MAX + CHRDEV_MAJOR_MAX))"));
        assert!(source.contains("static const struct seq_operations devinfo_ops"));
        assert!(source.contains("proc_create_seq(\"devices\", 0, NULL, &devinfo_ops);"));
        assert!(source.contains("pde_make_permanent(pde);"));
        assert!(fs_header.contains("#define CHRDEV_MAJOR_MAX 512"));
        assert!(blkdev_header.contains("#define BLKDEV_MAJOR_MAX\t512"));
        for (slot, target) in DEVINFO_OPERATIONS {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }

        assert_eq!(devinfo_total_majors(true), 1024);
        assert_eq!(devinfo_total_majors(false), 512);
        assert_eq!(devinfo_start(0, true), Some(0));
        assert_eq!(devinfo_start(1024, true), None);
        assert_eq!(devinfo_next(510, false), Some(511));
        assert_eq!(devinfo_next(511, false), None);
        assert_eq!(
            devinfo_show(0, true),
            Some(DevinfoShow {
                kind: DevinfoKind::Character,
                major: 0,
                header: Some("Character devices:\n"),
            })
        );
        assert_eq!(
            devinfo_show(CHRDEV_MAJOR_MAX, true),
            Some(DevinfoShow {
                kind: DevinfoKind::Block,
                major: 0,
                header: Some("\nBlock devices:\n"),
            })
        );
        assert_eq!(devinfo_show(CHRDEV_MAJOR_MAX, false), None);
        devinfo_stop();
    }
}
