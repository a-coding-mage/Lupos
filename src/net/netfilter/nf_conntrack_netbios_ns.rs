//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_conntrack_netbios_ns.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_conntrack_netbios_ns.c
//! NetBIOS name-service broadcast conntrack helper metadata.

use core::sync::atomic::{AtomicU32, Ordering};

pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "NetBIOS name service broadcast connection tracking helper";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ip_conntrack_netbios_ns", "nfct-helper-netbios-ns"];

pub const HELPER_NAME: &str = "netbios-ns";
pub const NMBD_PORT: u16 = 137;
pub const NFPROTO_IPV4: u16 = 2;
pub const IPPROTO_UDP: u8 = 17;
pub const DEFAULT_TIMEOUT_SECS: u32 = 3;

static TIMEOUT: AtomicU32 = AtomicU32::new(DEFAULT_TIMEOUT_SECS);

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

pub fn netbios_ns_help() -> u32 {
    timeout()
}

pub fn helper() -> ConntrackHelper {
    ConntrackHelper {
        name: HELPER_NAME,
        l3num: NFPROTO_IPV4,
        udp_port_be: NMBD_PORT.to_be(),
        protonum: IPPROTO_UDP,
        expect_policy: ConntrackExpectPolicy {
            timeout: timeout(),
            ..base_expect_policy()
        },
    }
}

pub fn nf_conntrack_netbios_ns_init() -> ConntrackHelper {
    helper()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_conntrack_netbios_ns_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_conntrack_netbios_ns.c"
        ));
        assert!(source.contains("#define HELPER_NAME\t\"netbios-ns\""));
        assert!(source.contains("#define NMBD_PORT\t137"));
        assert!(source.contains("MODULE_ALIAS(\"ip_conntrack_netbios_ns\");"));
        assert!(source.contains("MODULE_ALIAS_NFCT_HELPER(HELPER_NAME);"));
        assert!(source.contains("static unsigned int timeout __read_mostly = 3;"));
        assert!(source.contains(".max_expected\t= 1"));
        assert!(source.contains("return nf_conntrack_broadcast_help(skb, ct, ctinfo, timeout);"));
        assert!(source.contains(".tuple.src.l3num\t= NFPROTO_IPV4"));
        assert!(source.contains(".tuple.src.u.udp.port\t= cpu_to_be16(NMBD_PORT)"));
        assert!(source.contains(".tuple.dst.protonum\t= IPPROTO_UDP"));
        assert!(source.contains("exp_policy.timeout = timeout;"));
        assert!(source.contains("nf_conntrack_helper_register(&helper);"));

        set_timeout(DEFAULT_TIMEOUT_SECS);
        let helper = nf_conntrack_netbios_ns_init();
        assert_eq!(helper.name, "netbios-ns");
        assert_eq!(helper.udp_port_be, NMBD_PORT.to_be());
        assert_eq!(helper.expect_policy.max_expected, 1);
        assert_eq!(helper.expect_policy.timeout, DEFAULT_TIMEOUT_SECS);
        assert_eq!(netbios_ns_help(), DEFAULT_TIMEOUT_SECS);
    }
}
