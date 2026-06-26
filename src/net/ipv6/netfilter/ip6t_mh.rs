//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/netfilter/ip6t_mh.c
//! test-origin: linux:vendor/linux/net/ipv6/netfilter/ip6t_mh.c
//! Xtables IPv6 Mobility Header match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_DESCRIPTION: &str = "Xtables: IPv6 Mobility Header match";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_IPV6: u8 = 10;
pub const IPPROTO_NONE: u8 = 59;
pub const IPPROTO_MH: u8 = 135;
pub const IP6T_MH_INV_TYPE: u8 = 0x01;
pub const IP6T_MH_INV_MASK: u8 = IP6T_MH_INV_TYPE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ip6tMh {
    pub types: [u8; 2],
    pub invflags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ip6Mh {
    pub ip6mh_proto: u8,
    pub ip6mh_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MhPacket {
    pub fragoff: u16,
    pub header: Option<Ip6Mh>,
    pub hotdrop: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub family: u8,
    pub matchsize: usize,
    pub proto: u8,
}

pub const MH_MT6_REG: XtMatch = XtMatch {
    name: "mh",
    family: NFPROTO_IPV6,
    matchsize: core::mem::size_of::<Ip6tMh>(),
    proto: IPPROTO_MH,
};

pub const fn type_match(min: u8, max: u8, type_: u8, invert: bool) -> bool {
    (type_ >= min && type_ <= max) != invert
}

pub fn mh_mt6(packet: &mut MhPacket, mhinfo: Ip6tMh) -> bool {
    if packet.fragoff != 0 {
        return false;
    }

    let Some(mh) = packet.header else {
        packet.hotdrop = true;
        return false;
    };

    if mh.ip6mh_proto != IPPROTO_NONE {
        packet.hotdrop = true;
        return false;
    }

    type_match(
        mhinfo.types[0],
        mhinfo.types[1],
        mh.ip6mh_type,
        mhinfo.invflags & IP6T_MH_INV_TYPE != 0,
    )
}

pub const fn mh_mt6_check(mhinfo: Ip6tMh) -> Result<(), i32> {
    if mhinfo.invflags & !IP6T_MH_INV_MASK != 0 {
        Err(-EINVAL)
    } else {
        Ok(())
    }
}

pub const fn mh_mt6_init() -> &'static XtMatch {
    &MH_MT6_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip6t_mh_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/netfilter/ip6t_mh.c"
        ));
        assert!(source.contains("MODULE_DESCRIPTION(\"Xtables: IPv6 Mobility Header match\")"));
        assert!(
            source.contains("type_match(u_int8_t min, u_int8_t max, u_int8_t type, bool invert)")
        );
        assert!(source.contains("return (type >= min && type <= max) ^ invert;"));
        assert!(source.contains("static bool mh_mt6"));
        assert!(source.contains("if (par->fragoff != 0)"));
        assert!(source.contains("skb_header_pointer(skb, par->thoff, sizeof(_mh), &_mh);"));
        assert!(source.contains("Dropping evil MH tinygram."));
        assert!(source.contains("par->hotdrop = true;"));
        assert!(source.contains("if (mh->ip6mh_proto != IPPROTO_NONE)"));
        assert!(source.contains("Dropping invalid MH Payload Proto"));
        assert!(source.contains("IP6T_MH_INV_TYPE"));
        assert!(source.contains("return (mhinfo->invflags & ~IP6T_MH_INV_MASK) ? -EINVAL : 0;"));
        assert!(source.contains(".name\t\t= \"mh\""));
        assert!(source.contains(".family\t\t= NFPROTO_IPV6"));
        assert!(source.contains(".matchsize\t= sizeof(struct ip6t_mh)"));
        assert!(source.contains(".proto\t\t= IPPROTO_MH"));
        assert!(source.contains("xt_register_match(&mh_mt6_reg);"));
    }

    #[test]
    fn mh_match_checks_fragment_header_and_invert_flags() {
        let info = Ip6tMh {
            types: [5, 7],
            invflags: 0,
        };
        let mut packet = MhPacket {
            fragoff: 0,
            header: Some(Ip6Mh {
                ip6mh_proto: IPPROTO_NONE,
                ip6mh_type: 6,
            }),
            hotdrop: false,
        };
        assert!(mh_mt6(&mut packet, info));
        assert!(!mh_mt6(
            &mut MhPacket {
                fragoff: 1,
                ..packet
            },
            info
        ));
        let mut tiny = MhPacket {
            header: None,
            ..packet
        };
        assert!(!mh_mt6(&mut tiny, info));
        assert!(tiny.hotdrop);
        let mut invalid_proto = MhPacket {
            header: Some(Ip6Mh {
                ip6mh_proto: 17,
                ip6mh_type: 6,
            }),
            ..packet
        };
        assert!(!mh_mt6(&mut invalid_proto, info));
        assert!(invalid_proto.hotdrop);
        assert!(type_match(5, 7, 4, true));
        assert_eq!(
            mh_mt6_check(Ip6tMh {
                invflags: 0x2,
                ..info
            }),
            Err(-EINVAL)
        );
        assert_eq!(mh_mt6_init(), &MH_MT6_REG);
    }
}
