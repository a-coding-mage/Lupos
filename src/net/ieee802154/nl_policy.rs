//! linux-parity: complete
//! linux-source: vendor/linux/net/ieee802154/nl_policy.c
//! test-origin: linux:vendor/linux/net/ieee802154/nl_policy.c
//! IEEE 802.15.4 netlink attribute policy.

pub const IEEE802154_ATTR_MAX: usize = 56;
pub const IEEE802154_ATTR_DEV_NAME: usize = 1;
pub const IEEE802154_ATTR_DEV_INDEX: usize = 2;
pub const IEEE802154_ATTR_STATUS: usize = 3;
pub const IEEE802154_ATTR_HW_ADDR: usize = 5;
pub const IEEE802154_ATTR_CHANNEL: usize = 7;
pub const IEEE802154_ATTR_ED_LIST: usize = 22;
pub const IEEE802154_ATTR_PHY_NAME: usize = 31;
pub const IEEE802154_ATTR_TXPOWER: usize = 33;
pub const IEEE802154_ATTR_CCA_ED_LEVEL: usize = 36;
pub const IEEE802154_ATTR_LLSEC_KEY_BYTES: usize = 48;
pub const IEEE802154_ATTR_LLSEC_KEY_USAGE_COMMANDS: usize = 50;

pub const NLA_UNSPEC: u16 = 0;
pub const NLA_U8: u16 = 1;
pub const NLA_U16: u16 = 2;
pub const NLA_U32: u16 = 3;
pub const NLA_U64: u16 = 4;
pub const NLA_STRING: u16 = 5;
pub const NLA_S8: u16 = 12;
pub const NLA_S32: u16 = 14;
pub const NLA_HW_ADDR: u16 = NLA_U64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NlaPolicy {
    pub nla_type: u16,
    pub len: usize,
}

pub const IEEE802154_POLICY_LEN: usize = IEEE802154_ATTR_MAX + 1;

pub const fn ieee802154_policy(attr: usize) -> NlaPolicy {
    match attr {
        1 | 31 => NlaPolicy {
            nla_type: NLA_STRING,
            len: 0,
        },
        2 | 20 | 44 | 47 => NlaPolicy {
            nla_type: NLA_U32,
            len: 0,
        },
        4 | 6 | 8 | 10 | 11 | 13 | 14 | 16 => NlaPolicy {
            nla_type: NLA_U16,
            len: 0,
        },
        5 | 9 | 12 | 15 | 45 => NlaPolicy {
            nla_type: NLA_HW_ADDR,
            len: 0,
        },
        3 | 7 | 23 | 24 | 25 | 26 | 27 | 29 | 32 | 34 | 35 | 37 | 38 | 39 | 41 | 42 | 43 | 46
        | 49 | 51 | 52 | 53 | 54 | 55 => NlaPolicy {
            nla_type: NLA_U8,
            len: 0,
        },
        33 | 40 => NlaPolicy {
            nla_type: NLA_S8,
            len: 0,
        },
        36 => NlaPolicy {
            nla_type: NLA_S32,
            len: 0,
        },
        22 => NlaPolicy {
            nla_type: NLA_UNSPEC,
            len: 27,
        },
        30 => NlaPolicy {
            nla_type: NLA_UNSPEC,
            len: 32 * 4,
        },
        48 => NlaPolicy {
            nla_type: NLA_UNSPEC,
            len: 16,
        },
        50 => NlaPolicy {
            nla_type: NLA_UNSPEC,
            len: 258 / 8,
        },
        _ => NlaPolicy {
            nla_type: NLA_UNSPEC,
            len: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee802154_policy_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ieee802154/nl_policy.c"
        ));
        assert!(source.contains("#define NLA_HW_ADDR NLA_U64"));
        assert!(
            source.contains("const struct nla_policy ieee802154_policy[IEEE802154_ATTR_MAX + 1]")
        );
        assert!(source.contains("[IEEE802154_ATTR_DEV_NAME] = { .type = NLA_STRING, }"));
        assert!(source.contains("[IEEE802154_ATTR_DEV_INDEX] = { .type = NLA_U32, }"));
        assert!(source.contains("[IEEE802154_ATTR_HW_ADDR] = { .type = NLA_HW_ADDR, }"));
        assert!(source.contains("[IEEE802154_ATTR_ED_LIST] = { .len = 27 }"));
        assert!(source.contains("[IEEE802154_ATTR_CHANNEL_PAGE_LIST] = { .len = 32 * 4, }"));
        assert!(source.contains("[IEEE802154_ATTR_TXPOWER] = { .type = NLA_S8, }"));
        assert!(source.contains("[IEEE802154_ATTR_CCA_ED_LEVEL] = { .type = NLA_S32, }"));
        assert!(source.contains("[IEEE802154_ATTR_LLSEC_KEY_BYTES] = { .len = 16, }"));
        assert!(source.contains("[IEEE802154_ATTR_LLSEC_KEY_USAGE_COMMANDS] = { .len = 258 / 8 }"));

        assert_eq!(IEEE802154_POLICY_LEN, 57);
        assert_eq!(
            ieee802154_policy(IEEE802154_ATTR_DEV_NAME).nla_type,
            NLA_STRING
        );
        assert_eq!(
            ieee802154_policy(IEEE802154_ATTR_DEV_INDEX).nla_type,
            NLA_U32
        );
        assert_eq!(
            ieee802154_policy(IEEE802154_ATTR_HW_ADDR).nla_type,
            NLA_HW_ADDR
        );
        assert_eq!(ieee802154_policy(IEEE802154_ATTR_ED_LIST).len, 27);
        assert_eq!(ieee802154_policy(IEEE802154_ATTR_LLSEC_KEY_BYTES).len, 16);
        assert_eq!(
            ieee802154_policy(IEEE802154_ATTR_LLSEC_KEY_USAGE_COMMANDS).len,
            32
        );
    }
}
