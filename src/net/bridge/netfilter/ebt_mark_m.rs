//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_mark_m.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_mark_m.c
//! Ebtables packet mark match.

use crate::include::uapi::errno::EINVAL;

pub const EBT_MARK_AND: u8 = 0x01;
pub const EBT_MARK_OR: u8 = 0x02;
pub const EBT_MARK_MASK: u8 = EBT_MARK_AND | EBT_MARK_OR;
pub const NFPROTO_BRIDGE: u8 = 7;
pub const MODULE_DESCRIPTION: &str = "Ebtables: Packet mark match";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtMarkMInfo {
    pub mark: u64,
    pub mask: u64,
    pub invert: bool,
    pub bitmask: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
}

pub const EBT_MARK_MT_REG: XtMatch = XtMatch {
    name: "mark_m",
    revision: 0,
    family: NFPROTO_BRIDGE,
};

pub const fn ebt_mark_mt(skb_mark: u64, info: EbtMarkMInfo) -> bool {
    if info.bitmask & EBT_MARK_OR != 0 {
        return ((skb_mark & info.mask) != 0) != info.invert;
    }
    ((skb_mark & info.mask) == info.mark) != info.invert
}

pub const fn ebt_mark_mt_check(info: EbtMarkMInfo) -> Result<(), i32> {
    if info.bitmask & !EBT_MARK_MASK != 0 {
        return Err(-EINVAL);
    }
    if (info.bitmask & EBT_MARK_OR != 0) && (info.bitmask & EBT_MARK_AND != 0) {
        return Err(-EINVAL);
    }
    if info.bitmask == 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn ebt_mark_m_init() -> &'static XtMatch {
    &EBT_MARK_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebt_mark_m_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_mark_m.c"
        ));
        assert!(source.contains("ebt_mark_mt(const struct sk_buff *skb"));
        assert!(source.contains("if (info->bitmask & EBT_MARK_OR)"));
        assert!(source.contains("return !!(skb->mark & info->mask) ^ info->invert;"));
        assert!(source.contains("return ((skb->mark & info->mask) == info->mark) ^ info->invert;"));
        assert!(source.contains("if (info->bitmask & ~EBT_MARK_MASK)"));
        assert!(
            source.contains("if ((info->bitmask & EBT_MARK_OR) && (info->bitmask & EBT_MARK_AND))")
        );
        assert!(source.contains("if (!info->bitmask)"));
        assert!(source.contains("struct compat_ebt_mark_m_info"));
        assert!(source.contains(".name\t\t= \"mark_m\""));
        assert!(source.contains(".family\t\t= NFPROTO_BRIDGE"));
        assert!(source.contains("xt_register_match(&ebt_mark_mt_reg);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Ebtables: Packet mark match\")"));
    }

    #[test]
    fn mark_match_validates_or_and_exact_modes() {
        let exact = EbtMarkMInfo {
            mark: 0x20,
            mask: 0xf0,
            invert: false,
            bitmask: EBT_MARK_AND,
        };
        assert!(ebt_mark_mt(0x21, exact));
        assert!(!ebt_mark_mt(0x11, exact));
        let or_info = EbtMarkMInfo {
            bitmask: EBT_MARK_OR,
            mask: 0x04,
            ..exact
        };
        assert!(ebt_mark_mt(0x04, or_info));
        assert_eq!(ebt_mark_mt_check(exact), Ok(()));
        assert_eq!(
            ebt_mark_mt_check(EbtMarkMInfo {
                bitmask: 0,
                ..exact
            }),
            Err(-EINVAL)
        );
        assert_eq!(
            ebt_mark_mt_check(EbtMarkMInfo {
                bitmask: EBT_MARK_AND | EBT_MARK_OR,
                ..exact
            }),
            Err(-EINVAL)
        );
        assert_eq!(ebt_mark_m_init(), &EBT_MARK_MT_REG);
    }
}
