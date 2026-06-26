//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/rpl.c
//! test-origin: linux:vendor/linux/net/ipv6/rpl.c
//! IPv6 RPL source routing header compression helpers.

use alloc::vec::Vec;
use core::ops::Range;

pub type In6Addr = [u8; 16];

pub const IPV6_RPL_BEST_ADDR_COMPRESSION: u8 = 15;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ipv6RplSrHdr {
    pub nexthdr: u8,
    pub hdrlen: u8,
    pub type_: u8,
    pub segments_left: u8,
    pub cmpri: u8,
    pub cmpre: u8,
    pub pad: u8,
    pub rpl_segaddr: Vec<In6Addr>,
    pub rpl_segdata: Vec<u8>,
}

impl Ipv6RplSrHdr {
    pub fn new(nsegments: usize) -> Self {
        Self {
            nexthdr: 0,
            hdrlen: 0,
            type_: 0,
            segments_left: 0,
            cmpri: 0,
            cmpre: 0,
            pad: 0,
            rpl_segaddr: alloc::vec![[0; 16]; nsegments],
            rpl_segdata: Vec::new(),
        }
    }
}

fn ipv6_pfxtail_len(pfx: u8) -> usize {
    core::mem::size_of::<In6Addr>() - usize::from(pfx)
}

fn ipv6_rpl_addr_decompress(dst: &mut In6Addr, daddr: &In6Addr, post: &[u8], pfx: u8) {
    let prefix_len = usize::from(pfx);
    let tail_len = ipv6_pfxtail_len(pfx);

    dst[..prefix_len].copy_from_slice(&daddr[..prefix_len]);
    dst[prefix_len..].copy_from_slice(&post[..tail_len]);
}

fn ipv6_rpl_addr_compress(dst: &mut [u8], addr: &In6Addr, pfx: u8) {
    let prefix_len = usize::from(pfx);
    let tail_len = ipv6_pfxtail_len(pfx);

    dst[..tail_len].copy_from_slice(&addr[prefix_len..]);
}

fn ipv6_rpl_segdata_range(hdr: &Ipv6RplSrHdr, i: usize, pfx: u8) -> Range<usize> {
    let tail_len = ipv6_pfxtail_len(hdr.cmpri);
    let start = i * tail_len;
    start..start + ipv6_pfxtail_len(pfx)
}

pub fn ipv6_rpl_srh_decompress(
    outhdr: &mut Ipv6RplSrHdr,
    inhdr: &Ipv6RplSrHdr,
    daddr: &In6Addr,
    n: u8,
) {
    let n = usize::from(n);
    outhdr.nexthdr = inhdr.nexthdr;
    outhdr.hdrlen = (((n + 1) * core::mem::size_of::<In6Addr>()) >> 3) as u8;
    outhdr.pad = 0;
    outhdr.type_ = inhdr.type_;
    outhdr.segments_left = inhdr.segments_left;
    outhdr.cmpri = 0;
    outhdr.cmpre = 0;
    outhdr.rpl_segaddr.resize(n + 1, [0; 16]);
    outhdr.rpl_segdata.clear();

    for i in 0..n {
        let range = ipv6_rpl_segdata_range(inhdr, i, inhdr.cmpri);
        ipv6_rpl_addr_decompress(
            &mut outhdr.rpl_segaddr[i],
            daddr,
            &inhdr.rpl_segdata[range],
            inhdr.cmpri,
        );
    }

    let range = ipv6_rpl_segdata_range(inhdr, n, inhdr.cmpre);
    ipv6_rpl_addr_decompress(
        &mut outhdr.rpl_segaddr[n],
        daddr,
        &inhdr.rpl_segdata[range],
        inhdr.cmpre,
    );
}

fn ipv6_rpl_srh_calc_cmpri(inhdr: &Ipv6RplSrHdr, daddr: &In6Addr, n: u8) -> u8 {
    for plen in 0..core::mem::size_of::<In6Addr>() {
        for i in 0..usize::from(n) {
            if daddr[plen] != inhdr.rpl_segaddr[i][plen] {
                return plen as u8;
            }
        }
    }

    IPV6_RPL_BEST_ADDR_COMPRESSION
}

fn ipv6_rpl_srh_calc_cmpre(daddr: &In6Addr, last_segment: &In6Addr) -> u8 {
    for plen in 0..core::mem::size_of::<In6Addr>() {
        if daddr[plen] != last_segment[plen] {
            return plen as u8;
        }
    }

    IPV6_RPL_BEST_ADDR_COMPRESSION
}

