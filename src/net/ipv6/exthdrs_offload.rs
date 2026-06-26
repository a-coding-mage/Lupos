//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/exthdrs_offload.c
//! test-origin: linux:vendor/linux/net/ipv6/exthdrs_offload.c
//! IPv6 extension-header GSO/GRO offload registration.

extern crate alloc;

use alloc::vec::Vec;

pub const INET6_PROTO_GSO_EXTHDR: u32 = 0x1;
pub const IPPROTO_HOPOPTS: u8 = 0;
pub const IPPROTO_ROUTING: u8 = 43;
pub const IPPROTO_DSTOPTS: u8 = 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetOffload {
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ipv6ExthdrsOffloadInit {
    pub ret: i32,
    pub added: Vec<u8>,
    pub removed: Vec<u8>,
}

pub const RTHDR_OFFLOAD: NetOffload = NetOffload {
    flags: INET6_PROTO_GSO_EXTHDR,
};
pub const DSTOPT_OFFLOAD: NetOffload = NetOffload {
    flags: INET6_PROTO_GSO_EXTHDR,
};
pub const HBH_OFFLOAD: NetOffload = NetOffload {
    flags: INET6_PROTO_GSO_EXTHDR,
};
pub const EXTHDR_OFFLOAD_PROTOS: [u8; 3] = [IPPROTO_ROUTING, IPPROTO_DSTOPTS, IPPROTO_HOPOPTS];

pub fn ipv6_exthdrs_offload_init(fail_proto: Option<u8>) -> Ipv6ExthdrsOffloadInit {
    let mut added = Vec::new();
    let mut removed = Vec::new();

    for proto in EXTHDR_OFFLOAD_PROTOS {
        if fail_proto == Some(proto) {
            match proto {
                IPPROTO_DSTOPTS => removed.push(IPPROTO_ROUTING),
                IPPROTO_HOPOPTS => {
                    removed.push(IPPROTO_DSTOPTS);
                    removed.push(IPPROTO_ROUTING);
                }
                _ => {}
            }
            return Ipv6ExthdrsOffloadInit {
                ret: -1,
                added,
                removed,
            };
        }
        added.push(proto);
    }

    Ipv6ExthdrsOffloadInit {
        ret: 0,
        added,
        removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exthdrs_offload_init_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/exthdrs_offload.c"
        ));
        assert!(source.contains("static const struct net_offload rthdr_offload"));
        assert!(source.contains(".flags\t\t=\tINET6_PROTO_GSO_EXTHDR"));
        assert!(source.contains("inet6_add_offload(&rthdr_offload, IPPROTO_ROUTING);"));
        assert!(source.contains("inet6_add_offload(&dstopt_offload, IPPROTO_DSTOPTS);"));
        assert!(source.contains("inet6_add_offload(&hbh_offload, IPPROTO_HOPOPTS);"));
        assert!(source.contains("inet6_del_offload(&dstopt_offload, IPPROTO_DSTOPTS);"));
        assert!(source.contains("inet6_del_offload(&rthdr_offload, IPPROTO_ROUTING);"));

        assert_eq!(RTHDR_OFFLOAD.flags, INET6_PROTO_GSO_EXTHDR);
        assert_eq!(DSTOPT_OFFLOAD.flags, INET6_PROTO_GSO_EXTHDR);
        assert_eq!(HBH_OFFLOAD.flags, INET6_PROTO_GSO_EXTHDR);
        assert_eq!(
            ipv6_exthdrs_offload_init(None),
            Ipv6ExthdrsOffloadInit {
                ret: 0,
                added: alloc::vec![IPPROTO_ROUTING, IPPROTO_DSTOPTS, IPPROTO_HOPOPTS],
                removed: alloc::vec![],
            }
        );
        assert_eq!(
            ipv6_exthdrs_offload_init(Some(IPPROTO_HOPOPTS)),
            Ipv6ExthdrsOffloadInit {
                ret: -1,
                added: alloc::vec![IPPROTO_ROUTING, IPPROTO_DSTOPTS],
                removed: alloc::vec![IPPROTO_DSTOPTS, IPPROTO_ROUTING],
            }
        );
    }
}
