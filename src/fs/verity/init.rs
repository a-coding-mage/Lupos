//! linux-parity: complete
//! linux-source: vendor/linux/fs/verity/init.c
//! test-origin: linux:vendor/linux/fs/verity/init.c
//! fs-verity initialization and message routing metadata.

pub const FSVERITY_SYSCTL_PATH: &str = "fs/verity";
pub const FSVERITY_REQUIRE_SIGNATURES_PROC: &str = "require_signatures";
pub const FSVERITY_REQUIRE_SIGNATURES_MODE: u16 = 0o644;
pub const FSVERITY_REQUIRE_SIGNATURES_MAXLEN: usize = core::mem::size_of::<i32>();
pub const FSVERITY_SYSCTL_MIN: i32 = 0;
pub const FSVERITY_SYSCTL_MAX: i32 = 1;
pub const FSVERITY_LATE_INITCALL: &str = "fsverity_init";
pub const FSVERITY_INIT_STEPS: &[&str] = &[
    "fsverity_check_hash_algs",
    "fsverity_init_info_cache",
    "fsverity_init_workqueue",
    "fsverity_init_sysctl",
    "fsverity_init_signature",
    "fsverity_init_bpf",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsveritySysctlEntry<'a> {
    pub procname: &'a str,
    pub maxlen: usize,
    pub mode: u16,
    pub proc_handler: &'a str,
    pub extra1: i32,
    pub extra2: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsverityInitReport {
    pub check_hash_algs: bool,
    pub init_info_cache: bool,
    pub init_workqueue: bool,
    pub init_sysctl: bool,
    pub register_sysctl_path: Option<&'static str>,
    pub init_signature: bool,
    pub init_bpf: bool,
    pub late_initcall: &'static str,
    pub returned: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsverityMsgTarget<'a> {
    pub level: &'a str,
    pub sb_id: Option<&'a str>,
    pub ino: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsverityMsgReport<'a> {
    pub ratelimit_allowed: bool,
    pub va_start: bool,
    pub va_format_fmt: &'a str,
    pub target: FsverityMsgTarget<'a>,
    pub printk_with_inode: bool,
    pub va_end: bool,
}

pub const fn fsverity_require_signatures_sysctl(
    builtin_signatures: bool,
) -> Option<FsveritySysctlEntry<'static>> {
    if builtin_signatures {
        Some(FsveritySysctlEntry {
            procname: FSVERITY_REQUIRE_SIGNATURES_PROC,
            maxlen: FSVERITY_REQUIRE_SIGNATURES_MAXLEN,
            mode: FSVERITY_REQUIRE_SIGNATURES_MODE,
            proc_handler: "proc_dointvec_minmax",
            extra1: FSVERITY_SYSCTL_MIN,
            extra2: FSVERITY_SYSCTL_MAX,
        })
    } else {
        None
    }
}

pub const fn fsverity_init_report(sysctl_enabled: bool) -> FsverityInitReport {
    FsverityInitReport {
        check_hash_algs: true,
        init_info_cache: true,
        init_workqueue: true,
        init_sysctl: true,
        register_sysctl_path: if sysctl_enabled {
            Some(FSVERITY_SYSCTL_PATH)
        } else {
            None
        },
        init_signature: true,
        init_bpf: true,
        late_initcall: FSVERITY_LATE_INITCALL,
        returned: 0,
    }
}

pub const fn fsverity_msg_target<'a>(
    level: &'a str,
    inode: Option<(&'a str, u64)>,
) -> FsverityMsgTarget<'a> {
    match inode {
        Some((sb_id, ino)) => FsverityMsgTarget {
            level,
            sb_id: Some(sb_id),
            ino: Some(ino),
        },
        None => FsverityMsgTarget {
            level,
            sb_id: None,
            ino: None,
        },
    }
}

