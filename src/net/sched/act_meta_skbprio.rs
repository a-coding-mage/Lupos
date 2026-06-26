//! linux-parity: complete
//! linux-source: vendor/linux/net/sched/act_meta_skbprio.c
//! test-origin: linux:vendor/linux/net/sched/act_meta_skbprio.c
//! IFE skb priority metadata operation.

use crate::include::uapi::errno::EINVAL;

pub const MODULE_AUTHOR: &str = "Jamal Hadi Salim(2015)";
pub const MODULE_DESCRIPTION: &str = "Inter-FE skb prio metadata action";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIAS: &str = "ife-meta-skbprio";

pub const IFE_META_PRIO: u16 = 3;
pub const NLA_U32: u16 = 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SkBuffPriority {
    pub priority: u32,
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

pub const IFE_PRIO_OPS: TcfMetaOps = TcfMetaOps {
    metaid: IFE_META_PRIO,
    metatype: NLA_U32,
    name: "skbprio",
    synopsis: "skb prio metadata",
};

pub fn skbprio_encode(skb: &SkBuffPriority, _e: &TcfMetaInfo) -> [u8; 4] {
    skb.priority.to_be_bytes()
}

pub fn skbprio_decode(skb: &mut SkBuffPriority, data: &[u8]) -> Result<(), i32> {
    let bytes: [u8; 4] = data
        .get(..4)
        .ok_or(-EINVAL)?
        .try_into()
        .map_err(|_| -EINVAL)?;
    skb.priority = u32::from_be_bytes(bytes);
    Ok(())
}

pub const fn skbprio_check(skb: &SkBuffPriority, _e: &TcfMetaInfo) -> bool {
    skb.priority != 0
}

pub const fn ifeprio_init_module() -> &'static TcfMetaOps {
    &IFE_PRIO_OPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_meta_skbprio_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sched/act_meta_skbprio.c"
        ));
        assert!(source.contains("net/sched/act_meta_prio.c IFE skb->priority metadata module"));
        assert!(source.contains("static int skbprio_check"));
        assert!(source.contains("return ife_check_meta_u32(skb->priority, e);"));
        assert!(source.contains("static int skbprio_encode"));
        assert!(source.contains("u32 ifeprio = skb->priority;"));
        assert!(source.contains("return ife_encode_meta_u32(ifeprio, skbdata, e);"));
        assert!(source.contains("static int skbprio_decode"));
        assert!(source.contains("u32 ifeprio = *(u32 *)data;"));
        assert!(source.contains("skb->priority = ntohl(ifeprio);"));
        assert!(source.contains(".metaid = IFE_META_PRIO"));
        assert!(source.contains(".metatype = NLA_U32"));
        assert!(source.contains(".name = \"skbprio\""));
        assert!(source.contains(".synopsis = \"skb prio metadata\""));
        assert!(source.contains("register_ife_op(&ife_prio_ops);"));
        assert!(source.contains("unregister_ife_op(&ife_prio_ops);"));
        assert!(source.contains("MODULE_ALIAS_IFE_META(\"skbprio\");"));

        assert_eq!(IFE_PRIO_OPS.metaid, IFE_META_PRIO);
        assert_eq!(IFE_PRIO_OPS.metatype, NLA_U32);
    }

    #[test]
    fn skbprio_encode_decode_uses_network_order() {
        let info = TcfMetaInfo { present: true };
        let skb = SkBuffPriority {
            priority: 0x0102_0304,
        };
        assert_eq!(skbprio_encode(&skb, &info), [1, 2, 3, 4]);
        assert!(skbprio_check(&skb, &info));

        let mut decoded = SkBuffPriority::default();
        assert_eq!(
            skbprio_decode(&mut decoded, &[0xaa, 0xbb, 0xcc, 0xdd]),
            Ok(())
        );
        assert_eq!(decoded.priority, 0xaabb_ccdd);
        assert_eq!(skbprio_decode(&mut decoded, &[1, 2]), Err(-EINVAL));
        assert_eq!(ifeprio_init_module(), &IFE_PRIO_OPS);
    }
}
