//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_CHECKSUM.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_CHECKSUM.c
//! Xtables checksum target.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Michael S. Tsirkin <mst@redhat.com>";
pub const MODULE_DESCRIPTION: &str = "Xtables: checksum modification";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_CHECKSUM", "ip6t_CHECKSUM"];
pub const XT_CHECKSUM_OP_FILL: u8 = 0x01;
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const IPPROTO_UDP: u8 = 17;
pub const XT_INV_PROTO: u8 = 0x40;
pub const IP6T_F_PROTO: u8 = 0x01;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtChecksumInfo {
    pub operation: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChecksumRule {
    pub family: u8,
    pub ipv4_proto: u8,
    pub ipv4_invflags: u8,
    pub ipv6_flags: u8,
    pub ipv6_proto: u8,
    pub ipv6_invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumCheck {
    UdpRestricted,
    WarnUnrestricted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub family: u8,
    pub targetsize: usize,
    pub table: &'static str,
}

pub const CHECKSUM_TG_REG: [XtTarget; 2] = [
    XtTarget {
        name: "CHECKSUM",
        family: NFPROTO_IPV4,
        targetsize: core::mem::size_of::<XtChecksumInfo>(),
        table: "mangle",
    },
    XtTarget {
        name: "CHECKSUM",
        family: NFPROTO_IPV6,
        targetsize: core::mem::size_of::<XtChecksumInfo>(),
        table: "mangle",
    },
];

pub const fn checksum_tg(checksum_partial: bool, skb_is_gso: bool) -> (u32, bool) {
    let helped = checksum_partial && !skb_is_gso;
    (XT_CONTINUE, helped)
}

pub fn checksum_tg_check(info: XtChecksumInfo, rule: ChecksumRule) -> Result<ChecksumCheck, i32> {
    if info.operation & !XT_CHECKSUM_OP_FILL != 0 {
        return Err(-EINVAL);
    }
    if info.operation == 0 {
        return Err(-EINVAL);
    }

    match rule.family {
        NFPROTO_IPV4
            if rule.ipv4_proto == IPPROTO_UDP && (rule.ipv4_invflags & XT_INV_PROTO) == 0 =>
        {
            return Ok(ChecksumCheck::UdpRestricted);
        }
        NFPROTO_IPV6
            if (rule.ipv6_flags & IP6T_F_PROTO) != 0
                && rule.ipv6_proto == IPPROTO_UDP
                && (rule.ipv6_invflags & XT_INV_PROTO) == 0 =>
        {
            return Ok(ChecksumCheck::UdpRestricted);
        }
        _ => {}
    }
    Ok(ChecksumCheck::WarnUnrestricted)
}

pub const fn checksum_tg_init() -> &'static [XtTarget; 2] {
    &CHECKSUM_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_checksum_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_CHECKSUM.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_CHECKSUM.h"
        ));
        assert!(header.contains("#define XT_CHECKSUM_OP_FILL\t0x01"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_CHECKSUM\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_CHECKSUM\");"));
        assert!(source.contains("checksum_tg(struct sk_buff *skb"));
        assert!(source.contains("if (skb->ip_summed == CHECKSUM_PARTIAL && !skb_is_gso(skb))"));
        assert!(source.contains("skb_checksum_help(skb);"));
        assert!(source.contains("return XT_CONTINUE;"));
        assert!(source.contains("checksum_tg_check(const struct xt_tgchk_param *par)"));
        assert!(source.contains("einfo->operation & ~XT_CHECKSUM_OP_FILL"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("if (!einfo->operation)"));
        assert!(source.contains("case NFPROTO_IPV4:"));
        assert!(source.contains("i4->proto == IPPROTO_UDP"));
        assert!(source.contains("(i4->invflags & XT_INV_PROTO) == 0"));
        assert!(source.contains("case NFPROTO_IPV6:"));
        assert!(source.contains("(i6->flags & IP6T_F_PROTO)"));
        assert!(source.contains("i6->proto == IPPROTO_UDP"));
        assert!(source.contains("pr_warn_once(\"CHECKSUM should be avoided."));
        assert!(source.contains(".name\t\t= \"CHECKSUM\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV4"));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".table\t\t= \"mangle\""));
        assert!(
            source.contains("xt_register_targets(checksum_tg_reg, ARRAY_SIZE(checksum_tg_reg));")
        );
        assert!(
            source.contains("xt_unregister_targets(checksum_tg_reg, ARRAY_SIZE(checksum_tg_reg));")
        );
    }

    #[test]
    fn checksum_target_fills_partial_nongso_and_validates_udp_restriction() {
        assert_eq!(checksum_tg(true, false), (XT_CONTINUE, true));
        assert_eq!(checksum_tg(true, true), (XT_CONTINUE, false));
        assert_eq!(checksum_tg(false, false), (XT_CONTINUE, false));

        let ipv4_udp = ChecksumRule {
            family: NFPROTO_IPV4,
            ipv4_proto: IPPROTO_UDP,
            ipv4_invflags: 0,
            ipv6_flags: 0,
            ipv6_proto: 0,
            ipv6_invflags: 0,
        };
        assert_eq!(
            checksum_tg_check(
                XtChecksumInfo {
                    operation: XT_CHECKSUM_OP_FILL,
                },
                ipv4_udp
            ),
            Ok(ChecksumCheck::UdpRestricted)
        );
        assert_eq!(
            checksum_tg_check(
                XtChecksumInfo {
                    operation: XT_CHECKSUM_OP_FILL,
                },
                ChecksumRule {
                    ipv4_invflags: XT_INV_PROTO,
                    ..ipv4_udp
                }
            ),
            Ok(ChecksumCheck::WarnUnrestricted)
        );
        assert_eq!(
            checksum_tg_check(
                XtChecksumInfo {
                    operation: XT_CHECKSUM_OP_FILL,
                },
                ChecksumRule {
                    family: NFPROTO_IPV6,
                    ipv6_flags: IP6T_F_PROTO,
                    ipv6_proto: IPPROTO_UDP,
                    ..ipv4_udp
                }
            ),
            Ok(ChecksumCheck::UdpRestricted)
        );
        assert_eq!(
            checksum_tg_check(XtChecksumInfo { operation: 0 }, ipv4_udp),
            Err(-EINVAL)
        );
        assert_eq!(
            checksum_tg_check(XtChecksumInfo { operation: 0x2 }, ipv4_udp),
            Err(-EINVAL)
        );
        assert_eq!(checksum_tg_init().len(), 2);
    }
}
