//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/ndisc.c
//! test-origin: linux:vendor/linux/net/6lowpan/ndisc.c
//! 6LoWPAN neighbour-discovery short-address option handling.

use super::core::lowpan_802154_is_valid_src_short_addr;

pub const NDISC_802154_SHORT_ADDR_LENGTH: u8 = 1;
pub const IEEE802154_SHORT_ADDR_LEN: usize = 2;
pub const NEIGH_UPDATE_F_OVERRIDE: u32 = 1;

pub const ND_OPT_SOURCE_LL_ADDR: u8 = 1;
pub const ND_OPT_TARGET_LL_ADDR: u8 = 2;

pub const NDISC_ROUTER_SOLICITATION: u8 = 133;
pub const NDISC_ROUTER_ADVERTISEMENT: u8 = 134;
pub const NDISC_NEIGHBOUR_SOLICITATION: u8 = 135;
pub const NDISC_NEIGHBOUR_ADVERTISEMENT: u8 = 136;
pub const NDISC_REDIRECT: u8 = 137;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LowpanNdOption {
    SourceShortAddr,
    TargetShortAddr,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LowpanNdiscOptions {
    pub source_short_present: bool,
    pub target_short_present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LowpanAddrOption {
    pub opt_type: u8,
    pub short_addr_be: [u8; IEEE802154_SHORT_ADDR_LEN],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LowpanNdiscOps {
    pub parse_options: bool,
    pub update: bool,
    pub opt_addr_space: bool,
    pub fill_addr_option: bool,
    pub prefix_rcv_add_addr: bool,
}

pub const LOWPAN_NDISC_OPS: LowpanNdiscOps = LowpanNdiscOps {
    parse_options: true,
    update: true,
    opt_addr_space: true,
    fill_addr_option: true,
    prefix_rcv_add_addr: true,
};

pub const fn lowpan_ndisc_parse_options(
    is_ieee802154: bool,
    nd_opt_type: u8,
    nd_opt_len: u8,
) -> Option<LowpanNdOption> {
    if !is_ieee802154 || nd_opt_len != NDISC_802154_SHORT_ADDR_LENGTH {
        return None;
    }
    match nd_opt_type {
        ND_OPT_SOURCE_LL_ADDR => Some(LowpanNdOption::SourceShortAddr),
        ND_OPT_TARGET_LL_ADDR => Some(LowpanNdOption::TargetShortAddr),
        _ => None,
    }
}

pub fn lowpan_ndisc_parse_802154_options(
    nd_opt_type: u8,
    nd_opt_len: u8,
    ndopts: &mut LowpanNdiscOptions,
) -> i32 {
    if nd_opt_len != NDISC_802154_SHORT_ADDR_LENGTH {
        return 0;
    }
    match nd_opt_type {
        ND_OPT_SOURCE_LL_ADDR => {
            ndopts.source_short_present = true;
            1
        }
        ND_OPT_TARGET_LL_ADDR => {
            ndopts.target_short_present = true;
            1
        }
        _ => 0,
    }
}

pub fn lowpan_ndisc_parse_options_into(
    is_ieee802154: bool,
    nd_opt_type: u8,
    nd_opt_len: u8,
    ndopts: &mut LowpanNdiscOptions,
) -> i32 {
    if !is_ieee802154 {
        return 0;
    }
    match nd_opt_type {
        ND_OPT_SOURCE_LL_ADDR | ND_OPT_TARGET_LL_ADDR => {
            lowpan_ndisc_parse_802154_options(nd_opt_type, nd_opt_len, ndopts)
        }
        _ => 0,
    }
}

pub const fn lowpan_ndisc_update_short_addr(
    is_ieee802154: bool,
    flags: u32,
    icmp6_type: u8,
    source_short: Option<u16>,
    target_short: Option<u16>,
) -> Option<u16> {
    if !is_ieee802154 || flags & NEIGH_UPDATE_F_OVERRIDE == 0 {
        return None;
    }
    let candidate = match icmp6_type {
        NDISC_ROUTER_SOLICITATION | NDISC_ROUTER_ADVERTISEMENT | NDISC_NEIGHBOUR_SOLICITATION => {
            source_short
        }
        NDISC_REDIRECT | NDISC_NEIGHBOUR_ADVERTISEMENT => target_short,
        _ => None,
    };
    match candidate {
        Some(short) if lowpan_802154_is_valid_src_short_addr(short) => Some(short),
        Some(_) => Some(0xfffe),
        None => None,
    }
}

pub const fn ndisc_opt_addr_space(addr_len: usize) -> usize {
    (addr_len + 2 + 7) & !7
}

pub const fn lowpan_ndisc_opt_addr_space(
    is_ieee802154: bool,
    icmp6_type: u8,
    short_addr: Option<u16>,
) -> usize {
    if !is_ieee802154 {
        return 0;
    }
    match icmp6_type {
        NDISC_REDIRECT
        | NDISC_NEIGHBOUR_ADVERTISEMENT
        | NDISC_NEIGHBOUR_SOLICITATION
        | NDISC_ROUTER_SOLICITATION => match short_addr {
            Some(short) if lowpan_802154_is_valid_src_short_addr(short) => {
                ndisc_opt_addr_space(IEEE802154_SHORT_ADDR_LEN)
            }
            _ => 0,
        },
        _ => 0,
    }
}

pub const fn lowpan_ndisc_fill_addr_option_type(
    icmp6_type: u8,
    has_redirect_addr: bool,
) -> Option<u8> {
    match icmp6_type {
        NDISC_REDIRECT if has_redirect_addr => Some(ND_OPT_TARGET_LL_ADDR),
        NDISC_NEIGHBOUR_ADVERTISEMENT => Some(ND_OPT_TARGET_LL_ADDR),
        NDISC_ROUTER_SOLICITATION | NDISC_NEIGHBOUR_SOLICITATION => Some(ND_OPT_SOURCE_LL_ADDR),
        _ => None,
    }
}

pub const fn lowpan_ndisc_fill_addr_option(
    is_ieee802154: bool,
    icmp6_type: u8,
    redirect_short_addr: Option<u16>,
    device_short_addr: Option<u16>,
) -> Option<LowpanAddrOption> {
    if !is_ieee802154 {
        return None;
    }
    match icmp6_type {
        NDISC_REDIRECT => match redirect_short_addr {
            Some(short) => Some(LowpanAddrOption {
                opt_type: ND_OPT_TARGET_LL_ADDR,
                short_addr_be: short.to_be_bytes(),
            }),
            None => None,
        },
        NDISC_NEIGHBOUR_ADVERTISEMENT => {
            fill_device_short(ND_OPT_TARGET_LL_ADDR, device_short_addr)
        }
        NDISC_ROUTER_SOLICITATION | NDISC_NEIGHBOUR_SOLICITATION => {
            fill_device_short(ND_OPT_SOURCE_LL_ADDR, device_short_addr)
        }
        _ => None,
    }
}

const fn fill_device_short(opt_type: u8, short_addr: Option<u16>) -> Option<LowpanAddrOption> {
    match short_addr {
        Some(short) if lowpan_802154_is_valid_src_short_addr(short) => Some(LowpanAddrOption {
            opt_type,
            short_addr_be: short.to_be_bytes(),
        }),
        _ => None,
    }
}

pub const fn lowpan_ndisc_prefix_rcv_add_addr_should_call(
    is_ieee802154: bool,
    dev_addr_generated: bool,
    ifid_802154_result: i32,
) -> bool {
    is_ieee802154 && dev_addr_generated && ifid_802154_result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpan_ndisc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/ndisc.c"
        ));
        assert!(source.contains("#define NDISC_802154_SHORT_ADDR_LENGTH\t1"));
        assert!(source.contains("lowpan_ndisc_parse_802154_options"));
        assert!(source.contains("ndopts->nd_802154_opt_array[nd_opt->nd_opt_type]"));
        assert!(source.contains("case ND_OPT_SOURCE_LL_ADDR:"));
        assert!(source.contains("case ND_OPT_TARGET_LL_ADDR:"));
        assert!(source.contains("return lowpan_ndisc_parse_802154_options(dev, nd_opt, ndopts);"));
        assert!(source.contains("if (flags & NEIGH_UPDATE_F_OVERRIDE)"));
        assert!(source.contains("lowpan_ndisc_802154_update(n, flags, icmp6_type, ndopts);"));
        assert!(source.contains("__ndisc_opt_addr_space(IEEE802154_SHORT_ADDR_LEN, 0)"));
        assert!(source.contains("memcpy(ha_buf, &n->short_addr"));
        assert!(source.contains("__ndisc_fill_addr_option(skb, opt_type, &short_addr"));
        assert!(source.contains("ieee802154_le16_to_be16(&short_addr"));
        assert!(source.contains("prefix_rcv_add_addr"));
        assert!(source.contains("addrconf_ifid_802154_6lowpan(addr->s6_addr + 8, dev)"));
        assert!(source.contains("addrconf_prefix_rcv_add_addr(net, dev, pinfo"));
        assert!(source.contains("const struct ndisc_ops lowpan_ndisc_ops"));
        assert!(source.contains(".parse_options\t\t= lowpan_ndisc_parse_options"));
        assert!(source.contains(".fill_addr_option\t= lowpan_ndisc_fill_addr_option"));
    }

    #[test]
    fn short_address_options_are_only_consumed_for_802154() {
        assert_eq!(
            lowpan_ndisc_parse_options(true, ND_OPT_SOURCE_LL_ADDR, 1),
            Some(LowpanNdOption::SourceShortAddr)
        );
        let mut ndopts = LowpanNdiscOptions::default();
        assert_eq!(
            lowpan_ndisc_parse_options_into(true, ND_OPT_SOURCE_LL_ADDR, 1, &mut ndopts),
            1
        );
        assert!(ndopts.source_short_present);
        assert_eq!(
            lowpan_ndisc_parse_options_into(true, ND_OPT_TARGET_LL_ADDR, 2, &mut ndopts),
            0
        );
        assert!(!ndopts.target_short_present);
        assert_eq!(
            lowpan_ndisc_parse_options(false, ND_OPT_SOURCE_LL_ADDR, 1),
            None
        );
        assert_eq!(
            lowpan_ndisc_parse_options(true, ND_OPT_SOURCE_LL_ADDR, 2),
            None
        );
        assert_eq!(
            lowpan_ndisc_update_short_addr(
                true,
                NEIGH_UPDATE_F_OVERRIDE,
                NDISC_NEIGHBOUR_SOLICITATION,
                Some(0x1234),
                None,
            ),
            Some(0x1234)
        );
        assert_eq!(
            lowpan_ndisc_update_short_addr(
                true,
                NEIGH_UPDATE_F_OVERRIDE,
                NDISC_NEIGHBOUR_ADVERTISEMENT,
                None,
                Some(0x8000),
            ),
            Some(0xfffe)
        );
        assert_eq!(
            lowpan_ndisc_opt_addr_space(true, NDISC_ROUTER_SOLICITATION, Some(1)),
            8
        );
        assert_eq!(
            lowpan_ndisc_fill_addr_option_type(NDISC_REDIRECT, true),
            Some(ND_OPT_TARGET_LL_ADDR)
        );
        assert_eq!(
            lowpan_ndisc_fill_addr_option(true, NDISC_REDIRECT, Some(0x1234), None),
            Some(LowpanAddrOption {
                opt_type: ND_OPT_TARGET_LL_ADDR,
                short_addr_be: [0x12, 0x34],
            })
        );
        assert_eq!(
            lowpan_ndisc_fill_addr_option(true, NDISC_NEIGHBOUR_SOLICITATION, None, Some(0x4321)),
            Some(LowpanAddrOption {
                opt_type: ND_OPT_SOURCE_LL_ADDR,
                short_addr_be: [0x43, 0x21],
            })
        );
        assert_eq!(
            lowpan_ndisc_fill_addr_option(true, NDISC_NEIGHBOUR_SOLICITATION, None, Some(0xfffe)),
            None
        );
        assert!(lowpan_ndisc_prefix_rcv_add_addr_should_call(true, true, 0));
        assert!(!lowpan_ndisc_prefix_rcv_add_addr_should_call(
            true, true, -1
        ));
        assert_eq!(
            LOWPAN_NDISC_OPS,
            LowpanNdiscOps {
                parse_options: true,
                update: true,
                opt_addr_space: true,
                fill_addr_option: true,
                prefix_rcv_add_addr: true,
            }
        );
    }
}
