//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_iprange.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_iprange.c
//! Xtables IPv4 and IPv6 address range match.

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHORS: [&str; 2] = [
    "Jozsef Kadlecsik <kadlec@netfilter.org>",
    "Jan Engelhardt <jengelh@medozas.de>",
];
pub const MODULE_DESCRIPTION: &str = "Xtables: arbitrary IPv4 range matching";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_iprange", "ip6t_iprange"];

pub const IPRANGE_SRC: u8 = 1 << 0;
pub const IPRANGE_DST: u8 = 1 << 1;
pub const IPRANGE_SRC_INV: u8 = 1 << 4;
pub const IPRANGE_DST_INV: u8 = 1 << 5;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfInetAddr {
    pub ip: u32,
    pub in6: [u32; 4],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct XtIprangeMtinfo {
    pub src_min: NfInetAddr,
    pub src_max: NfInetAddr,
    pub dst_min: NfInetAddr,
    pub dst_max: NfInetAddr,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
    pub match_fn: &'static str,
}

pub const IPRANGE_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "iprange",
        revision: 1,
        family: NFPROTO_IPV4,
        matchsize: core::mem::size_of::<XtIprangeMtinfo>(),
        match_fn: "iprange_mt4",
    },
    XtMatch {
        name: "iprange",
        revision: 1,
        family: NFPROTO_IPV6,
        matchsize: core::mem::size_of::<XtIprangeMtinfo>(),
        match_fn: "iprange_mt6",
    },
];

pub const fn iprange_mt4(saddr_be: u32, daddr_be: u32, info: XtIprangeMtinfo) -> bool {
    if info.flags & IPRANGE_SRC != 0 {
        let mut m = u32::from_be(saddr_be) < u32::from_be(info.src_min.ip)
            || u32::from_be(saddr_be) > u32::from_be(info.src_max.ip);
        if info.flags & IPRANGE_SRC_INV != 0 {
            m = !m;
        }
        if m {
            return false;
        }
    }
    if info.flags & IPRANGE_DST != 0 {
        let mut m = u32::from_be(daddr_be) < u32::from_be(info.dst_min.ip)
            || u32::from_be(daddr_be) > u32::from_be(info.dst_max.ip);
        if info.flags & IPRANGE_DST_INV != 0 {
            m = !m;
        }
        if m {
            return false;
        }
    }
    true
}

pub const fn iprange_ipv6_lt(a_be32: &[u32; 4], b_be32: &[u32; 4]) -> bool {
    let mut i = 0;
    while i < 4 {
        if a_be32[i] != b_be32[i] {
            return u32::from_be(a_be32[i]) < u32::from_be(b_be32[i]);
        }
        i += 1;
    }
    false
}

pub const fn iprange_mt6(
    saddr_be32: [u32; 4],
    daddr_be32: [u32; 4],
    info: XtIprangeMtinfo,
) -> bool {
    if info.flags & IPRANGE_SRC != 0 {
        let mut m = iprange_ipv6_lt(&saddr_be32, &info.src_min.in6)
            || iprange_ipv6_lt(&info.src_max.in6, &saddr_be32);
        if info.flags & IPRANGE_SRC_INV != 0 {
            m = !m;
        }
        if m {
            return false;
        }
    }
    if info.flags & IPRANGE_DST != 0 {
        let mut m = iprange_ipv6_lt(&daddr_be32, &info.dst_min.in6)
            || iprange_ipv6_lt(&info.dst_max.in6, &daddr_be32);
        if info.flags & IPRANGE_DST_INV != 0 {
            m = !m;
        }
        if m {
            return false;
        }
    }
    true
}

pub const fn iprange_mt_init() -> &'static [XtMatch; 2] {
    &IPRANGE_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn be(v: u32) -> u32 {
        v.to_be()
    }

    #[test]
    fn xt_iprange_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_iprange.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_iprange.h"
        ));
        assert!(header.contains("IPRANGE_SRC     = 1 << 0"));
        assert!(header.contains("IPRANGE_DST_INV = 1 << 5"));
        assert!(source.contains("iprange_mt4(const struct sk_buff *skb"));
        assert!(source.contains("ntohl(iph->saddr) < ntohl(info->src_min.ip)"));
        assert!(source.contains("ntohl(iph->daddr) > ntohl(info->dst_max.ip)"));
        assert!(source.contains("m ^= !!(info->flags & IPRANGE_SRC_INV);"));
        assert!(source.contains("iprange_ipv6_lt(const struct in6_addr *a"));
        assert!(source.contains("for (i = 0; i < 4; ++i)"));
        assert!(source.contains("return ntohl(a->s6_addr32[i]) < ntohl(b->s6_addr32[i]);"));
        assert!(source.contains("iprange_mt6(const struct sk_buff *skb"));
        assert!(source.contains("iprange_ipv6_lt(&info->src_max.in6, &iph->saddr)"));
        assert!(source.contains(".name      = \"iprange\""));
        assert!(source.contains(".revision  = 1"));
        assert!(source.contains(".family    = NFPROTO_IPV4"));
        assert!(source.contains(".family    = NFPROTO_IPV6"));
        assert!(
            source.contains("xt_register_matches(iprange_mt_reg, ARRAY_SIZE(iprange_mt_reg));")
        );
        assert!(source.contains("MODULE_ALIAS(\"ipt_iprange\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_iprange\");"));
    }

    #[test]
    fn ipv4_and_ipv6_ranges_are_inclusive_and_invertible() {
        let info = XtIprangeMtinfo {
            src_min: NfInetAddr {
                ip: be(0x0a00_0001),
                in6: [be(0x2001_0db8), 0, 0, be(1)],
            },
            src_max: NfInetAddr {
                ip: be(0x0a00_00ff),
                in6: [be(0x2001_0db8), 0, 0, be(0xffff)],
            },
            dst_min: NfInetAddr {
                ip: be(0xc000_0201),
                in6: [be(0x2001_0db8), 0, 1, 0],
            },
            dst_max: NfInetAddr {
                ip: be(0xc000_02ff),
                in6: [be(0x2001_0db8), 0, 1, be(0xffff)],
            },
            flags: IPRANGE_SRC | IPRANGE_DST,
        };
        assert!(iprange_mt4(be(0x0a00_0001), be(0xc000_02ff), info));
        assert!(!iprange_mt4(be(0x0a00_0100), be(0xc000_02ff), info));
        assert!(iprange_mt4(
            be(0x0a00_0100),
            be(0xc000_02ff),
            XtIprangeMtinfo {
                flags: IPRANGE_SRC | IPRANGE_SRC_INV,
                ..info
            }
        ));
        assert!(iprange_mt6(
            [be(0x2001_0db8), 0, 0, be(2)],
            [be(0x2001_0db8), 0, 1, be(2)],
            info
        ));
        assert!(!iprange_ipv6_lt(
            &[be(0x2001_0db8), 0, 0, be(1)],
            &[be(0x2001_0db8), 0, 0, be(1)]
        ));
        assert_eq!(iprange_mt_init().len(), 2);
    }
}
