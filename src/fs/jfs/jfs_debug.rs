//! linux-parity: complete
//! linux-source: vendor/linux/fs/jfs/jfs_debug.c
//! test-origin: linux:vendor/linux/fs/jfs/jfs_debug.c
//! JFS proc debug setup and loglevel write behavior.

use crate::include::uapi::errno::{EFAULT, EINVAL};

pub const JFS_PROC_PATH: &str = "fs/jfs";
pub const JFS_STATISTICS_PROC_ENTRIES: &[&str] = &["lmstats", "txstats", "xtstat", "mpstat"];
pub const JFS_DEBUG_PROC_ENTRIES: &[&str] = &["TxAnchor", "loglevel"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JfsProcInitPlan {
    pub create_base: bool,
    pub statistics_entries: &'static [&'static str],
    pub debug_entries: &'static [&'static str],
}

pub const fn jfs_proc_init_plan(
    proc_mkdir_ok: bool,
    statistics_enabled: bool,
    debug_enabled: bool,
) -> JfsProcInitPlan {
    JfsProcInitPlan {
        create_base: proc_mkdir_ok,
        statistics_entries: if proc_mkdir_ok && statistics_enabled {
            JFS_STATISTICS_PROC_ENTRIES
        } else {
            &[]
        },
        debug_entries: if proc_mkdir_ok && debug_enabled {
            JFS_DEBUG_PROC_ENTRIES
        } else {
            &[]
        },
    }
}

pub const fn jfs_loglevel_proc_write(byte: Option<u8>, count: usize) -> Result<usize, i32> {
    let Some(c) = byte else {
        return Err(-EFAULT);
    };
    if c < b'0' || c > b'9' {
        return Err(-EINVAL);
    }
    Ok(count)
}

pub const fn jfs_loglevel_from_byte(byte: u8) -> Option<u8> {
    if byte >= b'0' && byte <= b'9' {
        Some(byte - b'0')
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jfs_debug_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jfs/jfs_debug.c"
        ));
        assert!(source.contains("#include <linux/proc_fs.h>"));
        assert!(source.contains("#include \"jfs_debug.h\""));
        assert!(source.contains("#ifdef PROC_FS_JFS"));
        assert!(source.contains("jfs_loglevel_proc_show"));
        assert!(source.contains("seq_printf(m, \"%d\\n\", jfsloglevel);"));
        assert!(source.contains("if (get_user(c, buffer))"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("if (c < '0' || c > '9')"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("jfsloglevel = c - '0';"));
        assert!(source.contains("proc_mkdir(\"fs/jfs\", NULL);"));
        assert!(source.contains("proc_create_single(\"lmstats\""));
        assert!(source.contains("proc_create(\"loglevel\""));
        assert!(source.contains("remove_proc_subtree(\"fs/jfs\", NULL);"));

        assert_eq!(jfs_loglevel_proc_write(Some(b'7'), 3), Ok(3));
        assert_eq!(jfs_loglevel_from_byte(b'7'), Some(7));
        assert_eq!(jfs_loglevel_proc_write(Some(b'x'), 3), Err(-EINVAL));
        assert_eq!(jfs_loglevel_proc_write(None, 3), Err(-EFAULT));
        let plan = jfs_proc_init_plan(true, true, true);
        assert_eq!(plan.statistics_entries.len(), 4);
        assert_eq!(plan.debug_entries, JFS_DEBUG_PROC_ENTRIES);
        assert!(
            jfs_proc_init_plan(false, true, true)
                .debug_entries
                .is_empty()
        );
    }
}
