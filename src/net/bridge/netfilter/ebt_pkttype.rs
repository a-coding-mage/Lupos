//! linux-parity: complete
//! linux-source: vendor/linux/net/bridge/netfilter/ebt_pkttype.c
//! test-origin: linux:vendor/linux/net/bridge/netfilter/ebt_pkttype.c
//! Ebtables link-layer packet type match.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_DESCRIPTION: &str = "Ebtables: Link layer packet type match";
pub const MODULE_LICENSE: &str = "GPL";
pub const NFPROTO_BRIDGE: u8 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EbtPkttypeInfo {
    pub pkt_type: u8,
    pub invert: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const EBT_PKTTYPE_MT_REG: XtMatch = XtMatch {
    name: "pkttype",
    revision: 0,
    family: NFPROTO_BRIDGE,
    matchsize: core::mem::size_of::<EbtPkttypeInfo>(),
};

pub fn ebt_pkttype_mt(pkt_type: u8, info: EbtPkttypeInfo) -> bool {
    (pkt_type == info.pkt_type) ^ (info.invert != 0)
}

pub fn ebt_pkttype_mt_check(info: EbtPkttypeInfo) -> Result<(), i32> {
    if info.invert != 0 && info.invert != 1 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub const fn ebt_pkttype_init() -> &'static XtMatch {
    &EBT_PKTTYPE_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebt_pkttype_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bridge/netfilter/ebt_pkttype.c"
        ));
        assert!(source.contains("ebt_pkttype_mt(const struct sk_buff *skb"));
        assert!(source.contains("return (skb->pkt_type == info->pkt_type) ^ info->invert;"));
        assert!(source.contains("if (info->invert != 0 && info->invert != 1)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("/* Allow any pkt_type value */"));
        assert!(source.contains(".name\t\t= \"pkttype\""));
        assert!(source.contains(".family\t\t= NFPROTO_BRIDGE"));
        assert!(source.contains(".matchsize\t= sizeof(struct ebt_pkttype_info)"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Ebtables: Link layer packet type match\")"));

        let info = EbtPkttypeInfo {
            pkt_type: 2,
            invert: 0,
        };
        assert_eq!(ebt_pkttype_mt_check(info), Ok(()));
        assert!(ebt_pkttype_mt(2, info));
        assert!(!ebt_pkttype_mt(1, info));
        assert!(ebt_pkttype_mt(1, EbtPkttypeInfo { invert: 1, ..info }));
        assert_eq!(
            ebt_pkttype_mt_check(EbtPkttypeInfo { invert: 2, ..info }),
            Err(EINVAL)
        );
        assert_eq!(ebt_pkttype_init(), &EBT_PKTTYPE_MT_REG);
    }
}
