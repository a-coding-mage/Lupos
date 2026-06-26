//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_conntrack_proto_generic.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_conntrack_proto_generic.c
//! Generic conntrack protocol timeout defaults.

pub const HZ: u32 = crate::kernel::time::jiffies::HZ as u32;
pub const NF_CT_GENERIC_TIMEOUT: u32 = 600 * HZ;
pub const NF_CONNTRACK_GENERIC_L4PROTO: u8 = 255;
pub const CTA_TIMEOUT_GENERIC_TIMEOUT: u16 = 1;
pub const CTA_TIMEOUT_GENERIC_MAX: u16 = CTA_TIMEOUT_GENERIC_TIMEOUT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfGenericNet {
    pub timeout: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenericTimeoutAttr {
    pub timeout_seconds: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfConntrackL4Proto {
    pub l4proto: u8,
    pub allow_clash: bool,
    pub nlattr_max: u16,
    pub obj_size: usize,
}

pub const NF_CONNTRACK_L4PROTO_GENERIC: NfConntrackL4Proto = NfConntrackL4Proto {
    l4proto: NF_CONNTRACK_GENERIC_L4PROTO,
    allow_clash: true,
    nlattr_max: CTA_TIMEOUT_GENERIC_MAX,
    obj_size: core::mem::size_of::<u32>(),
};

pub const fn nf_conntrack_generic_init_net() -> NfGenericNet {
    NfGenericNet {
        timeout: NF_CT_GENERIC_TIMEOUT,
    }
}

pub const fn generic_timeout_nlattr_to_obj(attr: GenericTimeoutAttr, net: NfGenericNet) -> u32 {
    match attr.timeout_seconds {
        Some(timeout) => timeout * HZ,
        None => net.timeout,
    }
}

pub const fn generic_timeout_obj_to_nlattr(timeout: u32) -> u32 {
    timeout / HZ
}

pub const fn nf_conntrack_generic_l4proto() -> &'static NfConntrackL4Proto {
    &NF_CONNTRACK_L4PROTO_GENERIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_conntrack_proto_generic_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_conntrack_proto_generic.c"
        ));
        assert!(source.contains("static const unsigned int nf_ct_generic_timeout = 600*HZ;"));
        assert!(source.contains("void nf_conntrack_generic_init_net(struct net *net)"));
        assert!(source.contains("gn->timeout = nf_ct_generic_timeout;"));
        assert!(source.contains(".l4proto\t\t= 255"));
        assert!(source.contains(".allow_clash            = true"));
        assert!(source.contains("generic_timeout_nlattr_to_obj"));
        assert!(source.contains("CTA_TIMEOUT_GENERIC_TIMEOUT"));
        assert!(source.contains("ntohl(nla_get_be32(tb[CTA_TIMEOUT_GENERIC_TIMEOUT])) * HZ"));
        assert!(source.contains("htonl(*timeout / HZ)"));
        assert!(source.contains(".obj_size\t= sizeof(unsigned int)"));

        let net = nf_conntrack_generic_init_net();
        assert_eq!(net.timeout, 600 * HZ);
        assert_eq!(
            generic_timeout_nlattr_to_obj(
                GenericTimeoutAttr {
                    timeout_seconds: Some(7)
                },
                net
            ),
            7 * HZ
        );
        assert_eq!(
            generic_timeout_nlattr_to_obj(
                GenericTimeoutAttr {
                    timeout_seconds: None
                },
                net
            ),
            net.timeout
        );
        assert_eq!(generic_timeout_obj_to_nlattr(12 * HZ), 12);
        assert_eq!(nf_conntrack_generic_l4proto().l4proto, 255);
    }
}
