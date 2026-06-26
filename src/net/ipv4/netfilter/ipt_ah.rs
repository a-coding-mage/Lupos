//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netfilter/ipt_ah.c
//! test-origin: linux:vendor/linux/net/ipv4/netfilter/ipt_ah.c
//! Xtables IPv4 IPsec AH SPI match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Yon Uriarte <yon@astaro.de>";
pub const MODULE_DESCRIPTION: &str = "Xtables: IPv4 IPsec-AH SPI match";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_IPV4: u8 = 2;
pub const IPPROTO_AH: u8 = 51;
pub const IPT_AH_INV_SPI: u8 = 0x01;
pub const IPT_AH_INV_MASK: u8 = IPT_AH_INV_SPI;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IptAh {
    pub spis: [u32; 2],
    pub invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AhPacket {
    pub fragoff: u16,
    pub spi: Option<u32>,
    pub hotdrop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
    pub proto: u8,
}

pub const AH_MT_REG: XtMatch = XtMatch {
    name: "ah",
    family: NFPROTO_IPV4,
    matchsize: core::mem::size_of::<IptAh>(),
    proto: IPPROTO_AH,
};

pub const fn spi_match(min: u32, max: u32, spi: u32, invert: bool) -> bool {
    (spi >= min && spi <= max) != invert
}

pub fn ah_mt(packet: &mut AhPacket, info: IptAh) -> bool {
    if packet.fragoff != 0 {
        return false;
    }
    let Some(spi) = packet.spi else {
        packet.hotdrop = true;
        return false;
    };
    spi_match(
        info.spis[0],
        info.spis[1],
        spi,
        info.invflags & IPT_AH_INV_SPI != 0,
    )
}

pub const fn ah_mt_check(info: IptAh) -> Result<(), i32> {
    if info.invflags & !IPT_AH_INV_MASK != 0 {
        Err(-EINVAL)
    } else {
        Ok(())
    }
}

pub const fn ah_mt_init() -> &'static XtMatch {
    &AH_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipt_ah_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netfilter/ipt_ah.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Yon Uriarte"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: IPv4 IPsec-AH SPI match\")"));
        assert!(
            source.contains("spi_match(u_int32_t min, u_int32_t max, u_int32_t spi, bool invert)")
        );
        assert!(source.contains("r = (spi >= min && spi <= max) ^ invert;"));
        assert!(source.contains("if (par->fragoff != 0)"));
        assert!(source.contains("return false;"));
        assert!(source.contains("skb_header_pointer(skb, par->thoff, sizeof(_ahdr), &_ahdr);"));
        assert!(source.contains("par->hotdrop = true;"));
        assert!(source.contains("ntohl(ah->spi)"));
        assert!(source.contains("IPT_AH_INV_SPI"));
        assert!(source.contains("if (ahinfo->invflags & ~IPT_AH_INV_MASK)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains(".name\t\t= \"ah\""));
        assert!(source.contains(".proto\t\t= IPPROTO_AH"));
        assert!(source.contains("xt_register_match(&ah_mt_reg);"));

        assert!(spi_match(10, 20, 10, false));
        assert!(!spi_match(10, 20, 9, false));
        assert!(spi_match(10, 20, 9, true));
        let info = IptAh {
            spis: [100, 200],
            invflags: 0,
        };
        let mut packet = AhPacket {
            fragoff: 0,
            spi: Some(150),
            hotdrop: false,
        };
        assert!(ah_mt(&mut packet, info));
        packet.spi = None;
        assert!(!ah_mt(&mut packet, info));
        assert!(packet.hotdrop);
        assert_eq!(
            ah_mt_check(IptAh {
                invflags: 0x2,
                ..info
            }),
            Err(-EINVAL)
        );
        assert_eq!(ah_mt_init(), &AH_MT_REG);
    }
}