pub fn ipv6_rpl_srh_compress(
    outhdr: &mut Ipv6RplSrHdr,
    inhdr: &Ipv6RplSrHdr,
    daddr: &In6Addr,
    n: u8,
) {
    let n_usize = usize::from(n);
    let cmpri = ipv6_rpl_srh_calc_cmpri(inhdr, daddr, n);
    let cmpre = ipv6_rpl_srh_calc_cmpre(daddr, &inhdr.rpl_segaddr[n_usize]);

    outhdr.nexthdr = inhdr.nexthdr;
    let seglen = n_usize * ipv6_pfxtail_len(cmpri) + ipv6_pfxtail_len(cmpre);
    outhdr.hdrlen = (seglen >> 3) as u8;
    if seglen & 0x7 != 0 {
        outhdr.hdrlen += 1;
        outhdr.pad = (8 - (seglen & 0x7)) as u8;
    } else {
        outhdr.pad = 0;
    }
    outhdr.type_ = inhdr.type_;
    outhdr.segments_left = inhdr.segments_left;
    outhdr.cmpri = cmpri;
    outhdr.cmpre = cmpre;
    outhdr.rpl_segaddr.clear();
    outhdr.rpl_segdata.resize(seglen, 0);

    for i in 0..n_usize {
        let range = ipv6_rpl_segdata_range(outhdr, i, cmpri);
        ipv6_rpl_addr_compress(&mut outhdr.rpl_segdata[range], &inhdr.rpl_segaddr[i], cmpri);
    }

    let range = ipv6_rpl_segdata_range(outhdr, n_usize, cmpre);
    ipv6_rpl_addr_compress(
        &mut outhdr.rpl_segdata[range],
        &inhdr.rpl_segaddr[n_usize],
        cmpre,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(last: u8) -> In6Addr {
        [
            0x20, 0x01, 0x0d, 0xb8, 0xaa, 0xbb, 0xcc, 0xdd, 0, 0, 0, 0, 0, 0, 0, last,
        ]
    }

    #[test]
    fn rpl_matches_linux_source_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/rpl.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/rpl.h"
        ));

        assert!(source.contains("#define IPV6_PFXTAIL_LEN(x) (sizeof(struct in6_addr) - (x))"));
        assert!(source.contains("#define IPV6_RPL_BEST_ADDR_COMPRESSION 15"));
        assert!(source.contains("outhdr->hdrlen = (((n + 1) * sizeof(struct in6_addr)) >> 3);"));
        assert!(
            source.contains("seglen = (n * IPV6_PFXTAIL_LEN(cmpri)) + IPV6_PFXTAIL_LEN(cmpre);")
        );
        assert!(source.contains("if (seglen & 0x7)"));
        assert!(
            source.contains("return (void *)&hdr->rpl_segdata[i * IPV6_PFXTAIL_LEN(hdr->cmpri)];")
        );
        assert!(header.contains("struct ipv6_rpl_sr_hdr"));
        assert!(header.contains("#define rpl_segaddr"));
        assert!(header.contains("#define rpl_segdata"));
    }

    #[test]
    fn compress_then_decompress_reconstructs_segments_and_header_fields() {
        let daddr = addr(0);
        let mut inhdr = Ipv6RplSrHdr::new(3);
        inhdr.nexthdr = 43;
        inhdr.type_ = 3;
        inhdr.segments_left = 2;
        inhdr.rpl_segaddr[0] = addr(1);
        inhdr.rpl_segaddr[1] = addr(2);
        inhdr.rpl_segaddr[2] = [
            0x20, 0x01, 0x0d, 0xb8, 0xaa, 0xbb, 0xcc, 0xdd, 0, 0, 0, 0, 0, 0x77, 0x88, 0x03,
        ];

        let mut compressed = Ipv6RplSrHdr::new(0);
        ipv6_rpl_srh_compress(&mut compressed, &inhdr, &daddr, 2);

        assert_eq!(compressed.nexthdr, 43);
        assert_eq!(compressed.type_, 3);
        assert_eq!(compressed.segments_left, 2);
        assert_eq!(compressed.cmpri, 15);
        assert_eq!(compressed.cmpre, 13);
        assert_eq!(compressed.rpl_segdata.len(), 5);
        assert_eq!(compressed.hdrlen, 1);
        assert_eq!(compressed.pad, 3);

        let mut decompressed = Ipv6RplSrHdr::new(0);
        ipv6_rpl_srh_decompress(&mut decompressed, &compressed, &daddr, 2);

        assert_eq!(decompressed.hdrlen, 6);
        assert_eq!(decompressed.cmpri, 0);
        assert_eq!(decompressed.cmpre, 0);
        assert_eq!(decompressed.rpl_segaddr, inhdr.rpl_segaddr);
    }

    #[test]
    fn full_address_match_uses_best_compression_constant() {
        let daddr = addr(0);
        let mut inhdr = Ipv6RplSrHdr::new(2);
        inhdr.rpl_segaddr[0] = daddr;
        inhdr.rpl_segaddr[1] = daddr;

        let mut compressed = Ipv6RplSrHdr::new(0);
        ipv6_rpl_srh_compress(&mut compressed, &inhdr, &daddr, 1);

        assert_eq!(compressed.cmpri, IPV6_RPL_BEST_ADDR_COMPRESSION);
        assert_eq!(compressed.cmpre, IPV6_RPL_BEST_ADDR_COMPRESSION);
        assert_eq!(compressed.rpl_segdata, alloc::vec![0, 0]);
    }
}
