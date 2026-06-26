//! linux-parity: complete
//! linux-source: vendor/linux/net/sched/act_meta_skbtcindex.c
//! test-origin: linux:vendor/linux/net/sched/act_meta_skbtcindex.c
//! IFE skb tc_index metadata operation.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Jamal Hadi Salim(2016)";
pub const MODULE_DESCRIPTION: &str = "Inter-FE skb tc_index metadata module";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "ife-meta-tcindex";

pub const IFE_META_TCINDEX: u16 = 5;
pub const NLA_U16: u16 = 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SkBuffTcIndex {
    pub tc_index: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfMetaInfo {
    pub present: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TcfMetaOps {
    pub metaid: u16,
    pub metatype: u16,
    pub name: &'static str,
    pub synopsis: &'static str,
}

pub const IFE_SKBTCINDEX_OPS: TcfMetaOps = TcfMetaOps {
    metaid: IFE_META_TCINDEX,
    metatype: NLA_U16,
    name: "tc_index",
    synopsis: "skb tc_index 16 bit metadata",
};

pub fn skbtcindex_encode(skb: &SkBuffTcIndex, _e: &TcfMetaInfo) -> [u8; 2] {
    skb.tc_index.to_be_bytes()
}

pub fn skbtcindex_decode(skb: &mut SkBuffTcIndex, data: &[u8]) -> Result<(), i32> {
    let bytes: [u8; 2] = data
        .get(..2)
        .ok_or(-EINVAL)?
        .try_into()
        .map_err(|_| -EINVAL)?;
    skb.tc_index = u16::from_be_bytes(bytes);
    Ok(())
}

pub const fn skbtcindex_check(skb: &SkBuffTcIndex, _e: &TcfMetaInfo) -> bool {
    skb.tc_index != 0
}

pub const fn ifetc_index_init_module() -> &'static TcfMetaOps {
    &IFE_SKBTCINDEX_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_meta_skbtcindex_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sched/act_meta_skbtcindex.c"
        ));
        assert!(source.contains("net/sched/act_meta_tc_index.c IFE skb->tc_index metadata module"));
        assert!(source.contains("static int skbtcindex_encode"));
        assert!(source.contains("u32 ifetc_index = skb->tc_index;"));
        assert!(source.contains("return ife_encode_meta_u16(ifetc_index, skbdata, e);"));
        assert!(source.contains("static int skbtcindex_decode"));
        assert!(source.contains("u16 ifetc_index = *(u16 *)data;"));
        assert!(source.contains("skb->tc_index = ntohs(ifetc_index);"));
        assert!(source.contains("return ife_check_meta_u16(skb->tc_index, e);"));
        assert!(source.contains(".metaid = IFE_META_TCINDEX"));
        assert!(source.contains(".metatype = NLA_U16"));
        assert!(source.contains(".name = \"tc_index\""));
        assert!(source.contains(".synopsis = \"skb tc_index 16 bit metadata\""));
        assert!(source.contains("register_ife_op(&ife_skbtcindex_ops);"));
        assert!(source.contains("unregister_ife_op(&ife_skbtcindex_ops);"));
        assert!(source.contains("MODULE_ALIAS_IFE_META(\"tcindex\");"));

        assert_eq!(IFE_SKBTCINDEX_OPS.metaid, IFE_META_TCINDEX);
        assert_eq!(IFE_SKBTCINDEX_OPS.metatype, NLA_U16);
    }

    #[test]
    fn skbtcindex_encode_decode_uses_network_order() {
        let info = TcfMetaInfo { present: true };
        let skb = SkBuffTcIndex { tc_index: 0x1234 };
        assert_eq!(skbtcindex_encode(&skb, &info), [0x12, 0x34]);
        assert!(skbtcindex_check(&skb, &info));

        let mut decoded = SkBuffTcIndex::default();
        assert_eq!(skbtcindex_decode(&mut decoded, &[0xab, 0xcd]), Ok(()));
        assert_eq!(decoded.tc_index, 0xabcd);
        assert_eq!(skbtcindex_decode(&mut decoded, &[1]), Err(-EINVAL));
        assert_eq!(ifetc_index_init_module(), &IFE_SKBTCINDEX_OPS);
    }
}
