//! linux-parity: complete
//! linux-source: vendor/linux/fs/sysctls.c
//! test-origin: linux:vendor/linux/fs/sysctls.c
//! Shared `/proc/sys/fs` sysctl table entries.

pub const SYSCTL_ZERO: i32 = 0;
pub const SYSCTL_MAXOLDUID: i32 = 65_535;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsSharedSysctl {
    pub procname: &'static str,
    pub data_symbol: &'static str,
    pub maxlen: usize,
    pub mode: u16,
    pub proc_handler: &'static str,
    pub min: i32,
    pub max: i32,
}

pub const FS_SHARED_SYSCTLS: &[FsSharedSysctl] = &[
    FsSharedSysctl {
        procname: "overflowuid",
        data_symbol: "fs_overflowuid",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        proc_handler: "proc_dointvec_minmax",
        min: SYSCTL_ZERO,
        max: SYSCTL_MAXOLDUID,
    },
    FsSharedSysctl {
        procname: "overflowgid",
        data_symbol: "fs_overflowgid",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        proc_handler: "proc_dointvec_minmax",
        min: SYSCTL_ZERO,
        max: SYSCTL_MAXOLDUID,
    },
];

pub const fn fs_shared_sysctl_accepts(value: i32) -> bool {
    value >= SYSCTL_ZERO && value <= SYSCTL_MAXOLDUID
}

pub const fn fs_shared_sysctl_registration_path() -> &'static str {
    "fs"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_shared_sysctls_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/sysctls.c"
        ));
        assert!(source.contains("static const struct ctl_table fs_shared_sysctls[]"));
        assert!(source.contains(".procname\t= \"overflowuid\""));
        assert!(source.contains(".data\t\t= &fs_overflowuid"));
        assert!(source.contains(".procname\t= \"overflowgid\""));
        assert!(source.contains(".data\t\t= &fs_overflowgid"));
        assert!(source.contains(".proc_handler\t= proc_dointvec_minmax"));
        assert!(source.contains(".extra1\t\t= SYSCTL_ZERO"));
        assert!(source.contains(".extra2\t\t= SYSCTL_MAXOLDUID"));
        assert!(source.contains("register_sysctl_init(\"fs\", fs_shared_sysctls);"));
        assert!(source.contains("early_initcall(init_fs_sysctls);"));

        assert_eq!(FS_SHARED_SYSCTLS.len(), 2);
        assert_eq!(FS_SHARED_SYSCTLS[0].procname, "overflowuid");
        assert_eq!(FS_SHARED_SYSCTLS[1].procname, "overflowgid");
        assert!(fs_shared_sysctl_accepts(0));
        assert!(fs_shared_sysctl_accepts(65_535));
        assert!(!fs_shared_sysctl_accepts(-1));
        assert!(!fs_shared_sysctl_accepts(65_536));
        assert_eq!(fs_shared_sysctl_registration_path(), "fs");
    }
}
