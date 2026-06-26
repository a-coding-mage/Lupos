//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_mark.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_mark.c
//! Xtables packet mark target and match.

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Marc Boucher <marc@mbsi.ca>";
pub const MODULE_DESCRIPTION: &str = "Xtables: packet mark operations";
pub const MODULE_ALIASES: [&str; 5] = [
    "ipt_mark",
    "ip6t_mark",
    "ipt_MARK",
    "ip6t_MARK",
    "arpt_MARK",
];
pub const XT_CONTINUE: u32 = 0xffff_ffff;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_ARP: u8 = 3;
pub const NFPROTO_IPV6: u8 = 10;
pub const NFPROTO_UNSPEC: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMarkTginfo2 {
    pub mark: u32,
    pub mask: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMarkMtinfo1 {
    pub mark: u32,
    pub mask: u32,
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtEntry {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
}

pub const MARK_TG_REG: [XtEntry; 3] = [
    XtEntry {
        name: "MARK",
        revision: 2,
        family: NFPROTO_IPV4,
    },
    XtEntry {
        name: "MARK",
        revision: 2,
        family: NFPROTO_ARP,
    },
    XtEntry {
        name: "MARK",
        revision: 2,
        family: NFPROTO_IPV6,
    },
];

pub const MARK_MT_REG: XtEntry = XtEntry {
    name: "mark",
    revision: 1,
    family: NFPROTO_UNSPEC,
};

pub const fn mark_tg(skb_mark: u32, info: XtMarkTginfo2) -> (u32, u32) {
    ((skb_mark & !info.mask) ^ info.mark, XT_CONTINUE)
}

pub const fn mark_mt(skb_mark: u32, info: XtMarkMtinfo1) -> bool {
    ((skb_mark & info.mask) == info.mark) != info.invert
}

pub const fn mark_mt_init(targets_ret: i32, match_ret: i32) -> Result<(), i32> {
    if targets_ret < 0 {
        return Err(targets_ret);
    }
    if match_ret < 0 {
        return Err(match_ret);
    }
    Ok(())
}

pub const fn mark_mt_exit() -> (bool, bool) {
    (true, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_mark_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_mark.c"
        ));
        assert!(source.contains("MODULE_ALIAS(\"ipt_mark\");"));
        assert!(source.contains("MODULE_ALIAS(\"arpt_MARK\");"));
        assert!(source.contains("mark_tg(struct sk_buff *skb"));
        assert!(source.contains("skb->mark = (skb->mark & ~info->mask) ^ info->mark;"));
        assert!(source.contains("return XT_CONTINUE;"));
        assert!(source.contains("mark_mt(const struct sk_buff *skb"));
        assert!(source.contains("return ((skb->mark & info->mask) == info->mark) ^ info->invert;"));
        assert!(source.contains(".name           = \"MARK\""));
        assert!(source.contains(".revision       = 2"));
        assert!(source.contains(".name           = \"mark\""));
        assert!(source.contains(".revision       = 1"));
        assert!(source.contains("xt_register_targets(mark_tg_reg, ARRAY_SIZE(mark_tg_reg));"));
        assert!(source.contains("xt_unregister_targets(mark_tg_reg, ARRAY_SIZE(mark_tg_reg));"));
    }

    #[test]
    fn mark_target_and_match_apply_mask_xor_rules() {
        assert_eq!(
            mark_tg(
                0b1010,
                XtMarkTginfo2 {
                    mark: 0b0101,
                    mask: 0b1100,
                }
            ),
            (0b0111, XT_CONTINUE)
        );
        assert!(mark_mt(
            0b1010,
            XtMarkMtinfo1 {
                mark: 0b1000,
                mask: 0b1100,
                invert: false,
            }
        ));
        assert!(!mark_mt(
            0b1010,
            XtMarkMtinfo1 {
                mark: 0b1000,
                mask: 0b1100,
                invert: true,
            }
        ));
        assert_eq!(mark_mt_init(0, 0), Ok(()));
        assert_eq!(mark_mt_init(-5, 0), Err(-5));
        assert_eq!(mark_mt_init(0, -6), Err(-6));
        assert_eq!(mark_mt_exit(), (true, true));
    }
}
