//! linux-parity: complete
//! linux-source: vendor/linux/net/rxrpc/utils.c
//! test-origin: linux:vendor/linux/net/rxrpc/utils.c
//! RxRPC peer-address extraction from packets.

use crate::include::uapi::errno::{EAFNOSUPPORT, EINVAL};
use crate::net::ip::{ETH_P_IP, ETH_P_IPV6, parse_ipv4_packet, parse_ipv6_packet};
use crate::net::skbuff::SkBuff;
use crate::net::socket::{AF_INET, AF_INET6, SOCK_DGRAM};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RxrpcTransportAddr {
    Inet {
        family: u16,
        port: u16,
        addr: u32,
    },
    Inet6 {
        family: u16,
        port: u16,
        addr: [u8; 16],
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SockaddrRxrpc {
    pub transport_type: u16,
    pub transport_len: usize,
    pub transport: RxrpcTransportAddr,
}

pub fn rxrpc_extract_addr_from_skb(protocol: u16, skb: &SkBuff) -> Result<SockaddrRxrpc, i32> {
    match protocol {
        ETH_P_IP => {
            let ip = parse_ipv4_packet(skb)?;
            let source_port = udp_source_port(&ip.payload)?;
            Ok(SockaddrRxrpc {
                transport_type: SOCK_DGRAM,
                transport_len: 16,
                transport: RxrpcTransportAddr::Inet {
                    family: AF_INET,
                    port: source_port,
                    addr: ip.src,
                },
            })
        }
        ETH_P_IPV6 => {
            let ip = parse_ipv6_packet(skb)?;
            let source_port = udp_source_port(&ip.payload)?;
            Ok(SockaddrRxrpc {
                transport_type: SOCK_DGRAM,
                transport_len: 28,
                transport: RxrpcTransportAddr::Inet6 {
                    family: AF_INET6,
                    port: source_port,
                    addr: ip.src,
                },
            })
        }
        _ => Err(EAFNOSUPPORT),
    }
}

fn udp_source_port(payload: &[u8]) -> Result<u16, i32> {
    if payload.len() < 2 {
        return Err(EINVAL);
    }
    Ok(u16::from_be_bytes([payload[0], payload[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fib::ipv4;
    use crate::net::ip::{IPPROTO_UDP, build_ipv4_packet, build_ipv6_packet};

    #[test]
    fn rxrpc_extract_addr_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/rxrpc/utils.c"
        ));
        assert!(source.contains("int rxrpc_extract_addr_from_skb"));
        assert!(source.contains("memset(srx, 0, sizeof(*srx));"));
        assert!(source.contains("case ETH_P_IP:"));
        assert!(source.contains("srx->transport_type = SOCK_DGRAM;"));
        assert!(source.contains("srx->transport.sin.sin_family = AF_INET;"));
        assert!(source.contains("srx->transport.sin.sin_port = udp_hdr(skb)->source;"));
        assert!(source.contains("srx->transport.sin.sin_addr.s_addr = ip_hdr(skb)->saddr;"));
        assert!(source.contains("case ETH_P_IPV6:"));
        assert!(source.contains("srx->transport.sin6.sin6_family = AF_INET6;"));
        assert!(source.contains("return -EAFNOSUPPORT;"));

        let udp = [0x12, 0x34, 0, 53, 0, 8, 0, 0];
        let skb =
            build_ipv4_packet(ipv4(10, 1, 2, 3), ipv4(10, 1, 2, 4), IPPROTO_UDP, &udp, 64).unwrap();
        assert_eq!(
            rxrpc_extract_addr_from_skb(ETH_P_IP, &skb).unwrap(),
            SockaddrRxrpc {
                transport_type: SOCK_DGRAM,
                transport_len: 16,
                transport: RxrpcTransportAddr::Inet {
                    family: AF_INET,
                    port: 0x1234,
                    addr: ipv4(10, 1, 2, 3),
                },
            }
        );

        let src = [0x20, 1, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let dst = [0x20, 1, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
        let skb = build_ipv6_packet(src, dst, IPPROTO_UDP, &udp, 64).unwrap();
        assert_eq!(
            rxrpc_extract_addr_from_skb(ETH_P_IPV6, &skb)
                .unwrap()
                .transport,
            RxrpcTransportAddr::Inet6 {
                family: AF_INET6,
                port: 0x1234,
                addr: src,
            }
        );
        assert_eq!(rxrpc_extract_addr_from_skb(0x1234, &skb), Err(EAFNOSUPPORT));
    }
}
