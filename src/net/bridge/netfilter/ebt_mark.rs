//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_mark.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_mark.c
//! Ebtables packet mark target.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_DESCRIPTION: &str = "Ebtables: Packet mark modification";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;
pub const EBT_RETURN: i32 = -4;
pub const NUM_STANDARD_TARGETS: i32 = 4;
pub const EBT_VERDICT_BITS: i32 = 0x0000_000f;
pub const MARK_SET_VALUE: i32 = -16;
pub const MARK_OR_VALUE: i32 = -32;
pub const MARK_AND_VALUE: i32 = -48;
pub const MARK_XOR_VALUE: i32 = -64;
pub const MARK_ACTION_MASK: i32 = -16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtMarkTInfo {
    pub mark: u64,
    pub target: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtMarkPacket {
    pub mark: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompatEbtMarkTInfo {
    pub mark: u32,
    pub target: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtTarget {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub targetsize: usize,
}

pub const EBT_MARK_TG_REG: XtTarget = XtTarget {
    name: "mark",
    revision: 0,
    family: NFPROTO_BRIDGE,
    targetsize: core::mem::size_of::<EbtMarkTInfo>(),
};

pub const fn ebt_mark_tg(packet: &mut EbtMarkPacket, info: EbtMarkTInfo) -> i32 {
    let action = info.target & MARK_ACTION_MASK;

    if action == MARK_SET_VALUE {
        packet.mark = info.mark;
    } else if action == MARK_OR_VALUE {
        packet.mark |= info.mark;
    } else if action == MARK_AND_VALUE {
        packet.mark &= info.mark;
    } else {
        packet.mark ^= info.mark;
    }

    info.target | !EBT_VERDICT_BITS
}

pub const fn ebt_mark_target(action: i32, verdict: i32) -> i32 {
    action | (verdict & EBT_VERDICT_BITS)
}

pub const fn ebt_invalid_target(target: i32) -> bool {
    target < -NUM_STANDARD_TARGETS || target >= 0
}

pub const fn ebt_mark_tg_check(base_chain: bool, info: EbtMarkTInfo) -> Result<(), i32> {
    let tmp = info.target | !EBT_VERDICT_BITS;
    if base_chain && tmp == EBT_RETURN {
        return Err(-EINVAL);
    }
    if ebt_invalid_target(tmp) {
        return Err(-EINVAL);
    }

    let action = info.target & !EBT_VERDICT_BITS;
    if action != MARK_SET_VALUE
        && action != MARK_OR_VALUE
        && action != MARK_AND_VALUE
        && action != MARK_XOR_VALUE
    {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn mark_tg_compat_from_user(user: CompatEbtMarkTInfo) -> EbtMarkTInfo {
    EbtMarkTInfo {
        mark: user.mark as u64,
        target: user.target as i32,
    }
}

pub const fn mark_tg_compat_to_user(kern: EbtMarkTInfo) -> CompatEbtMarkTInfo {
    CompatEbtMarkTInfo {
        mark: kern.mark as u32,
        target: kern.target as u32,
    }
}

pub const fn ebt_mark_init() -> &'static XtTarget {
    &EBT_MARK_TG_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebt_mark_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_mark.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter_bridge/ebt_mark_t.h"
        ));
        assert!(header.contains("#define MARK_SET_VALUE (0xfffffff0)"));
        assert!(header.contains("#define MARK_OR_VALUE  (0xffffffe0)"));
        assert!(header.contains("#define MARK_AND_VALUE (0xffffffd0)"));
        assert!(header.contains("#define MARK_XOR_VALUE (0xffffffc0)"));
        assert!(source.contains("ebt_mark_tg(struct sk_buff *skb"));
        assert!(source.contains("int action = info->target & -16;"));
        assert!(source.contains("if (action == MARK_SET_VALUE)"));
        assert!(source.contains("skb->mark = info->mark;"));
        assert!(source.contains("skb->mark |= info->mark;"));
        assert!(source.contains("skb->mark &= info->mark;"));
        assert!(source.contains("skb->mark ^= info->mark;"));
        assert!(source.contains("return info->target | ~EBT_VERDICT_BITS;"));
        assert!(source.contains("if (BASE_CHAIN && tmp == EBT_RETURN)"));
        assert!(source.contains("tmp = info->target & ~EBT_VERDICT_BITS;"));
        assert!(source.contains("struct compat_ebt_mark_t_info"));
        assert!(source.contains(".name\t\t= \"mark\""));
        assert!(source.contains(".family\t\t= NFPROTO_BRIDGE"));
        assert!(source.contains("xt_register_target(&ebt_mark_tg_reg);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Ebtables: Packet mark modification\")"));
    }

    #[test]
    fn mark_target_applies_set_or_and_xor_actions_and_validates_target() {
        let mut packet = EbtMarkPacket { mark: 0x0f };
        let set = EbtMarkTInfo {
            mark: 0x10,
            target: ebt_mark_target(MARK_SET_VALUE, -1),
        };
        assert_eq!(ebt_mark_tg(&mut packet, set), -1);
        assert_eq!(packet.mark, 0x10);

        let or = EbtMarkTInfo {
            mark: 0x03,
            target: ebt_mark_target(MARK_OR_VALUE, -2),
        };
        assert_eq!(ebt_mark_tg(&mut packet, or), -2);
        assert_eq!(packet.mark, 0x13);

        let and = EbtMarkTInfo {
            mark: 0x11,
            target: ebt_mark_target(MARK_AND_VALUE, -3),
        };
        assert_eq!(ebt_mark_tg(&mut packet, and), -3);
        assert_eq!(packet.mark, 0x11);

        let xor = EbtMarkTInfo {
            mark: 0x01,
            target: ebt_mark_target(MARK_XOR_VALUE, -1),
        };
        assert_eq!(ebt_mark_tg(&mut packet, xor), -1);
        assert_eq!(packet.mark, 0x10);
        assert_eq!(ebt_mark_tg_check(false, set), Ok(()));
        assert_eq!(
            ebt_mark_tg_check(
                true,
                EbtMarkTInfo {
                    target: ebt_mark_target(MARK_SET_VALUE, EBT_RETURN),
                    ..set
                }
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            ebt_mark_tg_check(false, EbtMarkTInfo { target: 0, ..set }),
            Err(-EINVAL)
        );
        assert_eq!(
            mark_tg_compat_from_user(CompatEbtMarkTInfo {
                mark: 7,
                target: ebt_mark_target(MARK_SET_VALUE, -1) as u32,
            }),
            EbtMarkTInfo {
                mark: 7,
                target: ebt_mark_target(MARK_SET_VALUE, -1),
            }
        );
        assert_eq!(ebt_mark_init(), &EBT_MARK_TG_REG);
    }
}