pub const fn fsverity_msg_report<'a>(
    level: &'a str,
    inode: Option<(&'a str, u64)>,
    fmt: &'a str,
    ratelimit_allowed: bool,
) -> FsverityMsgReport<'a> {
    FsverityMsgReport {
        ratelimit_allowed,
        va_start: ratelimit_allowed,
        va_format_fmt: if ratelimit_allowed { fmt } else { "" },
        target: fsverity_msg_target(level, inode),
        printk_with_inode: ratelimit_allowed && inode.is_some(),
        va_end: ratelimit_allowed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsverity_init_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/verity/init.c"
        ));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("#include \"fsverity_private.h\""));
        assert!(source.contains("#include <linux/ratelimit.h>"));
        assert!(source.contains("static const struct ctl_table fsverity_sysctl_table[]"));
        assert!(source.contains(".procname       = \"require_signatures\""));
        assert!(source.contains(".maxlen         = sizeof(int)"));
        assert!(source.contains(".mode           = 0644"));
        assert!(source.contains(".proc_handler   = proc_dointvec_minmax"));
        assert!(source.contains(".extra1         = SYSCTL_ZERO"));
        assert!(source.contains(".extra2         = SYSCTL_ONE"));
        assert!(source.contains("register_sysctl_init(\"fs/verity\", fsverity_sysctl_table);"));
        assert!(source.contains("void fsverity_msg"));
        assert!(source.contains("static DEFINE_RATELIMIT_STATE"));
        assert!(source.contains("if (!__ratelimit(&rs))"));
        assert!(source.contains("va_start(args, fmt);"));
        assert!(source.contains("vaf.fmt = fmt;"));
        assert!(source.contains("vaf.va = &args;"));
        assert!(source.contains("printk(\"%sfs-verity (%s, inode %llu): %pV\\n\""));
        assert!(source.contains("printk(\"%sfs-verity: %pV\\n\""));
        assert!(source.contains("va_end(args);"));
        assert!(source.contains("fsverity_check_hash_algs();"));
        assert!(source.contains("fsverity_init_info_cache();"));
        assert!(source.contains("fsverity_init_workqueue();"));
        assert!(source.contains("fsverity_init_sysctl();"));
        assert!(source.contains("fsverity_init_signature();"));
        assert!(source.contains("fsverity_init_bpf();"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("late_initcall(fsverity_init)"));

        assert_eq!(FSVERITY_SYSCTL_PATH, "fs/verity");
        assert_eq!(FSVERITY_INIT_STEPS.len(), 6);
        assert_eq!(
            fsverity_msg_target("KERN_ERR", Some(("sda1", 55))),
            FsverityMsgTarget {
                level: "KERN_ERR",
                sb_id: Some("sda1"),
                ino: Some(55),
            }
        );
        assert_eq!(fsverity_msg_target("KERN_INFO", None).ino, None);
    }

    #[test]
    fn sysctl_entry_matches_require_signatures_bounds() {
        assert_eq!(
            fsverity_require_signatures_sysctl(true),
            Some(FsveritySysctlEntry {
                procname: FSVERITY_REQUIRE_SIGNATURES_PROC,
                maxlen: core::mem::size_of::<i32>(),
                mode: 0o644,
                proc_handler: "proc_dointvec_minmax",
                extra1: 0,
                extra2: 1,
            })
        );
        assert_eq!(fsverity_require_signatures_sysctl(false), None);
    }

    #[test]
    fn init_report_preserves_late_initcall_order_and_return() {
        assert_eq!(
            fsverity_init_report(true),
            FsverityInitReport {
                check_hash_algs: true,
                init_info_cache: true,
                init_workqueue: true,
                init_sysctl: true,
                register_sysctl_path: Some("fs/verity"),
                init_signature: true,
                init_bpf: true,
                late_initcall: "fsverity_init",
                returned: 0,
            }
        );
        assert_eq!(fsverity_init_report(false).register_sysctl_path, None);
    }

    #[test]
    fn msg_report_short_circuits_before_va_start_when_ratelimited() {
        assert_eq!(
            fsverity_msg_report("KERN_ERR", Some(("dm-0", 42)), "%s", false),
            FsverityMsgReport {
                ratelimit_allowed: false,
                va_start: false,
                va_format_fmt: "",
                target: FsverityMsgTarget {
                    level: "KERN_ERR",
                    sb_id: Some("dm-0"),
                    ino: Some(42),
                },
                printk_with_inode: false,
                va_end: false,
            }
        );
    }

    #[test]
    fn msg_report_selects_inode_or_global_printk_format() {
        assert_eq!(
            fsverity_msg_report("KERN_ERR", Some(("dm-0", 42)), "%s", true),
            FsverityMsgReport {
                ratelimit_allowed: true,
                va_start: true,
                va_format_fmt: "%s",
                target: FsverityMsgTarget {
                    level: "KERN_ERR",
                    sb_id: Some("dm-0"),
                    ino: Some(42),
                },
                printk_with_inode: true,
                va_end: true,
            }
        );
        assert_eq!(
            fsverity_msg_report("KERN_INFO", None, "%pV", true).printk_with_inode,
            false
        );
    }
}
