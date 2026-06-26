//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs/sysctl.c
//! test-origin: linux:vendor/linux/fs/nfs/sysctl.c
//! NFS client sysctl table metadata.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfsSysctl {
    pub procname: &'static str,
    pub data_symbol: &'static str,
    pub maxlen_symbol: &'static str,
    pub mode: u16,
    pub proc_handler: &'static str,
}

pub const NFS_SYSCTL_PATH: &str = "fs/nfs";
pub const NFS_CB_SYSCTLS: &[NfsSysctl] = &[
    NfsSysctl {
        procname: "nfs_mountpoint_timeout",
        data_symbol: "nfs_mountpoint_expiry_timeout",
        maxlen_symbol: "sizeof(nfs_mountpoint_expiry_timeout)",
        mode: 0o644,
        proc_handler: "proc_dointvec_jiffies",
    },
    NfsSysctl {
        procname: "nfs_congestion_kb",
        data_symbol: "nfs_congestion_kb",
        maxlen_symbol: "sizeof(nfs_congestion_kb)",
        mode: 0o644,
        proc_handler: "proc_dointvec",
    },
];

static NFS_SYSCTL_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn nfs_register_sysctl(register_sysctl_ok: bool) -> Result<(), i32> {
    if register_sysctl_ok {
        NFS_SYSCTL_REGISTERED.store(true, Ordering::Release);
        Ok(())
    } else {
        Err(-ENOMEM)
    }
}

pub fn nfs_unregister_sysctl() {
    NFS_SYSCTL_REGISTERED.store(false, Ordering::Release);
}

pub fn nfs_sysctl_registered() -> bool {
    NFS_SYSCTL_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs_sysctl_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfs/sysctl.c"
        ));
        assert!(source.contains("#include <linux/types.h>"));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("#include <linux/nfs_fs.h>"));
        assert!(source.contains("static struct ctl_table_header *nfs_callback_sysctl_table;"));
        assert!(source.contains("static const struct ctl_table nfs_cb_sysctls[]"));
        assert!(source.contains(".procname\t= \"nfs_mountpoint_timeout\""));
        assert!(source.contains(".data\t\t= &nfs_mountpoint_expiry_timeout"));
        assert!(source.contains(".proc_handler\t= proc_dointvec_jiffies"));
        assert!(source.contains(".procname\t= \"nfs_congestion_kb\""));
        assert!(source.contains(".data\t\t= &nfs_congestion_kb"));
        assert!(source.contains(".proc_handler\t= proc_dointvec"));
        assert!(source.contains("register_sysctl(\"fs/nfs\", nfs_cb_sysctls);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_sysctl_table(nfs_callback_sysctl_table);"));
        assert!(source.contains("nfs_callback_sysctl_table = NULL;"));

        assert_eq!(NFS_CB_SYSCTLS.len(), 2);
        assert_eq!(NFS_CB_SYSCTLS[0].procname, "nfs_mountpoint_timeout");
        assert_eq!(NFS_CB_SYSCTLS[0].proc_handler, "proc_dointvec_jiffies");
        assert_eq!(NFS_CB_SYSCTLS[1].procname, "nfs_congestion_kb");
        assert_eq!(NFS_SYSCTL_PATH, "fs/nfs");
        assert_eq!(nfs_register_sysctl(false), Err(-ENOMEM));
        assert_eq!(nfs_register_sysctl(true), Ok(()));
        assert!(nfs_sysctl_registered());
        nfs_unregister_sysctl();
        assert!(!nfs_sysctl_registered());
    }
}
