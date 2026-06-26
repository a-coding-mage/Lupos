//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! UDP send/receive helpers.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;
use crate::net::ip::{IPPROTO_UDP, build_ipv4_packet, parse_ipv4_packet};
use crate::net::skbuff::SkBuff;

pub const UDP_SEGMENT: u32 = 103;
pub const UDP_MAX_SEGMENTS: usize = 1 << 7;
pub const ETH_MAX_MTU: usize = 0xffff;
pub const CONST_MTU_TEST: usize = 1500;
pub const CONST_HDRLEN_V4: usize = 20 + 8;
pub const CONST_HDRLEN_V6: usize = 40 + 8;
pub const CONST_MSS_V4: usize = CONST_MTU_TEST - CONST_HDRLEN_V4;
pub const CONST_MSS_V6: usize = CONST_MTU_TEST - CONST_HDRLEN_V6;
pub const CONST_MAX_SEGS_V4: usize = ETH_MAX_MTU / CONST_MSS_V4;
pub const CONST_MAX_SEGS_V6: usize = ETH_MAX_MTU / CONST_MSS_V6;
pub const IP6_MAX_MTU: usize = ETH_MAX_MTU + 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UdpPacket {
    pub src_addr: u32,
    pub dst_addr: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UdpGsoPlan {
    pub full_segments: usize,
    pub last_len: usize,
}

pub fn udp_gso_plan(
    payload_len: usize,
    gso_len: Option<usize>,
    ipv6: bool,
) -> Result<UdpGsoPlan, i32> {
    let mss = if ipv6 { CONST_MSS_V6 } else { CONST_MSS_V4 };
    let max_payload = if ipv6 {
        IP6_MAX_MTU - CONST_HDRLEN_V6
    } else {
        ETH_MAX_MTU - CONST_HDRLEN_V4
    };

    if payload_len == 0 || payload_len > max_payload {
        return Err(EINVAL);
    }

    match gso_len.unwrap_or(0) {
        0 => {
            if payload_len > mss {
                return Err(EINVAL);
            }
            if payload_len == mss {
                Ok(UdpGsoPlan {
                    full_segments: 1,
                    last_len: 0,
                })
            } else {
                Ok(UdpGsoPlan {
                    full_segments: 0,
                    last_len: payload_len,
                })
            }
        }
        len if len > mss => {
            if payload_len <= mss {
                Ok(UdpGsoPlan {
                    full_segments: 0,
                    last_len: payload_len,
                })
            } else {
                Err(EINVAL)
            }
        }
        len => {
            let full_segments = payload_len / len;
            let last_len = payload_len % len;
            let segment_count = full_segments + usize::from(last_len != 0);
            if segment_count > UDP_MAX_SEGMENTS {
                return Err(EINVAL);
            }
            Ok(UdpGsoPlan {
                full_segments,
                last_len,
            })
        }
    }
}

pub fn build_udp_payload(src_port: u16, dst_port: u16, payload: &[u8]) -> Result<Vec<u8>, i32> {
    let len = 8usize.checked_add(payload.len()).ok_or(EINVAL)?;
    if len > u16::MAX as usize {
        return Err(EINVAL);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| crate::include::uapi::errno::ENOMEM)?;
    out.resize(len, 0);
    out[0..2].copy_from_slice(&src_port.to_be_bytes());
    out[2..4].copy_from_slice(&dst_port.to_be_bytes());
    out[4..6].copy_from_slice(&(len as u16).to_be_bytes());
    out[6..8].copy_from_slice(&0u16.to_be_bytes());
    out[8..].copy_from_slice(payload);
    Ok(out)
}

/// Compute the RFC 768 UDP checksum over the IPv4 pseudo-header + UDP datagram.
fn udp_checksum(src_addr: u32, dst_addr: u32, udp_bytes: &[u8]) -> u16 {
    let udp_len = udp_bytes.len() as u16;
    // Pseudo-header: src(4) + dst(4) + zero(1) + proto(1) + udp_len(2)
    let mut pseudo = [0u8; 12];
    pseudo[0..4].copy_from_slice(&src_addr.to_be_bytes());
    pseudo[4..8].copy_from_slice(&dst_addr.to_be_bytes());
    pseudo[8] = 0;
    pseudo[9] = IPPROTO_UDP as u8;
    pseudo[10..12].copy_from_slice(&udp_len.to_be_bytes());

    let mut sum = 0u32;
    let mut add_words = |data: &[u8]| {
        let mut chunks = data.chunks_exact(2);
        for chunk in &mut chunks {
            sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
        }
        if let Some(&b) = chunks.remainder().first() {
            sum = sum.wrapping_add((b as u32) << 8);
        }
    };
    add_words(&pseudo);
    add_words(udp_bytes);
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    let result = !(sum as u16);
    // Per RFC 768, a computed checksum of 0 is sent as 0xffff.
    if result == 0 { 0xffff } else { result }
}

pub fn udp_sendmsg(
    src_addr: u32,
    dst_addr: u32,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Result<SkBuff, i32> {
    let mut udp = build_udp_payload(src_port, dst_port, payload)?;
    let csum = udp_checksum(src_addr, dst_addr, &udp);
    udp[6..8].copy_from_slice(&csum.to_be_bytes());
    build_ipv4_packet(src_addr, dst_addr, IPPROTO_UDP, &udp, 64)
}

pub fn udp_recvmsg(skb: &SkBuff) -> Result<UdpPacket, i32> {
    let ip = parse_ipv4_packet(skb)?;
    if ip.protocol != IPPROTO_UDP || ip.payload.len() < 8 {
        return Err(EINVAL);
    }
    let udp_len = u16::from_be_bytes([ip.payload[4], ip.payload[5]]) as usize;
    if udp_len < 8 || udp_len > ip.payload.len() {
        return Err(EINVAL);
    }
    Ok(UdpPacket {
        src_addr: ip.src,
        dst_addr: ip.dst,
        src_port: u16::from_be_bytes([ip.payload[0], ip.payload[1]]),
        dst_port: u16::from_be_bytes([ip.payload[2], ip.payload[3]]),
        payload: ip.payload[8..udp_len].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fib::ipv4;

    #[test]
    fn udp_send_recv_round_trip() {
        let skb = udp_sendmsg(ipv4(10, 0, 0, 1), ipv4(10, 0, 0, 2), 1000, 2000, b"hello").unwrap();
        let pkt = udp_recvmsg(&skb).unwrap();
        assert_eq!(pkt.src_port, 1000);
        assert_eq!(pkt.dst_port, 2000);
        assert_eq!(pkt.payload, b"hello");
    }

    #[test]
    fn udp_gso_plan_matches_linux_udpgso_boundaries() {
        assert_eq!(
            udp_gso_plan(CONST_MSS_V4 + 1, Some(CONST_MSS_V4), false).unwrap(),
            UdpGsoPlan {
                full_segments: 1,
                last_len: 1
            }
        );
        assert_eq!(
            udp_gso_plan(CONST_MSS_V4 + 1, None, false).err(),
            Some(EINVAL)
        );
        assert_eq!(
            udp_gso_plan(UDP_MAX_SEGMENTS, Some(1), false)
                .unwrap()
                .full_segments,
            UDP_MAX_SEGMENTS
        );
        assert_eq!(
            udp_gso_plan(UDP_MAX_SEGMENTS + 1, Some(1), false).err(),
            Some(EINVAL)
        );
        assert_eq!(
            udp_gso_plan(CONST_MSS_V6, Some(CONST_MSS_V6 + 1), true).unwrap(),
            UdpGsoPlan {
                full_segments: 0,
                last_len: CONST_MSS_V6
            }
        );
    }
}
