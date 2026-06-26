//! linux-parity: complete
//! linux-source: vendor/linux/net/sched/act_meta_mark.c
//! test-origin: linux:vendor/linux/net/sched/act_meta_mark.c
//! IFE skb mark metadata operation.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Jamal Hadi Salim(2015)";
pub const MODULE_DESCRIPTION: &str = "Inter-FE skb mark metadata module";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "ife-meta-skbmark";

pub const IFE_META_SKBMARK: u16 = 1;
pub const NLA_U32: u16 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SkBuffMark {
    pub mark: u32,
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

pub const IFE_SKBMARK_OPS: TcfMetaOps = TcfMetaOps {
    metaid: IFE_META_SKBMARK,
    metatype: NLA_U32,
    name: "skbmark",
    synopsis: "skb mark 32 bit metadata",
};

pub fn skbmark_encode(skb: &SkBuffMark, _e: &TcfMetaInfo) -> [u8; 4] {
    skb.mark.to_be_bytes()
}

pub fn skbmark_decode(skb: &mut SkBuffMark, data: &[u8]) -> Result<(), i32> {
    let bytes: [u8; 4] = data
        .get(..4)
        .ok_or(-EINVAL)?
        .try_into()
        .map_err(|_| -EINVAL)?;
    skb.mark = u32::from_be_bytes(bytes);
    Ok(())
}

pub const fn skbmark_check(skb: &SkBuffMark, _e: &TcfMetaInfo) -> bool {
    skb.mark != 0
}

pub const fn ifemark_init_module() -> &'static TcfMetaOps {
    &IFE_SKBMARK_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_meta_mark_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sched/act_meta_mark.c"
        ));
        assert!(source.contains("net/sched/act_meta_mark.c IFE skb->mark metadata module"));
        assert!(source.contains("static int skbmark_encode"));
        assert!(source.contains("u32 ifemark = skb->mark;"));
        assert!(source.contains("return ife_encode_meta_u32(ifemark, skbdata, e);"));
        assert!(source.contains("static int skbmark_decode"));
        assert!(source.contains("u32 ifemark = *(u32 *)data;"));
        assert!(source.contains("skb->mark = ntohl(ifemark);"));
        assert!(source.contains("return ife_check_meta_u32(skb->mark, e);"));
        assert!(source.contains(".metaid = IFE_META_SKBMARK"));
        assert!(source.contains(".metatype = NLA_U32"));
        assert!(source.contains(".name = \"skbmark\""));
        assert!(source.contains(".synopsis = \"skb mark 32 bit metadata\""));
        assert!(source.contains("register_ife_op(&ife_skbmark_ops);"));
        assert!(source.contains("unregister_ife_op(&ife_skbmark_ops);"));
        assert!(source.contains("MODULE_ALIAS_IFE_META(\"skbmark\");"));

        assert_eq!(IFE_SKBMARK_OPS.metaid, IFE_META_SKBMARK);
        assert_eq!(IFE_SKBMARK_OPS.metatype, NLA_U32);
    }

    #[test]
    fn skbmark_encode_decode_uses_network_order() {
        let info = TcfMetaInfo { present: true };
        let skb = SkBuffMark { mark: 0x1234_5678 };
        assert_eq!(skbmark_encode(&skb, &info), [0x12, 0x34, 0x56, 0x78]);
        assert!(skbmark_check(&skb, &info));

        let mut decoded = SkBuffMark::default();
        assert_eq!(
            skbmark_decode(&mut decoded, &[0xab, 0xcd, 0xef, 0x01]),
            Ok(())
        );
        assert_eq!(decoded.mark, 0xabcd_ef01);
        assert_eq!(skbmark_decode(&mut decoded, &[1, 2]), Err(-EINVAL));
        assert_eq!(ifemark_init_module(), &IFE_SKBMARK_OPS);
    }
}
