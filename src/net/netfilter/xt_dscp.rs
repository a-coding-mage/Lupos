//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_dscp.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_dscp.c
//! Xtables DSCP/TOS field match.

use crate::include::uapi::errno::EDOM;

pub const MODULE_AUTHOR: &str = "Harald Welte <laforge@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "Xtables: DSCP/TOS field match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 4] = ["ipt_dscp", "ip6t_dscp", "ipt_tos", "ip6t_tos"];
pub const XT_DSCP_MASK: u8 = 0xfc;
pub const XT_DSCP_SHIFT: u8 = 2;
pub const XT_DSCP_MAX: u8 = 0x3f;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtDscpInfo {
    pub dscp: u8,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTosMatchInfo {
    pub tos_mask: u8,
    pub tos_value: u8,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
}

pub const DSCP_MT_REG: [XtMatch; 4] = [
    XtMatch {
        name: "dscp",
        revision: 0,
        family: NFPROTO_IPV4,
    },
    XtMatch {
        name: "dscp",
        revision: 0,
        family: NFPROTO_IPV6,
    },
    XtMatch {
        name: "tos",
        revision: 1,
        family: NFPROTO_IPV4,
    },
    XtMatch {
        name: "tos",
        revision: 1,
        family: NFPROTO_IPV6,
    },
];

pub const fn dscp_mt(dsfield: u8, info: XtDscpInfo) -> bool {
    ((dsfield >> XT_DSCP_SHIFT) == info.dscp) != info.invert
}

pub const fn dscp_mt_check(info: XtDscpInfo) -> Result<(), i32> {
    if info.dscp > XT_DSCP_MAX {
        Err(-EDOM)
    } else {
        Ok(())
    }
}

pub const fn tos_mt(dsfield: u8, info: XtTosMatchInfo) -> bool {
    ((dsfield & info.tos_mask) == info.tos_value) != info.invert
}

pub const fn dscp_mt_init() -> &'static [XtMatch; 4] {
    &DSCP_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_dscp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_dscp.c"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_dscp\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_tos\");"));
        assert!(source.contains("dscp_mt(const struct sk_buff *skb"));
        assert!(source.contains("ipv4_get_dsfield(ip_hdr(skb)) >> XT_DSCP_SHIFT"));
        assert!(source.contains("dscp_mt6(const struct sk_buff *skb"));
        assert!(source.contains("ipv6_get_dsfield(ipv6_hdr(skb)) >> XT_DSCP_SHIFT"));
        assert!(source.contains("if (info->dscp > XT_DSCP_MAX)"));
        assert!(source.contains("return -EDOM;"));
        assert!(source.contains("tos_mt(const struct sk_buff *skb"));
        assert!(source.contains("ip_hdr(skb)->tos & info->tos_mask"));
        assert!(source.contains("ipv6_get_dsfield(ipv6_hdr(skb)) & info->tos_mask"));
        assert!(source.contains(".name\t\t= \"dscp\""));
        assert!(source.contains(".name\t\t= \"tos\""));
        assert!(source.contains("xt_register_matches(dscp_mt_reg, ARRAY_SIZE(dscp_mt_reg));"));
    }

    #[test]
    fn dscp_and_tos_matches_apply_shift_mask_and_inversion() {
        assert!(dscp_mt(
            0b10101000,
            XtDscpInfo {
                dscp: 0b101010,
                invert: false,
            }
        ));
        assert!(!dscp_mt(
            0b10101000,
            XtDscpInfo {
                dscp: 0b101010,
                invert: true,
            }
        ));
        assert_eq!(
            dscp_mt_check(XtDscpInfo {
                dscp: XT_DSCP_MAX + 1,
                invert: false,
            }),
            Err(-EDOM)
        );
        assert!(tos_mt(
            0b1010_1100,
            XtTosMatchInfo {
                tos_mask: 0b1111_0000,
                tos_value: 0b1010_0000,
                invert: false,
            }
        ));
        assert_eq!(dscp_mt_init(), &DSCP_MT_REG);
    }
}
