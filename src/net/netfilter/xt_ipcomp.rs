//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_ipcomp.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_ipcomp.c
//! Xtables IPv4/IPv6 IPComp CPI range match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Fan Du <fan.du@windriver.com>";
pub const MODULE_DESCRIPTION: &str = "Xtables: IPv4/6 IPsec-IPComp SPI match";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_ipcomp", "ip6t_ipcomp"];
pub const XT_IPCOMP_INV_SPI: u8 = 0x01;
pub const XT_IPCOMP_INV_MASK: u8 = 0x01;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const IPPROTO_COMP: u8 = 108;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtIpcomp {
    pub spis: [u32; 2],
    pub invflags: u8,
    pub hdrres: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompMatchResult {
    pub matched: bool,
    pub hotdrop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub proto: u8,
}

pub const COMP_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "ipcomp",
        family: NFPROTO_IPV4,
        proto: IPPROTO_COMP,
    },
    XtMatch {
        name: "ipcomp",
        family: NFPROTO_IPV6,
        proto: IPPROTO_COMP,
    },
];

pub const fn spi_match(min: u32, max: u32, spi: u32, invert: bool) -> bool {
    (spi >= min && spi <= max) != invert
}

pub const fn comp_mt(info: XtIpcomp, fragoff: u16, cpi_be: Option<u16>) -> CompMatchResult {
    if fragoff != 0 {
        return CompMatchResult {
            matched: false,
            hotdrop: false,
        };
    }
    let Some(cpi_be) = cpi_be else {
        return CompMatchResult {
            matched: false,
            hotdrop: true,
        };
    };
    CompMatchResult {
        matched: spi_match(
            info.spis[0],
            info.spis[1],
            u16::from_be(cpi_be) as u32,
            info.invflags & XT_IPCOMP_INV_SPI != 0,
        ),
        hotdrop: false,
    }
}

pub const fn comp_mt_check(info: XtIpcomp) -> Result<(), i32> {
    if info.invflags & !XT_IPCOMP_INV_MASK != 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn comp_mt_init() -> &'static [XtMatch; 2] {
    &COMP_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_ipcomp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_ipcomp.c"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_ipcomp\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_ipcomp\");"));
        assert!(source.contains("spi_match(u_int32_t min, u_int32_t max"));
        assert!(source.contains("r = (spi >= min && spi <= max) ^ invert;"));
        assert!(source.contains("comp_mt(const struct sk_buff *skb"));
        assert!(source.contains("if (par->fragoff != 0)"));
        assert!(
            source.contains("skb_header_pointer(skb, par->thoff, sizeof(_comphdr), &_comphdr);")
        );
        assert!(source.contains("par->hotdrop = true;"));
        assert!(source.contains("ntohs(chdr->cpi)"));
        assert!(source.contains("if (compinfo->invflags & ~XT_IPCOMP_INV_MASK)"));
        assert!(source.contains(".name\t\t= \"ipcomp\""));
        assert!(source.contains(".proto\t\t= IPPROTO_COMP"));
        assert!(source.contains("xt_register_matches(comp_mt_reg, ARRAY_SIZE(comp_mt_reg));"));
    }

    #[test]
    fn comp_match_handles_fragments_hotdrop_and_inversion() {
        let info = XtIpcomp {
            spis: [100, 200],
            invflags: 0,
            hdrres: 0,
        };
        assert!(comp_mt(info, 0, Some(150u16.to_be())).matched);
        assert!(!comp_mt(info, 1, Some(150u16.to_be())).matched);
        assert!(comp_mt(info, 0, None).hotdrop);
        assert!(
            !comp_mt(
                XtIpcomp {
                    invflags: XT_IPCOMP_INV_SPI,
                    ..info
                },
                0,
                Some(150u16.to_be())
            )
            .matched
        );
        assert_eq!(
            comp_mt_check(XtIpcomp {
                invflags: 0x80,
                ..info
            }),
            Err(-EINVAL)
        );
        assert_eq!(comp_mt_init(), &COMP_MT_REG);
    }
}
