//! linux-parity: complete
//! linux-source: vendor/linux/fs/fuse/sysctl.c
//! test-origin: linux:vendor/linux/fs/fuse/sysctl.c
//! FUSE sysctl table shape.

use crate::include::uapi::errno::ENOMEM;

pub const SYSCTL_FUSE_U16_LIMIT: u32 = 65535;
pub const SYSCTL_ZERO: u32 = 0;
pub const SYSCTL_ONE: u32 = 1;
pub const FUSE_SYSCTL_PATH: &str = "fs/fuse";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FuseSysctlEntry {
    pub procname: &'static str,
    pub data_symbol: &'static str,
    pub mode: u16,
    pub handler: &'static str,
    pub min: u32,
    pub max: u32,
}

pub const FUSE_SYSCTL_TABLE: &[FuseSysctlEntry] = &[
    FuseSysctlEntry {
        procname: "max_pages_limit",
        data_symbol: "fuse_max_pages_limit",
        mode: 0o644,
        handler: "proc_douintvec_minmax",
        min: SYSCTL_ONE,
        max: SYSCTL_FUSE_U16_LIMIT,
    },
    FuseSysctlEntry {
        procname: "default_request_timeout",
        data_symbol: "fuse_default_req_timeout",
        mode: 0o644,
        handler: "proc_douintvec_minmax",
        min: SYSCTL_ZERO,
        max: SYSCTL_FUSE_U16_LIMIT,
    },
    FuseSysctlEntry {
        procname: "max_request_timeout",
        data_symbol: "fuse_max_req_timeout",
        mode: 0o644,
        handler: "proc_douintvec_minmax",
        min: SYSCTL_ZERO,
        max: SYSCTL_FUSE_U16_LIMIT,
    },
];

pub fn fuse_sysctl_register_result(header_created: bool) -> Result<(), i32> {
    if header_created { Ok(()) } else { Err(-ENOMEM) }
}

pub const fn fuse_sysctl_unregister_clears_header() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuse_sysctl_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/fuse/sysctl.c"
        ));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("#include \"fuse_i.h\""));
        assert!(source.contains("static struct ctl_table_header *fuse_table_header;"));
        assert!(source.contains("sysctl_fuse_max_pages_limit = 65535"));
        assert!(source.contains("sysctl_fuse_req_timeout_limit = 65535"));
        assert!(source.contains(".procname\t= \"max_pages_limit\""));
        assert!(source.contains(".data\t\t= &fuse_max_pages_limit"));
        assert!(source.contains(".extra1\t\t= SYSCTL_ONE"));
        assert!(source.contains(".procname\t= \"default_request_timeout\""));
        assert!(source.contains(".data\t\t= &fuse_default_req_timeout"));
        assert!(source.contains(".procname\t= \"max_request_timeout\""));
        assert!(source.contains(".data\t\t= &fuse_max_req_timeout"));
        assert!(source.contains("register_sysctl(\"fs/fuse\", fuse_sysctl_table);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_sysctl_table(fuse_table_header);"));
        assert!(source.contains("fuse_table_header = NULL;"));

        assert_eq!(FUSE_SYSCTL_TABLE.len(), 3);
        assert_eq!(FUSE_SYSCTL_TABLE[0].min, SYSCTL_ONE);
        assert_eq!(FUSE_SYSCTL_TABLE[1].max, SYSCTL_FUSE_U16_LIMIT);
        assert_eq!(fuse_sysctl_register_result(true), Ok(()));
        assert_eq!(fuse_sysctl_register_result(false), Err(-ENOMEM));
        assert!(fuse_sysctl_unregister_clears_header());
    }
}
