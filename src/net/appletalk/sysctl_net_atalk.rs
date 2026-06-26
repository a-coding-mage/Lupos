//! linux-parity: complete
//! linux-source: vendor/linux/net/appletalk/sysctl_net_atalk.c
//! test-origin: linux:vendor/linux/net/appletalk/sysctl_net_atalk.c
//! AppleTalk sysctl table registration metadata.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtalkSysctl {
    pub procname: &'static str,
    pub data_symbol: &'static str,
    pub maxlen_symbol: &'static str,
    pub mode: u16,
    pub proc_handler: &'static str,
}

pub const ATALK_SYSCTL_PATH: &str = "net/appletalk";
pub const ATALK_SYSCTLS: [AtalkSysctl; 4] = [
    AtalkSysctl {
        procname: "aarp-expiry-time",
        data_symbol: "sysctl_aarp_expiry_time",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec_jiffies",
    },
    AtalkSysctl {
        procname: "aarp-tick-time",
        data_symbol: "sysctl_aarp_tick_time",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec_jiffies",
    },
    AtalkSysctl {
        procname: "aarp-retransmit-limit",
        data_symbol: "sysctl_aarp_retransmit_limit",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec",
    },
    AtalkSysctl {
        procname: "aarp-resolve-time",
        data_symbol: "sysctl_aarp_resolve_time",
        maxlen_symbol: "sizeof(int)",
        mode: 0o644,
        proc_handler: "proc_dointvec_jiffies",
    },
];

static ATALK_SYSCTL_REGISTERED: AtomicBool = AtomicBool::new(false);

pub fn atalk_register_sysctl(register_net_sysctl_ok: bool) -> Result<(), i32> {
    if register_net_sysctl_ok {
        ATALK_SYSCTL_REGISTERED.store(true, Ordering::Release);
        Ok(())
    } else {
        Err(-ENOMEM)
    }
}

pub fn atalk_unregister_sysctl() {
    ATALK_SYSCTL_REGISTERED.store(false, Ordering::Release);
}

pub fn atalk_sysctl_registered() -> bool {
    ATALK_SYSCTL_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atalk_sysctl_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/appletalk/sysctl_net_atalk.c"
        ));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("#include <linux/atalk.h>"));
        assert!(source.contains("static struct ctl_table atalk_table[]"));
        assert!(source.contains(".procname\t= \"aarp-expiry-time\""));
        assert!(source.contains(".data\t\t= &sysctl_aarp_expiry_time"));
        assert!(source.contains(".proc_handler\t= proc_dointvec_jiffies"));
        assert!(source.contains(".procname\t= \"aarp-retransmit-limit\""));
        assert!(source.contains(".proc_handler\t= proc_dointvec"));
        assert!(source.contains("register_net_sysctl(&init_net, \"net/appletalk\", atalk_table);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_net_sysctl_table(atalk_table_header);"));

        assert_eq!(ATALK_SYSCTL_PATH, "net/appletalk");
        assert_eq!(ATALK_SYSCTLS.len(), 4);
        assert_eq!(ATALK_SYSCTLS[0].procname, "aarp-expiry-time");
        assert_eq!(ATALK_SYSCTLS[2].proc_handler, "proc_dointvec");
        assert_eq!(atalk_register_sysctl(false), Err(-ENOMEM));
        assert_eq!(atalk_register_sysctl(true), Ok(()));
        assert!(atalk_sysctl_registered());
        atalk_unregister_sysctl();
        assert!(!atalk_sysctl_registered());
    }
}
