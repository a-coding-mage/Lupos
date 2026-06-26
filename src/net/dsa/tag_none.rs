//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/tag_none.c
//! test-origin: linux:vendor/linux/net/dsa/tag_none.c
//! DSA no-op tag driver.

use crate::net::skbuff::SkBuff;

pub const NONE_NAME: &str = "none";
pub const DSA_TAG_PROTO_NONE: u8 = 0;
pub const MODULE_DESCRIPTION: &str = "DSA no-op tag driver";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaDeviceOps {
    pub name: &'static str,
    pub proto: u8,
    pub xmit: fn(SkBuff, &str) -> SkBuff,
}

pub const NONE_OPS: DsaDeviceOps = DsaDeviceOps {
    name: NONE_NAME,
    proto: DSA_TAG_PROTO_NONE,
    xmit: dsa_user_notag_xmit,
};

pub fn dsa_user_notag_xmit(skb: SkBuff, _dev_name: &str) -> SkBuff {
    skb
}

pub fn module_aliases() -> [&'static str; 2] {
    ["dsa_tag:none", "dsa_tag:id-0"]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::skbuff::{alloc_skb, skb_put};

    #[test]
    fn dsa_tag_none_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/tag_none.c"
        ));
        assert!(source.contains("#define NONE_NAME"));
        assert!(source.contains("\"none\""));
        assert!(source.contains("static struct sk_buff *dsa_user_notag_xmit"));
        assert!(source.contains("return skb;"));
        assert!(source.contains(".name\t= NONE_NAME"));
        assert!(source.contains(".proto\t= DSA_TAG_PROTO_NONE"));
        assert!(source.contains(".xmit\t= dsa_user_notag_xmit"));
        assert!(source.contains("module_dsa_tag_driver(none_ops);"));
        assert!(source.contains("MODULE_ALIAS_DSA_TAG_DRIVER(DSA_TAG_PROTO_NONE, NONE_NAME);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"DSA no-op tag driver\");"));

        assert_eq!(NONE_OPS.name, "none");
        assert_eq!(NONE_OPS.proto, DSA_TAG_PROTO_NONE);
        assert_eq!(module_aliases(), ["dsa_tag:none", "dsa_tag:id-0"]);
    }

    #[test]
    fn dsa_user_notag_xmit_returns_original_skb_contents() {
        let mut skb = alloc_skb(8).unwrap();
        skb_put(&mut skb, 4).unwrap().copy_from_slice(&[1, 2, 3, 4]);

        let skb = (NONE_OPS.xmit)(skb, "swp0");
        assert_eq!(skb.data(), &[1, 2, 3, 4]);
        assert_eq!(skb.len, 4);
    }
}
