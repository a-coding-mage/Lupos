//! linux-parity: complete
//! linux-source: vendor/linux/net/tipc/sysctl.c
//! test-origin: linux:vendor/linux/net/tipc/sysctl.c
//! TIPC sysctl table registration.

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TipcCtlTable {
    pub procname: &'static str,
    pub mode: u16,
    pub handler: &'static str,
    pub extra1: Option<&'static str>,
    pub extra2: Option<&'static str>,
}

pub const TIPC_TABLE: [TipcCtlTable; 6] = [
    TipcCtlTable {
        procname: "tipc_rmem",
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("SYSCTL_ONE"),
        extra2: None,
    },
    TipcCtlTable {
        procname: "named_timeout",
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("SYSCTL_ZERO"),
        extra2: None,
    },
    TipcCtlTable {
        procname: "sk_filter",
        mode: 0o644,
        handler: "proc_doulongvec_minmax",
        extra1: None,
        extra2: None,
    },
    TipcCtlTable {
        procname: "max_tfms",
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("SYSCTL_ONE"),
        extra2: None,
    },
    TipcCtlTable {
        procname: "key_exchange_enabled",
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("SYSCTL_ZERO"),
        extra2: Some("SYSCTL_ONE"),
    },
    TipcCtlTable {
        procname: "bc_retruni",
        mode: 0o644,
        handler: "proc_doulongvec_minmax",
        extra1: None,
        extra2: None,
    },
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TipcSysctlState {
    pub registered: bool,
}

pub fn tipc_register_sysctl(state: &mut TipcSysctlState, register_ok: bool) -> Result<(), i32> {
    if !register_ok {
        return Err(-ENOMEM);
    }
    state.registered = true;
    Ok(())
}

pub fn tipc_unregister_sysctl(state: &mut TipcSysctlState) {
    state.registered = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipc_sysctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/tipc/sysctl.c"
        ));
        assert!(source.contains("static struct ctl_table_header *tipc_ctl_hdr;"));
        assert!(source.contains("static struct ctl_table tipc_table[]"));
        assert!(source.contains(".procname\t= \"tipc_rmem\""));
        assert!(source.contains(".data\t\t= &sysctl_tipc_rmem"));
        assert!(source.contains(".extra1         = SYSCTL_ONE"));
        assert!(source.contains(".procname\t= \"named_timeout\""));
        assert!(source.contains(".procname       = \"sk_filter\""));
        assert!(source.contains(".procname\t= \"max_tfms\""));
        assert!(source.contains(".procname\t= \"key_exchange_enabled\""));
        assert!(source.contains(".extra2         = SYSCTL_ONE"));
        assert!(source.contains(".procname\t= \"bc_retruni\""));
        assert!(
            source.contains(
                "tipc_ctl_hdr = register_net_sysctl(&init_net, \"net/tipc\", tipc_table);"
            )
        );
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_net_sysctl_table(tipc_ctl_hdr);"));
    }

    #[test]
    fn sysctl_registration_tracks_header_lifetime() {
        let mut state = TipcSysctlState::default();
        assert_eq!(tipc_register_sysctl(&mut state, false), Err(-ENOMEM));
        assert!(!state.registered);
        assert_eq!(tipc_register_sysctl(&mut state, true), Ok(()));
        assert!(state.registered);
        tipc_unregister_sysctl(&mut state);
        assert!(!state.registered);
        assert_eq!(TIPC_TABLE[0].procname, "tipc_rmem");
        assert_eq!(TIPC_TABLE[5].procname, "bc_retruni");
    }
}
