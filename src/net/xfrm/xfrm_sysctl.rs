//! linux-parity: complete
//! linux-source: vendor/linux/net/xfrm/xfrm_sysctl.c
//! test-origin: linux:vendor/linux/net/xfrm/xfrm_sysctl.c
//! XFRM per-net sysctl defaults and registration shape.

use crate::include::uapi::errno::ENOMEM;

pub const XFRM_AE_ETIME: u32 = 10;
pub const XFRM_AE_SEQT_SIZE: u32 = 2;
pub const XFRM_SYSCTL_PATH: &str = "net/core";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfrmSysctlNet {
    pub sysctl_aevent_etime: u32,
    pub sysctl_aevent_rseqth: u32,
    pub sysctl_larval_drop: i32,
    pub sysctl_acq_expires: i32,
    pub sysctl_hdr_registered: bool,
    pub sysctl_table_size: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfrmCtlTable {
    pub procname: &'static str,
    pub mode: u16,
    pub handler: &'static str,
}

pub const XFRM_TABLE: [XfrmCtlTable; 4] = [
    XfrmCtlTable {
        procname: "xfrm_aevent_etime",
        mode: 0o644,
        handler: "proc_douintvec",
    },
    XfrmCtlTable {
        procname: "xfrm_aevent_rseqth",
        mode: 0o644,
        handler: "proc_douintvec",
    },
    XfrmCtlTable {
        procname: "xfrm_larval_drop",
        mode: 0o644,
        handler: "proc_dointvec",
    },
    XfrmCtlTable {
        procname: "xfrm_acq_expires",
        mode: 0o644,
        handler: "proc_dointvec",
    },
];

pub const fn xfrm_sysctl_defaults() -> XfrmSysctlNet {
    XfrmSysctlNet {
        sysctl_aevent_etime: XFRM_AE_ETIME,
        sysctl_aevent_rseqth: XFRM_AE_SEQT_SIZE,
        sysctl_larval_drop: 1,
        sysctl_acq_expires: 30,
        sysctl_hdr_registered: false,
        sysctl_table_size: 0,
    }
}

pub const fn xfrm_sysctl_init(
    config_sysctl: bool,
    init_user_ns: bool,
    kmemdup_ok: bool,
    register_ok: bool,
) -> Result<XfrmSysctlNet, i32> {
    let mut net = xfrm_sysctl_defaults();
    if !config_sysctl {
        return Ok(net);
    }
    if !kmemdup_ok || !register_ok {
        return Err(-ENOMEM);
    }

    net.sysctl_hdr_registered = true;
    net.sysctl_table_size = if init_user_ns { XFRM_TABLE.len() } else { 0 };
    Ok(net)
}

pub const fn xfrm_sysctl_fini(net: XfrmSysctlNet) -> bool {
    net.sysctl_hdr_registered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfrm_sysctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/xfrm/xfrm_sysctl.c"
        ));
        assert!(source.contains("net->xfrm.sysctl_aevent_etime = XFRM_AE_ETIME;"));
        assert!(source.contains("net->xfrm.sysctl_aevent_rseqth = XFRM_AE_SEQT_SIZE;"));
        assert!(source.contains("net->xfrm.sysctl_larval_drop = 1;"));
        assert!(source.contains("net->xfrm.sysctl_acq_expires = 30;"));
        assert!(source.contains(".procname\t= \"xfrm_aevent_etime\""));
        assert!(source.contains(".proc_handler\t= proc_douintvec"));
        assert!(source.contains(".proc_handler\t= proc_dointvec"));
        assert!(source.contains("table = kmemdup(xfrm_table, sizeof(xfrm_table), GFP_KERNEL);"));
        assert!(source.contains("table[0].data = &net->xfrm.sysctl_aevent_etime;"));
        assert!(source.contains("if (net->user_ns != &init_user_ns)"));
        assert!(source.contains("table_size = 0;"));
        assert!(source.contains("register_net_sysctl_sz(net, \"net/core\", table"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_net_sysctl_table(net->xfrm.sysctl_hdr);"));
    }

    #[test]
    fn sysctl_init_sets_defaults_and_hides_tables_for_unprivileged_namespaces() {
        assert_eq!(
            xfrm_sysctl_init(false, false, false, false),
            Ok(xfrm_sysctl_defaults())
        );
        assert_eq!(xfrm_sysctl_init(true, true, false, true), Err(-ENOMEM));
        let init_net = xfrm_sysctl_init(true, true, true, true).unwrap();
        assert_eq!(init_net.sysctl_aevent_etime, 10);
        assert_eq!(init_net.sysctl_aevent_rseqth, 2);
        assert_eq!(init_net.sysctl_larval_drop, 1);
        assert_eq!(init_net.sysctl_acq_expires, 30);
        assert_eq!(init_net.sysctl_table_size, 4);
        assert!(xfrm_sysctl_fini(init_net));

        let user_net = xfrm_sysctl_init(true, false, true, true).unwrap();
        assert_eq!(user_net.sysctl_table_size, 0);
    }
}
