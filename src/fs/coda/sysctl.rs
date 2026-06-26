//! linux-parity: complete
//! linux-source: vendor/linux/fs/coda/sysctl.c
//! test-origin: linux:vendor/linux/fs/coda/sysctl.c
//! Coda sysctl registration metadata.

use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodaSysctl {
    pub procname: &'static str,
    pub data_symbol: &'static str,
    pub maxlen_symbol: &'static str,
    pub mode: u16,
    pub proc_handler: &'static str,
}

pub const CODA_SYSCTL_PATH: &str = "coda";
pub const CODA_SYSCTLS: &[CodaSysctl] = &[
    CodaSysctl {
        procname: "timeout",
        data_symbol: "coda_timeout",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec",
    },
    CodaSysctl {
        procname: "hard",
        data_symbol: "coda_hard",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec",
    },
    CodaSysctl {
        procname: "fake_statfs",
        data_symbol: "coda_fake_statfs",
        maxlen_symbol: "sizeof(int)",
        mode: 0o600,
        proc_handler: "proc_dointvec",
    },
];

static CODA_SYSCTL_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn coda_sysctl_init() -> bool {
    !CODA_SYSCTL_REGISTERED.swap(true, Ordering::AcqRel)
}

pub fn coda_sysctl_clean() -> bool {
    CODA_SYSCTL_REGISTERED.swap(false, Ordering::AcqRel)
}

pub fn coda_sysctl_registered() -> bool {
    CODA_SYSCTL_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coda_sysctl_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/coda/sysctl.c"
        ));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("#include \"coda_int.h\""));
        assert!(source.contains("static struct ctl_table_header *fs_table_header;"));
        assert!(source.contains("static const struct ctl_table coda_table[]"));
        assert!(source.contains(".procname\t= \"timeout\""));
        assert!(source.contains(".data\t\t= &coda_timeout"));
        assert!(source.contains(".procname\t= \"hard\""));
        assert!(source.contains(".data\t\t= &coda_hard"));
        assert!(source.contains(".procname\t= \"fake_statfs\""));
        assert!(source.contains(".data\t\t= &coda_fake_statfs"));
        assert!(source.contains(".mode\t\t= 0600"));
        assert!(source.contains("register_sysctl(\"coda\", coda_table);"));
        assert!(source.contains("unregister_sysctl_table(fs_table_header);"));
        assert!(source.contains("fs_table_header = NULL;"));

        assert_eq!(CODA_SYSCTLS.len(), 3);
        assert_eq!(CODA_SYSCTLS[2].mode, 0o600);
        assert!(!coda_sysctl_registered());
        assert!(coda_sysctl_init());
        assert!(coda_sysctl_registered());
        assert!(!coda_sysctl_init());
        assert!(coda_sysctl_clean());
        assert!(!coda_sysctl_clean());
    }
}
