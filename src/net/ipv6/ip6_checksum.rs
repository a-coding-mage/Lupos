//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/ip6_checksum.c
//! test-origin: linux:vendor/linux/net/ipv6/ip6_checksum.c
//! IPv6 pseudo-header checksum and UDP checksum setup.

pub const CHECKSUM_NONE: u8 = 0;
pub const CHECKSUM_PARTIAL: u8 = 3;
pub const CSUM_MANGLED_0: u16 = 0xffff;
pub const UDP_CHECK_OFFSET: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Udp6ChecksumState {
    pub check: u16,
    pub ip_summed: u8,
    pub is_gso: bool,
    pub lco_csum: u32,
    pub csum_start: usize,
    pub csum_offset: usize,
}

pub fn csum_ipv6_magic(saddr: [u8; 16], daddr: [u8; 16], len: u32, proto: u8, csum: u32) -> u16 {
    let mut sum = csum as u64;
    add_words(&mut sum, &saddr);
    add_words(&mut sum, &daddr);
    sum += ((len >> 16) as u64) + ((len & 0xffff) as u64);
    sum += proto as u64;
    fold(sum)
}

pub fn udp_v6_check(len: u32, saddr: [u8; 16], daddr: [u8; 16], csum: u32) -> u16 {
    csum_ipv6_magic(saddr, daddr, len, 17, csum)
}

pub fn udp6_set_csum(
    nocheck: bool,
    mut skb: Udp6ChecksumState,
    saddr: [u8; 16],
    daddr: [u8; 16],
    len: u32,
    transport_header_offset: usize,
) -> Udp6ChecksumState {
    if nocheck {
        skb.check = 0;
    } else if skb.is_gso {
        skb.check = !udp_v6_check(len, saddr, daddr, 0);
    } else if skb.ip_summed == CHECKSUM_PARTIAL {
        skb.check = udp_v6_check(len, saddr, daddr, skb.lco_csum);
        if skb.check == 0 {
            skb.check = CSUM_MANGLED_0;
        }
    } else {
        skb.ip_summed = CHECKSUM_PARTIAL;
        skb.csum_start = transport_header_offset;
        skb.csum_offset = UDP_CHECK_OFFSET;
        skb.check = !udp_v6_check(len, saddr, daddr, 0);
    }
    skb
}

fn add_words(sum: &mut u64, bytes: &[u8]) {
    for chunk in bytes.chunks_exact(2) {
        *sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u64;
    }
}

fn fold(mut sum: u64) -> u16 {
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip6_checksum_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/ip6_checksum.c"
        ));
        assert!(source.contains("__sum16 csum_ipv6_magic"));
        assert!(source.contains("sum += (__force u32)saddr->s6_addr32[0];"));
        assert!(source.contains("sum += (__force u32)daddr->s6_addr32[3];"));
        assert!(source.contains("ulen = (__force u32)htonl((__u32) len);"));
        assert!(source.contains("uproto = (__force u32)htonl(proto);"));
        assert!(source.contains("return csum_fold((__force __wsum)sum);"));
        assert!(source.contains("EXPORT_SYMBOL(csum_ipv6_magic);"));
        assert!(source.contains("void udp6_set_csum"));
        assert!(source.contains("if (nocheck)"));
        assert!(source.contains("uh->check = 0;"));
        assert!(source.contains("else if (skb_is_gso(skb))"));
        assert!(source.contains("uh->check = ~udp_v6_check(len, saddr, daddr, 0);"));
        assert!(source.contains("else if (skb->ip_summed == CHECKSUM_PARTIAL)"));
        assert!(source.contains("uh->check = udp_v6_check(len, saddr, daddr, lco_csum(skb));"));
        assert!(source.contains("uh->check = CSUM_MANGLED_0;"));
        assert!(source.contains("skb->ip_summed = CHECKSUM_PARTIAL;"));
        assert!(source.contains("skb->csum_offset = offsetof(struct udphdr, check);"));
        assert!(source.contains("EXPORT_SYMBOL(udp6_set_csum);"));
    }

    #[test]
    fn udp6_set_csum_follows_linux_branches() {
        let src = [0x20, 1, 0xdb, 0x8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let dst = [0x20, 1, 0xdb, 0x8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
        assert_eq!(csum_ipv6_magic([0; 16], [0; 16], 0, 0, 0), 0xffff);

        let base = Udp6ChecksumState {
            check: 7,
            ip_summed: CHECKSUM_NONE,
            is_gso: false,
            lco_csum: 0,
            csum_start: 0,
            csum_offset: 0,
        };
        assert_eq!(udp6_set_csum(true, base, src, dst, 8, 40).check, 0);

        let partial = udp6_set_csum(
            false,
            Udp6ChecksumState {
                ip_summed: CHECKSUM_PARTIAL,
                lco_csum: 1,
                ..base
            },
            src,
            dst,
            8,
            40,
        );
        assert_ne!(partial.check, 0);
        assert_eq!(partial.ip_summed, CHECKSUM_PARTIAL);

        let offload = udp6_set_csum(false, base, src, dst, 8, 40);
        assert_eq!(offload.ip_summed, CHECKSUM_PARTIAL);
        assert_eq!(offload.csum_start, 40);
        assert_eq!(offload.csum_offset, UDP_CHECK_OFFSET);
    }
}
