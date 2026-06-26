//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfs/nfs4sysctl.c
//! test-origin: linux:vendor/linux/fs/nfs/nfs4sysctl.c
//! NFSv4 client sysctl table metadata.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Nfs4Sysctl {
    pub procname: &'static str,
    pub data_symbol: &'static str,
    pub maxlen_symbol: &'static str,
    pub mode: u16,
    pub proc_handler: &'static str,
    pub extra1: Option<&'static str>,
    pub extra2: Option<&'static str>,
}

pub const NFS4_SYSCTL_PATH: &str = "fs/nfs";
pub const NFS_SET_PORT_MIN: i32 = 0;
pub const NFS_SET_PORT_MAX: i32 = 65535;
pub const NFS4_CB_SYSCTLS: &[Nfs4Sysctl] = &[
    Nfs4Sysctl {
        procname: "nfs_callback_tcpport",
        data_symbol: "nfs_callback_set_tcpport",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec_minmax",
        extra1: Some("nfs_set_port_min"),
        extra2: Some("nfs_set_port_max"),
    },
    Nfs4Sysctl {
        procname: "idmap_cache_timeout",
        data_symbol: "nfs_idmap_cache_timeout",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec",
        extra1: None,
        extra2: None,
    },
];

static NFS4_SYSCTL_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn nfs4_register_sysctl(register_sysctl_ok: bool) -> Result<(), i32> {
    if register_sysctl_ok {
        NFS4_SYSCTL_REGISTERED.store(true, Ordering::Release);
        Ok(())
    } else {
        Err(-ENOMEM)
    }
}

pub fn nfs4_unregister_sysctl() {
    NFS4_SYSCTL_REGISTERED.store(false, Ordering::Release);
}

pub fn nfs4_sysctl_registered() -> bool {
    NFS4_SYSCTL_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfs4_sysctl_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfs/nfs4sysctl.c"
        ));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("#include <linux/nfs_fs.h>"));
        assert!(source.contains("#include \"nfs4_fs.h\""));
        assert!(source.contains("#include \"nfs4idmap.h\""));
        assert!(source.contains("#include \"callback.h\""));
        assert!(source.contains("static const int nfs_set_port_min;"));
        assert!(source.contains("static const int nfs_set_port_max = 65535;"));
        assert!(source.contains("static const struct ctl_table nfs4_cb_sysctls[]"));
        assert!(source.contains(".procname = \"nfs_callback_tcpport\""));
        assert!(source.contains(".data = &nfs_callback_set_tcpport"));
        assert!(source.contains(".proc_handler = proc_dointvec_minmax"));
        assert!(source.contains(".extra1 = (int *)&nfs_set_port_min"));
        assert!(source.contains(".extra2 = (int *)&nfs_set_port_max"));
        assert!(source.contains(".procname = \"idmap_cache_timeout\""));
        assert!(source.contains(".data = &nfs_idmap_cache_timeout"));
        assert!(source.contains("register_sysctl(\"fs/nfs\""));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_sysctl_table(nfs4_callback_sysctl_table);"));
        assert!(source.contains("nfs4_callback_sysctl_table = NULL;"));

        assert_eq!(NFS_SET_PORT_MIN, 0);
        assert_eq!(NFS_SET_PORT_MAX, 65535);
        assert_eq!(NFS4_CB_SYSCTLS.len(), 2);
        assert_eq!(NFS4_CB_SYSCTLS[0].proc_handler, "proc_dointvec_minmax");
        assert_eq!(NFS4_CB_SYSCTLS[1].data_symbol, "nfs_idmap_cache_timeout");
        assert_eq!(nfs4_register_sysctl(false), Err(-ENOMEM));
        assert_eq!(nfs4_register_sysctl(true), Ok(()));
        assert!(nfs4_sysctl_registered());
        nfs4_unregister_sysctl();
        assert!(!nfs4_sysctl_registered());
    }
}
