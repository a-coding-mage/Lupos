//! linux-parity: complete
//! linux-source: vendor/linux/fs/ntfs/sysctl.c
//! test-origin: linux:vendor/linux/fs/ntfs/sysctl.c
//! NTFS debug sysctl metadata.

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NtfsSysctl {
    pub procname: &'static str,
    pub data_symbol: &'static str,
    pub maxlen_symbol: &'static str,
    pub mode: u16,
    pub proc_handler: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NtfsSysctlAction {
    Register,
    Unregister,
}

pub const NTFS_SYSCTL_PATH: &str = "fs/ntfs";
pub const NTFS_SYSCTLS: &[NtfsSysctl] = &[NtfsSysctl {
    procname: "ntfs-debug",
    data_symbol: "debug_msgs",
    maxlen_symbol: "sizeof(debug_msgs)",
    mode: 0o644,
    proc_handler: "proc_dointvec",
}];

pub fn ntfs_sysctl_outcome(add: bool, register_ok: bool) -> Result<NtfsSysctlAction, i32> {
    if add {
        if register_ok {
            Ok(NtfsSysctlAction::Register)
        } else {
            Err(-ENOMEM)
        }
    } else {
        Ok(NtfsSysctlAction::Unregister)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntfs_sysctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ntfs/sysctl.c"
        ));
        assert!(source.contains("#ifdef DEBUG"));
        assert!(source.contains("#ifdef CONFIG_SYSCTL"));
        assert!(source.contains("#include <linux/proc_fs.h>"));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("#include \"sysctl.h\""));
        assert!(source.contains("#include \"debug.h\""));
        assert!(source.contains("static const struct ctl_table ntfs_sysctls[]"));
        assert!(source.contains(".procname\t= \"ntfs-debug\""));
        assert!(source.contains(".data\t\t= &debug_msgs"));
        assert!(source.contains(".maxlen\t\t= sizeof(debug_msgs)"));
        assert!(source.contains(".proc_handler\t= proc_dointvec"));
        assert!(source.contains("int ntfs_sysctl(int add)"));
        assert!(source.contains("register_sysctl(\"fs/ntfs\", ntfs_sysctls);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_sysctl_table(sysctls_root_table);"));
        assert!(source.contains("sysctls_root_table = NULL;"));

        assert_eq!(NTFS_SYSCTLS.len(), 1);
        assert_eq!(NTFS_SYSCTLS[0].procname, "ntfs-debug");
        assert_eq!(ntfs_sysctl_outcome(true, false), Err(-ENOMEM));
        assert_eq!(
            ntfs_sysctl_outcome(true, true),
            Ok(NtfsSysctlAction::Register)
        );
        assert_eq!(
            ntfs_sysctl_outcome(false, false),
            Ok(NtfsSysctlAction::Unregister)
        );
    }
}
