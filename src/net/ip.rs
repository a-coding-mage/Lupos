//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! IPv4/IPv6 and ICMP packet helpers.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;
use crate::net::skbuff::{SkBuff, alloc_skb, skb_put};

pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_IPV6: u16 = 0x86dd;
pub const IPPROTO_ICMP: u8 = 1;
pub const IPPROTO_TCP: u8 = 6;
pub const IPPROTO_UDP: u8 = 17;
pub const IPPROTO_ICMPV6: u8 = 58;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ipv4Packet {
    pub src: u32,
    pub dst: u32,
    pub ttl: u8,
    pub protocol: u8,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ipv6Packet {
    pub src: [u8; 16],
    pub dst: [u8; 16],
    pub next_header: u8,
    pub hop_limit: u8,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcmpPacket {
    pub icmp_type: u8,
    pub code: u8,
    pub payload: Vec<u8>,
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    if let Some(&byte) = chunks.remainder().first() {
        sum = sum.wrapping_add((byte as u32) << 8);
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn build_ipv4_packet(
    src: u32,
    dst: u32,
    protocol: u8,
    payload: &[u8],
    ttl: u8,
) -> Result<SkBuff, i32> {
    let total_len = 20usize.checked_add(payload.len()).ok_or(EINVAL)?;
    if total_len > u16::MAX as usize {
        return Err(EINVAL);
    }

    let mut skb = alloc_skb(total_len)?;
    let out = skb_put(&mut skb, total_len)?;
    out[0] = 0x45;
    out[1] = 0;
    out[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    out[4..6].copy_from_slice(&0u16.to_be_bytes());
    out[6..8].copy_from_slice(&0x4000u16.to_be_bytes());
    out[8] = ttl;
    out[9] = protocol;
    out[10..12].copy_from_slice(&0u16.to_be_bytes());
    out[12..16].copy_from_slice(&src.to_be_bytes());
    out[16..20].copy_from_slice(&dst.to_be_bytes());
    let csum = checksum(&out[..20]);
    out[10..12].copy_from_slice(&csum.to_be_bytes());
    out[20..].copy_from_slice(payload);
    Ok(skb)
}

pub fn parse_ipv4_packet(skb: &SkBuff) -> Result<Ipv4Packet, i32> {
    let data = skb.data();
    if data.len() < 20 || data[0] >> 4 != 4 {
        return Err(EINVAL);
    }
    let ihl = ((data[0] & 0x0f) as usize) * 4;
    if ihl < 20 || data.len() < ihl {
        return Err(EINVAL);
    }
    let total_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    if total_len < ihl || total_len > data.len() {
        return Err(EINVAL);
    }
    if checksum(&data[..ihl]) != 0 {
        return Err(EINVAL);
    }

    Ok(Ipv4Packet {
        src: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
        dst: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
        ttl: data[8],
        protocol: data[9],
        payload: data[ihl..total_len].to_vec(),
    })
}

pub fn build_ipv6_packet(
    src: [u8; 16],
    dst: [u8; 16],
    next_header: u8,
    payload: &[u8],
    hop_limit: u8,
) -> Result<SkBuff, i32> {
    if payload.len() > u16::MAX as usize {
        return Err(EINVAL);
    }
    let total_len = 40 + payload.len();
    let mut skb = alloc_skb(total_len)?;
    let out = skb_put(&mut skb, total_len)?;
    out[0] = 0x60;
    out[1] = 0;
    out[2] = 0;
    out[3] = 0;
    out[4..6].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    out[6] = next_header;
    out[7] = hop_limit;
    out[8..24].copy_from_slice(&src);
    out[24..40].copy_from_slice(&dst);
    out[40..].copy_from_slice(payload);
    Ok(skb)
}

pub fn parse_ipv6_packet(skb: &SkBuff) -> Result<Ipv6Packet, i32> {
    let data = skb.data();
    if data.len() < 40 || data[0] >> 4 != 6 {
        return Err(EINVAL);
    }
    let payload_len = u16::from_be_bytes([data[4], data[5]]) as usize;
    if data.len() < 40 + payload_len {
        return Err(EINVAL);
    }
    let mut src = [0u8; 16];
    let mut dst = [0u8; 16];
    src.copy_from_slice(&data[8..24]);
    dst.copy_from_slice(&data[24..40]);
    Ok(Ipv6Packet {
        src,
        dst,
        next_header: data[6],
        hop_limit: data[7],
        payload: data[40..40 + payload_len].to_vec(),
    })
}

pub fn build_icmp_echo(ident: u16, seq: u16, payload: &[u8], reply: bool) -> Result<Vec<u8>, i32> {
    let mut out = Vec::new();
    out.try_reserve_exact(8 + payload.len())
        .map_err(|_| crate::include::uapi::errno::ENOMEM)?;
    out.resize(8 + payload.len(), 0);
    out[0] = if reply { 0 } else { 8 };
    out[1] = 0;
    out[4..6].copy_from_slice(&ident.to_be_bytes());
    out[6..8].copy_from_slice(&seq.to_be_bytes());
    out[8..].copy_from_slice(payload);
    let csum = checksum(&out);
    out[2..4].copy_from_slice(&csum.to_be_bytes());
    Ok(out)
}

pub fn parse_icmp(data: &[u8]) -> Result<IcmpPacket, i32> {
    if data.len() < 4 || checksum(data) != 0 {
        return Err(EINVAL);
    }
    Ok(IcmpPacket {
        icmp_type: data[0],
        code: data[1],
        payload: data[4..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fib::ipv4;

    #[test]
    fn ipv4_packet_round_trip_verifies_checksum() {
        let skb = build_ipv4_packet(
            ipv4(10, 0, 0, 1),
            ipv4(10, 0, 0, 2),
            IPPROTO_UDP,
            b"abc",
            64,
        )
        .unwrap();
        let pkt = parse_ipv4_packet(&skb).unwrap();
        assert_eq!(pkt.src, ipv4(10, 0, 0, 1));
        assert_eq!(pkt.dst, ipv4(10, 0, 0, 2));
        assert_eq!(pkt.protocol, IPPROTO_UDP);
        assert_eq!(pkt.payload, b"abc");
    }

    #[test]
    fn ipv6_and_icmp_helpers_round_trip() {
        let src = [1u8; 16];
        let dst = [2u8; 16];
        let skb = build_ipv6_packet(src, dst, IPPROTO_ICMPV6, b"payload", 64).unwrap();
        let pkt = parse_ipv6_packet(&skb).unwrap();
        assert_eq!(pkt.src, src);
        assert_eq!(pkt.dst, dst);
        assert_eq!(pkt.payload, b"payload");

        let icmp = build_icmp_echo(7, 1, b"ping", false).unwrap();
        assert_eq!(parse_icmp(&icmp).unwrap().icmp_type, 8);
    }
}
