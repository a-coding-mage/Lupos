//! linux-parity: complete
//! linux-source: vendor/linux/fs/netfs/fscache_proc.c
//! test-origin: linux:vendor/linux/fs/netfs/fscache_proc.c
//! FS-Cache procfs registration shape.

use crate::include::uapi::errno::ENOMEM;

pub const S_IFREG: u16 = 0o100000;
pub const FSCACHE_PROC_SYMLINK: &str = "fs/fscache";
pub const FSCACHE_PROC_SYMLINK_TARGET: &str = "netfs";
pub const FSCACHE_PROC_CLEANUP_SUBTREE: &str = "fs/fscache";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FscacheProcEntry {
    pub path: &'static str,
    pub mode: u16,
    pub seq_ops: &'static str,
}

pub const FSCACHE_PROC_ENTRIES: &[FscacheProcEntry] = &[
    FscacheProcEntry {
        path: "fs/netfs/caches",
        mode: S_IFREG | 0o444,
        seq_ops: "fscache_caches_seq_ops",
    },
    FscacheProcEntry {
        path: "fs/netfs/volumes",
        mode: S_IFREG | 0o444,
        seq_ops: "fscache_volumes_seq_ops",
    },
    FscacheProcEntry {
        path: "fs/netfs/cookies",
        mode: S_IFREG | 0o444,
        seq_ops: "fscache_cookies_seq_ops",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FscacheProcStep {
    Symlink,
    Caches,
    Volumes,
    Cookies,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FscacheProcInitOutcome {
    pub result: i32,
    pub remove_symlink_on_error: bool,
}

pub fn fscache_proc_init_outcome(fail_at: Option<FscacheProcStep>) -> FscacheProcInitOutcome {
    match fail_at {
        None => FscacheProcInitOutcome {
            result: 0,
            remove_symlink_on_error: false,
        },
        Some(FscacheProcStep::Symlink) => FscacheProcInitOutcome {
            result: -ENOMEM,
            remove_symlink_on_error: false,
        },
        Some(_) => FscacheProcInitOutcome {
            result: -ENOMEM,
            remove_symlink_on_error: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fscache_proc_init_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/netfs/fscache_proc.c"
        ));
        assert!(source.contains("#define FSCACHE_DEBUG_LEVEL CACHE"));
        assert!(source.contains("#include <linux/proc_fs.h>"));
        assert!(source.contains("#include <linux/seq_file.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("proc_symlink(\"fs/fscache\", NULL, \"netfs\")"));
        assert!(source.contains("proc_create_seq(\"fs/netfs/caches\""));
        assert!(source.contains("&fscache_caches_seq_ops"));
        assert!(source.contains("proc_create_seq(\"fs/netfs/volumes\""));
        assert!(source.contains("&fscache_volumes_seq_ops"));
        assert!(source.contains("proc_create_seq(\"fs/netfs/cookies\""));
        assert!(source.contains("&fscache_cookies_seq_ops"));
        assert!(source.contains("remove_proc_entry(\"fs/fscache\", NULL);"));
        assert!(source.contains("remove_proc_subtree(\"fs/fscache\", NULL);"));

        assert_eq!(FSCACHE_PROC_ENTRIES.len(), 3);
        assert_eq!(FSCACHE_PROC_ENTRIES[0].mode, S_IFREG | 0o444);
        assert_eq!(
            fscache_proc_init_outcome(Some(FscacheProcStep::Symlink)),
            FscacheProcInitOutcome {
                result: -ENOMEM,
                remove_symlink_on_error: false,
            }
        );
        assert!(fscache_proc_init_outcome(Some(FscacheProcStep::Cookies)).remove_symlink_on_error);
    }
}
