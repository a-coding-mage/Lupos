//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_hl.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_hl.c
//! Xtables IPv4 TTL and IPv6 Hop Limit match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Maciej Soltysiak <solt@dns.toxicfilms.tv>";
pub const MODULE_DESCRIPTION: &str = "Xtables: Hoplimit/TTL field match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_ttl", "ip6t_hl"];

pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const IPT_TTL_EQ: u8 = 0;
pub const IPT_TTL_NE: u8 = 1;
pub const IPT_TTL_LT: u8 = 2;
pub const IPT_TTL_GT: u8 = 3;
pub const IP6T_HL_EQ: u8 = 0;
pub const IP6T_HL_NE: u8 = 1;
pub const IP6T_HL_LT: u8 = 2;
pub const IP6T_HL_GT: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IptTtlInfo {
    pub mode: u8,
    pub ttl: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ip6tHlInfo {
    pub mode: u8,
    pub hop_limit: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
    pub checkentry: &'static str,
    pub matcher: &'static str,
}

pub const HL_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "ttl",
        revision: 0,
        family: NFPROTO_IPV4,
        matchsize: core::mem::size_of::<IptTtlInfo>(),
        checkentry: "ttl_mt_check",
        matcher: "ttl_mt",
    },
    XtMatch {
        name: "hl",
        revision: 0,
        family: NFPROTO_IPV6,
        matchsize: core::mem::size_of::<Ip6tHlInfo>(),
        checkentry: "hl_mt6_check",
        matcher: "hl_mt6",
    },
];

pub const fn ttl_mt_check(info: IptTtlInfo) -> Result<(), i32> {
    if info.mode > IPT_TTL_GT {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn ttl_mt(ttl: u8, info: IptTtlInfo) -> bool {
    match info.mode {
        IPT_TTL_EQ => ttl == info.ttl,
        IPT_TTL_NE => ttl != info.ttl,
        IPT_TTL_LT => ttl < info.ttl,
        IPT_TTL_GT => ttl > info.ttl,
        _ => false,
    }
}

pub const fn hl_mt6_check(info: Ip6tHlInfo) -> Result<(), i32> {
    if info.mode > IP6T_HL_GT {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn hl_mt6(hop_limit: u8, info: Ip6tHlInfo) -> bool {
    match info.mode {
        IP6T_HL_EQ => hop_limit == info.hop_limit,
        IP6T_HL_NE => hop_limit != info.hop_limit,
        IP6T_HL_LT => hop_limit < info.hop_limit,
        IP6T_HL_GT => hop_limit > info.hop_limit,
        _ => false,
    }
}

pub const fn hl_mt_init() -> &'static [XtMatch; 2] {
    &HL_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_hl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_hl.c"
        ));
        let ipt_ttl = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv4/ipt_ttl.h"
        ));
        let ip6t_hl = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv6/ip6t_hl.h"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_ttl\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_hl\");"));
        assert!(source.contains("ttl_mt_check(const struct xt_mtchk_param *par)"));
        assert!(source.contains("if (info->mode > IPT_TTL_GT)"));
        assert!(source.contains("ttl_mt(const struct sk_buff *skb"));
        assert!(source.contains("const u8 ttl = ip_hdr(skb)->ttl;"));
        assert!(source.contains("case IPT_TTL_EQ:"));
        assert!(source.contains("case IPT_TTL_NE:"));
        assert!(source.contains("case IPT_TTL_LT:"));
        assert!(source.contains("case IPT_TTL_GT:"));
        assert!(source.contains("hl_mt6_check(const struct xt_mtchk_param *par)"));
        assert!(source.contains("if (info->mode > IP6T_HL_GT)"));
        assert!(source.contains("const struct ipv6hdr *ip6h = ipv6_hdr(skb);"));
        assert!(source.contains("case IP6T_HL_EQ:"));
        assert!(source.contains("case IP6T_HL_NE:"));
        assert!(source.contains("case IP6T_HL_LT:"));
        assert!(source.contains("case IP6T_HL_GT:"));
        assert!(source.contains(".name       = \"ttl\""));
        assert!(source.contains(".family     = NFPROTO_IPV4"));
        assert!(source.contains(".matchsize  = sizeof(struct ipt_ttl_info)"));
        assert!(source.contains(".name       = \"hl\""));
        assert!(source.contains(".family     = NFPROTO_IPV6"));
        assert!(source.contains(".matchsize  = sizeof(struct ip6t_hl_info)"));
        assert!(source.contains("xt_register_matches(hl_mt_reg, ARRAY_SIZE(hl_mt_reg));"));
        assert!(source.contains("xt_unregister_matches(hl_mt_reg, ARRAY_SIZE(hl_mt_reg));"));
        assert!(ipt_ttl.contains("IPT_TTL_EQ = 0"));
        assert!(ipt_ttl.contains("IPT_TTL_GT"));
        assert!(ip6t_hl.contains("IP6T_HL_EQ = 0"));
        assert!(ip6t_hl.contains("IP6T_HL_GT"));
    }

    #[test]
    fn ttl_and_hop_limit_match_modes_follow_linux_switches() {
        assert_eq!(
            ttl_mt_check(IptTtlInfo {
                mode: IPT_TTL_GT + 1,
                ttl: 64,
            }),
            Err(-EINVAL)
        );
        assert!(ttl_mt(
            64,
            IptTtlInfo {
                mode: IPT_TTL_EQ,
                ttl: 64,
            }
        ));
        assert!(ttl_mt(
            63,
            IptTtlInfo {
                mode: IPT_TTL_NE,
                ttl: 64,
            }
        ));
        assert!(ttl_mt(
            63,
            IptTtlInfo {
                mode: IPT_TTL_LT,
                ttl: 64,
            }
        ));
        assert!(ttl_mt(
            65,
            IptTtlInfo {
                mode: IPT_TTL_GT,
                ttl: 64,
            }
        ));
        assert_eq!(
            hl_mt6_check(Ip6tHlInfo {
                mode: IP6T_HL_GT + 1,
                hop_limit: 64,
            }),
            Err(-EINVAL)
        );
        assert!(hl_mt6(
            64,
            Ip6tHlInfo {
                mode: IP6T_HL_EQ,
                hop_limit: 64,
            }
        ));
        assert_eq!(hl_mt_init(), &HL_MT_REG);
    }
}
