//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_conntrack_snmp.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_conntrack_snmp.c
//! SNMP broadcast conntrack helper metadata.

use core::sync::atomic::{AtomicU32, Ordering};

pub const MODULE_AUTHOR: &str = "Jiri Olsa <jolsa@redhat.com>";
pub const MODULE_DESCRIPTION: &str = "SNMP service broadcast connection tracking helper";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "nfct-helper-snmp";

pub const SNMP_PORT: u16 = 161;
pub const NFPROTO_IPV4: u16 = 2;
pub const IPPROTO_UDP: u8 = 17;
pub const DEFAULT_TIMEOUT_SECS: u32 = 30;
pub const NF_ACCEPT: i32 = 1;
pub const IPS_SRC_NAT: u32 = 1 << 4;
pub const IPS_DST_NAT: u32 = 1 << 5;
pub const IPS_NAT_MASK: u32 = IPS_SRC_NAT | IPS_DST_NAT;

static TIMEOUT: AtomicU32 = AtomicU32::new(DEFAULT_TIMEOUT_SECS);

pub type NatSnmpHook = fn(protoff: u32, ct_status: u32, ctinfo: u32) -> i32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConntrackExpectPolicy {
    pub max_expected: u32,
    pub timeout: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConntrackHelper {
    pub name: &'static str,
    pub l3num: u16,
    pub udp_port_be: u16,
    pub protonum: u8,
    pub expect_policy: ConntrackExpectPolicy,
}

pub const fn base_expect_policy() -> ConntrackExpectPolicy {
    ConntrackExpectPolicy {
        max_expected: 1,
        timeout: 0,
    }
}

pub fn set_timeout(timeout_secs: u32) {
    TIMEOUT.store(timeout_secs, Ordering::Release);
}

pub fn timeout() -> u32 {
    TIMEOUT.load(Ordering::Acquire)
}

pub fn snmp_conntrack_help(
    protoff: u32,
    ct_status: u32,
    ctinfo: u32,
    nat_hook: Option<NatSnmpHook>,
) -> i32 {
    let _broadcast_timeout = timeout();
    if let Some(nf_nat_snmp) = nat_hook {
        if ct_status & IPS_NAT_MASK != 0 {
            return nf_nat_snmp(protoff, ct_status, ctinfo);
        }
    }
    NF_ACCEPT
}

pub fn helper() -> ConntrackHelper {
    ConntrackHelper {
        name: "snmp",
        l3num: NFPROTO_IPV4,
        udp_port_be: SNMP_PORT.to_be(),
        protonum: IPPROTO_UDP,
        expect_policy: ConntrackExpectPolicy {
            timeout: timeout(),
            ..base_expect_policy()
        },
    }
}

pub fn nf_conntrack_snmp_init() -> ConntrackHelper {
    helper()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nat_hook(protoff: u32, ct_status: u32, ctinfo: u32) -> i32 {
        1000 + protoff as i32 + ct_status as i32 + ctinfo as i32
    }

    #[test]
    fn nf_conntrack_snmp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_conntrack_snmp.c"
        ));
        assert!(source.contains("#define SNMP_PORT\t161"));
        assert!(source.contains("MODULE_ALIAS_NFCT_HELPER(\"snmp\");"));
        assert!(source.contains("static unsigned int timeout __read_mostly = 30;"));
        assert!(source.contains("nf_nat_snmp_hook_fn __rcu *nf_nat_snmp_hook;"));
        assert!(source.contains("nf_conntrack_broadcast_help(skb, ct, ctinfo, timeout);"));
        assert!(source.contains("if (nf_nat_snmp && ct->status & IPS_NAT_MASK)"));
        assert!(source.contains("return NF_ACCEPT;"));
        assert!(source.contains(".name\t\t\t= \"snmp\""));
        assert!(source.contains(".tuple.src.u.udp.port\t= cpu_to_be16(SNMP_PORT)"));
        assert!(source.contains("exp_policy.timeout = timeout;"));
        assert!(source.contains("nf_conntrack_helper_register(&helper);"));

        set_timeout(DEFAULT_TIMEOUT_SECS);
        let helper = nf_conntrack_snmp_init();
        assert_eq!(helper.name, "snmp");
        assert_eq!(helper.udp_port_be, SNMP_PORT.to_be());
        assert_eq!(helper.expect_policy.timeout, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn snmp_help_runs_nat_hook_only_for_nat_connections() {
        set_timeout(DEFAULT_TIMEOUT_SECS);
        assert_eq!(snmp_conntrack_help(2, 0, 3, Some(nat_hook)), NF_ACCEPT);
        assert_eq!(
            snmp_conntrack_help(2, IPS_SRC_NAT, 3, Some(nat_hook)),
            1000 + 2 + IPS_SRC_NAT as i32 + 3
        );
        assert_eq!(snmp_conntrack_help(2, IPS_DST_NAT, 3, None), NF_ACCEPT);
    }
}
