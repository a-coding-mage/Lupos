//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/netlink.c
//! test-origin: linux:vendor/linux/net/ipv4/netlink.c
//! IPv4 route netlink protocol parser.

use crate::include::uapi::errno::EOPNOTSUPP;
use crate::net::ip::{IPPROTO_ICMP, IPPROTO_ICMPV6, IPPROTO_TCP, IPPROTO_UDP};
use crate::net::socket::{AF_INET, AF_INET6};

pub const fn rtm_getroute_parse_ip_proto(ip_proto: u8, family: u16) -> Result<u8, i32> {
    match ip_proto {
        IPPROTO_TCP | IPPROTO_UDP => Ok(ip_proto),
        IPPROTO_ICMP if family == AF_INET => Ok(ip_proto),
        IPPROTO_ICMPV6 if family == AF_INET6 => Ok(ip_proto),
        _ => Err(EOPNOTSUPP),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtm_getroute_parse_ip_proto_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/netlink.c"
        ));
        assert!(source.contains("*ip_proto = nla_get_u8(attr);"));
        assert!(source.contains("case IPPROTO_TCP:"));
        assert!(source.contains("case IPPROTO_UDP:"));
        assert!(source.contains("case IPPROTO_ICMP:"));
        assert!(source.contains("case IPPROTO_ICMPV6:"));
        assert!(source.contains("Unsupported ip proto"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert_eq!(
            rtm_getroute_parse_ip_proto(IPPROTO_TCP, AF_INET),
            Ok(IPPROTO_TCP)
        );
        assert_eq!(
            rtm_getroute_parse_ip_proto(IPPROTO_UDP, AF_INET6),
            Ok(IPPROTO_UDP)
        );
        assert_eq!(
            rtm_getroute_parse_ip_proto(IPPROTO_ICMP, AF_INET),
            Ok(IPPROTO_ICMP)
        );
        assert_eq!(
            rtm_getroute_parse_ip_proto(IPPROTO_ICMP, AF_INET6),
            Err(EOPNOTSUPP)
        );
        assert_eq!(
            rtm_getroute_parse_ip_proto(IPPROTO_ICMPV6, AF_INET6),
            Ok(IPPROTO_ICMPV6)
        );
        assert_eq!(rtm_getroute_parse_ip_proto(255, AF_INET), Err(EOPNOTSUPP));
    }
}
