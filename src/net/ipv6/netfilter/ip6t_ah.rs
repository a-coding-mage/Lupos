//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/ip6t_ah.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/ip6t_ah.c
//! Xtables IPv6 IPsec AH match.

use crate::include::uapi::errno::{EINVAL, ENOENT};

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "Xtables: IPv6 IPsec-AH match";
pub const MODULE_AUTHOR: &str = "Andras Kis-Szabo <kisza@sch.bme.hu>";
pub const NFPROTO_IPV6: u8 = 10;
pub const NEXTHDR_AUTH: i32 = 51;
pub const IP6T_AH_INV_SPI: u8 = 0x01;
pub const IP6T_AH_INV_LEN: u8 = 0x02;
pub const IP6T_AH_INV_MASK: u8 = 0x03;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ip6tAh {
    pub spis: [u32; 2],
    pub hdrlen: u32,
    pub hdrres: u8,
    pub invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpAuthHdr {
    pub hdrlen: u8,
    pub reserved: u16,
    pub spi: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AhPacket {
    pub find_hdr_ret: Result<Option<IpAuthHdr>, i32>,
    pub hotdrop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
}

pub const AH_MT6_REG: XtMatch = XtMatch {
    name: "ah",
    family: NFPROTO_IPV6,
    matchsize: core::mem::size_of::<Ip6tAh>(),
};

pub const fn spi_match(min: u32, max: u32, spi: u32, invert: bool) -> bool {
    (spi >= min && spi <= max) != invert
}

pub const fn ipv6_authlen(ah: IpAuthHdr) -> u32 {
    ((ah.hdrlen as u32) + 2) << 2
}

pub fn ah_mt6(packet: &mut AhPacket, ahinfo: Ip6tAh) -> bool {
    let ah = match packet.find_hdr_ret {
        Err(err) => {
            if err != -ENOENT {
                packet.hotdrop = true;
            }
            return false;
        }
        Ok(None) => {
            packet.hotdrop = true;
            return false;
        }
        Ok(Some(ah)) => ah,
    };

    let hdrlen = ipv6_authlen(ah);
    spi_match(
        ahinfo.spis[0],
        ahinfo.spis[1],
        u32::from_be(ah.spi),
        ahinfo.invflags & IP6T_AH_INV_SPI != 0,
    ) && (ahinfo.hdrlen == 0
        || (ahinfo.hdrlen == hdrlen) != (ahinfo.invflags & IP6T_AH_INV_LEN != 0))
        && !(ahinfo.hdrres != 0 && ah.reserved != 0)
}

pub const fn ah_mt6_check(ahinfo: Ip6tAh) -> Result<(), i32> {
    if ahinfo.invflags & !IP6T_AH_INV_MASK != 0 {
        Err(-EINVAL)
    } else {
        Ok(())
    }
}

pub const fn ah_mt6_init() -> &'static XtMatch {
    &AH_MT6_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> Ip6tAh {
        Ip6tAh {
            spis: [10, 20],
            hdrlen: 12,
            hdrres: 1,
            invflags: 0,
        }
    }

    fn packet(spi: u32) -> AhPacket {
        AhPacket {
            find_hdr_ret: Ok(Some(IpAuthHdr {
                hdrlen: 1,
                reserved: 0,
                spi: spi.to_be(),
            })),
            hotdrop: false,
        }
    }

    #[test]
    fn ip6t_ah_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/ip6t_ah.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_ipv6/ip6t_ah.h"
        ));
        let ipv6 = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/ipv6.h"
        ));

        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: IPv6 IPsec-AH match\")"));
        assert!(source.contains("MODULE_AUTHOR(\"Andras Kis-Szabo"));
        assert!(
            source.contains("spi_match(u_int32_t min, u_int32_t max, u_int32_t spi, bool invert)")
        );
        assert!(source.contains("r = (spi >= min && spi <= max) ^ invert;"));
        assert!(source.contains("err = ipv6_find_hdr(skb, &ptr, NEXTHDR_AUTH, NULL, NULL);"));
        assert!(source.contains("if (err != -ENOENT)"));
        assert!(source.contains("par->hotdrop = true;"));
        assert!(source.contains("skb_header_pointer(skb, ptr, sizeof(_ah), &_ah);"));
        assert!(source.contains("hdrlen = ipv6_authlen(ah);"));
        assert!(source.contains("ntohl(ah->spi)"));
        assert!(source.contains("!!(ahinfo->invflags & IP6T_AH_INV_SPI)"));
        assert!(source.contains("!!(ahinfo->invflags & IP6T_AH_INV_LEN)"));
        assert!(source.contains("!(ahinfo->hdrres && ah->reserved)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains(".name\t\t= \"ah\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".matchsize\t= sizeof(struct ip6t_ah)"));
        assert!(source.contains("xt_register_match(&ah_mt6_reg);"));
        assert!(header.contains("struct ip6t_ah"));
        assert!(header.contains("#define IP6T_AH_INV_MASK\t0x03"));
        assert!(ipv6.contains("#define ipv6_authlen(p) (((p)->hdrlen+2) << 2)"));
    }

    #[test]
    fn ah_match_checks_spi_length_reserved_and_hotdrop_paths() {
        assert!(spi_match(10, 20, 15, false));
        assert!(spi_match(10, 20, 9, true));
        assert_eq!(
            ipv6_authlen(IpAuthHdr {
                hdrlen: 1,
                reserved: 0,
                spi: 0
            }),
            12
        );

        let mut good = packet(15);
        assert!(ah_mt6(&mut good, info()));
        assert!(!good.hotdrop);

        let mut bad_spi = packet(30);
        assert!(!ah_mt6(&mut bad_spi, info()));

        let mut inverted_spi = packet(30);
        assert!(ah_mt6(
            &mut inverted_spi,
            Ip6tAh {
                invflags: IP6T_AH_INV_SPI,
                ..info()
            },
        ));

        let mut bad_len = packet(15);
        assert!(!ah_mt6(
            &mut bad_len,
            Ip6tAh {
                hdrlen: 16,
                ..info()
            },
        ));

        let mut inverted_len = packet(15);
        assert!(ah_mt6(
            &mut inverted_len,
            Ip6tAh {
                hdrlen: 16,
                invflags: IP6T_AH_INV_LEN,
                ..info()
            },
        ));

        let mut reserved = AhPacket {
            find_hdr_ret: Ok(Some(IpAuthHdr {
                reserved: 1,
                ..packet(15).find_hdr_ret.unwrap().unwrap()
            })),
            hotdrop: false,
        };
        assert!(!ah_mt6(&mut reserved, info()));

        let mut not_found = AhPacket {
            find_hdr_ret: Err(-ENOENT),
            hotdrop: false,
        };
        assert!(!ah_mt6(&mut not_found, info()));
        assert!(!not_found.hotdrop);

        let mut find_error = AhPacket {
            find_hdr_ret: Err(-5),
            hotdrop: false,
        };
        assert!(!ah_mt6(&mut find_error, info()));
        assert!(find_error.hotdrop);

        let mut tiny = AhPacket {
            find_hdr_ret: Ok(None),
            hotdrop: false,
        };
        assert!(!ah_mt6(&mut tiny, info()));
        assert!(tiny.hotdrop);

        assert_eq!(
            ah_mt6_check(Ip6tAh {
                invflags: 0x4,
                ..info()
            }),
            Err(-EINVAL)
        );
        assert_eq!(ah_mt6_check(info()), Ok(()));
        assert_eq!(ah_mt6_init(), &AH_MT6_REG);
    }
}
