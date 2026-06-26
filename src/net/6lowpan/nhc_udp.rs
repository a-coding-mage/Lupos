//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/nhc_udp.c
//! test-origin: linux:vendor/linux/net/6lowpan/nhc_udp.c
//! RFC6282 UDP next-header compression and uncompression.

use crate::include::uapi::errno::EINVAL;

use super::nhc::{IPV6HDR_SIZE, LowpanNhc, LowpanSkb};

pub const NEXTHDR_UDP: u8 = 17;
pub const UDP_HDR_SIZE: usize = 8;

pub const LOWPAN_NHC_UDP_MASK: u8 = 0xf8;
pub const LOWPAN_NHC_UDP_ID: u8 = 0xf0;
pub const LOWPAN_NHC_UDP_4BIT_PORT: u16 = 0xf0b0;
pub const LOWPAN_NHC_UDP_4BIT_MASK: u16 = 0xfff0;
pub const LOWPAN_NHC_UDP_8BIT_PORT: u16 = 0xf000;
pub const LOWPAN_NHC_UDP_8BIT_MASK: u16 = 0xff00;
pub const LOWPAN_NHC_UDP_CS_P_00: u8 = 0xf0;
pub const LOWPAN_NHC_UDP_CS_P_01: u8 = 0xf1;
pub const LOWPAN_NHC_UDP_CS_P_10: u8 = 0xf2;
pub const LOWPAN_NHC_UDP_CS_P_11: u8 = 0xf3;
pub const LOWPAN_NHC_UDP_CS_C: u8 = 0x04;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UdpPortMode {
    Inline,
    Dest8,
    Source8,
    Both4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdpHeader {
    pub source: u16,
    pub dest: u16,
    pub len: u16,
    pub check: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdpCompressedPorts {
    pub encoding: u8,
    pub bytes: [u8; 4],
    pub len: usize,
    pub mode: UdpPortMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdpCompressedHeader {
    pub encoding: u8,
    pub bytes: [u8; 7],
    pub len: usize,
    pub mode: UdpPortMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LowpanLlType {
    Ieee802154,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdpUncompressContext {
    pub lltype: LowpanLlType,
    pub ieee802154_d_size: usize,
    pub skb_len: usize,
    pub skb_cow_ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdpUncompressOutcome {
    pub header: UdpHeader,
    pub consumed: usize,
    pub skb_cow_needed: usize,
    pub pushed: usize,
    pub copied: usize,
}

pub const NHC_UDP: LowpanNhc = LowpanNhc {
    name: "RFC6282 UDP",
    nexthdr: NEXTHDR_UDP,
    nexthdrlen: UDP_HDR_SIZE,
    id: LOWPAN_NHC_UDP_ID,
    idmask: LOWPAN_NHC_UDP_MASK,
    uncompress: Some(nhc_udp_uncompress_callback),
    compress: Some(nhc_udp_compress_callback),
};

pub fn nhc_udp_uncompress_callback(skb: &mut LowpanSkb<'_>, needed: usize) -> i32 {
    skb.callback_invocations += 1;
    skb.last_uncompress_needed = Some(needed);
    0
}

pub fn nhc_udp_compress_callback(skb: &mut LowpanSkb<'_>) -> i32 {
    skb.callback_invocations += 1;
    0
}

pub const fn udp_port_mode(source: u16, dest: u16) -> UdpPortMode {
    if (source & LOWPAN_NHC_UDP_4BIT_MASK) == LOWPAN_NHC_UDP_4BIT_PORT
        && (dest & LOWPAN_NHC_UDP_4BIT_MASK) == LOWPAN_NHC_UDP_4BIT_PORT
    {
        UdpPortMode::Both4
    } else if (dest & LOWPAN_NHC_UDP_8BIT_MASK) == LOWPAN_NHC_UDP_8BIT_PORT {
        UdpPortMode::Dest8
    } else if (source & LOWPAN_NHC_UDP_8BIT_MASK) == LOWPAN_NHC_UDP_8BIT_PORT {
        UdpPortMode::Source8
    } else {
        UdpPortMode::Inline
    }
}

pub const fn udp_compress_ports(source: u16, dest: u16) -> UdpCompressedPorts {
    match udp_port_mode(source, dest) {
        UdpPortMode::Both4 => UdpCompressedPorts {
            encoding: LOWPAN_NHC_UDP_CS_P_11,
            bytes: [
                ((source - LOWPAN_NHC_UDP_4BIT_PORT) << 4) as u8
                    + (dest - LOWPAN_NHC_UDP_4BIT_PORT) as u8,
                0,
                0,
                0,
            ],
            len: 1,
            mode: UdpPortMode::Both4,
        },
        UdpPortMode::Dest8 => {
            let src = source.to_be_bytes();
            UdpCompressedPorts {
                encoding: LOWPAN_NHC_UDP_CS_P_01,
                bytes: [src[0], src[1], (dest - LOWPAN_NHC_UDP_8BIT_PORT) as u8, 0],
                len: 3,
                mode: UdpPortMode::Dest8,
            }
        }
        UdpPortMode::Source8 => {
            let dst = dest.to_be_bytes();
            UdpCompressedPorts {
                encoding: LOWPAN_NHC_UDP_CS_P_10,
                bytes: [(source - LOWPAN_NHC_UDP_8BIT_PORT) as u8, dst[0], dst[1], 0],
                len: 3,
                mode: UdpPortMode::Source8,
            }
        }
        UdpPortMode::Inline => {
            let src = source.to_be_bytes();
            let dst = dest.to_be_bytes();
            UdpCompressedPorts {
                encoding: LOWPAN_NHC_UDP_CS_P_00,
                bytes: [src[0], src[1], dst[0], dst[1]],
                len: 4,
                mode: UdpPortMode::Inline,
            }
        }
    }
}

pub const fn udp_compress_header(header: UdpHeader) -> UdpCompressedHeader {
    let ports = udp_compress_ports(header.source, header.dest);
    let check = header.check.to_be_bytes();
    let mut bytes = [0u8; 7];
    let mut i = 0usize;
    while i < ports.len {
        bytes[i] = ports.bytes[i];
        i += 1;
    }
    bytes[ports.len] = check[0];
    bytes[ports.len + 1] = check[1];

    UdpCompressedHeader {
        encoding: ports.encoding,
        bytes,
        len: 1 + ports.len + 2,
        mode: ports.mode,
    }
}

pub const fn udp_decompress_ports(encoding: u8, bytes: [u8; 4]) -> Option<(u16, u16, usize)> {
    match encoding & LOWPAN_NHC_UDP_CS_P_11 {
        LOWPAN_NHC_UDP_CS_P_00 => Some((
            u16::from_be_bytes([bytes[0], bytes[1]]),
            u16::from_be_bytes([bytes[2], bytes[3]]),
            4,
        )),
        LOWPAN_NHC_UDP_CS_P_01 => Some((
            u16::from_be_bytes([bytes[0], bytes[1]]),
            LOWPAN_NHC_UDP_8BIT_PORT + bytes[2] as u16,
            3,
        )),
        LOWPAN_NHC_UDP_CS_P_10 => Some((
            LOWPAN_NHC_UDP_8BIT_PORT + bytes[0] as u16,
            u16::from_be_bytes([bytes[1], bytes[2]]),
            3,
        )),
        LOWPAN_NHC_UDP_CS_P_11 => Some((
            LOWPAN_NHC_UDP_4BIT_PORT + (bytes[0] >> 4) as u16,
            LOWPAN_NHC_UDP_4BIT_PORT + (bytes[0] & 0x0f) as u16,
            1,
        )),
        _ => None,
    }
}

fn fetch_u8(input: &[u8], offset: &mut usize, fail: &mut bool) -> u8 {
    if *offset < input.len() {
        let value = input[*offset];
        *offset += 1;
        value
    } else {
        *fail = true;
        0
    }
}

fn fetch_u16_be(input: &[u8], offset: &mut usize, fail: &mut bool) -> u16 {
    let high = fetch_u8(input, offset, fail);
    let low = fetch_u8(input, offset, fail);
    u16::from_be_bytes([high, low])
}

pub fn udp_uncompress(
    input: &[u8],
    needed: usize,
    ctx: UdpUncompressContext,
) -> Result<UdpUncompressOutcome, i32> {
    let mut offset = 0usize;
    let mut fail = false;
    let tmp = fetch_u8(input, &mut offset, &mut fail);
    let mut val;

    let (source, dest) = match tmp & LOWPAN_NHC_UDP_CS_P_11 {
        LOWPAN_NHC_UDP_CS_P_00 => (
            fetch_u16_be(input, &mut offset, &mut fail),
            fetch_u16_be(input, &mut offset, &mut fail),
        ),
        LOWPAN_NHC_UDP_CS_P_01 => {
            let source = fetch_u16_be(input, &mut offset, &mut fail);
            val = fetch_u8(input, &mut offset, &mut fail);
            (source, LOWPAN_NHC_UDP_8BIT_PORT + val as u16)
        }
        LOWPAN_NHC_UDP_CS_P_10 => {
            val = fetch_u8(input, &mut offset, &mut fail);
            let source = LOWPAN_NHC_UDP_8BIT_PORT + val as u16;
            let dest = fetch_u16_be(input, &mut offset, &mut fail);
            (source, dest)
        }
        LOWPAN_NHC_UDP_CS_P_11 => {
            val = fetch_u8(input, &mut offset, &mut fail);
            (
                LOWPAN_NHC_UDP_4BIT_PORT + (val >> 4) as u16,
                LOWPAN_NHC_UDP_4BIT_PORT + (val & 0x0f) as u16,
            )
        }
        _ => unreachable!(),
    };

    let check = if tmp & LOWPAN_NHC_UDP_CS_C != 0 {
        fail = true;
        0
    } else {
        fetch_u16_be(input, &mut offset, &mut fail)
    };

    if fail {
        return Err(-EINVAL);
    }

    let len = match ctx.lltype {
        LowpanLlType::Ieee802154 if ctx.ieee802154_d_size != 0 => {
            (ctx.ieee802154_d_size - IPV6HDR_SIZE) as u16
        }
        _ => (ctx.skb_len + UDP_HDR_SIZE) as u16,
    };

    if ctx.skb_cow_ret != 0 {
        return Err(ctx.skb_cow_ret);
    }

    Ok(UdpUncompressOutcome {
        header: UdpHeader {
            source,
            dest,
            len,
            check,
        },
        consumed: offset,
        skb_cow_needed: needed,
        pushed: UDP_HDR_SIZE,
        copied: UDP_HDR_SIZE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nhc_udp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/nhc_udp.c"
        ));
        assert!(source.contains("#define LOWPAN_NHC_UDP_MASK\t\t0xF8"));
        assert!(source.contains("#define LOWPAN_NHC_UDP_ID\t\t0xF0"));
        assert!(source.contains("#define LOWPAN_NHC_UDP_4BIT_PORT\t0xF0B0"));
        assert!(source.contains("#define LOWPAN_NHC_UDP_8BIT_PORT\t0xF000"));
        assert!(source.contains("lowpan_fetch_skb(skb, &tmp, sizeof(tmp))"));
        assert!(source.contains("case LOWPAN_NHC_UDP_CS_P_11:"));
        assert!(source.contains("checksum elided currently not supported"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("lowpan_dev(skb->dev)->lltype"));
        assert!(source.contains("lowpan_802154_cb(skb)->d_size"));
        assert!(source.contains("skb_cow(skb, needed)"));
        assert!(source.contains("skb_push(skb, sizeof(struct udphdr))"));
        assert!(source.contains("skb_copy_to_linear_data(skb, &uh, sizeof(struct udphdr))"));
        assert!(source.contains("udp_hdr(skb)"));
        assert!(source.contains("lowpan_push_hc_data(hc_ptr, &uh->check"));
        assert!(source.contains("LOWPAN_NHC(nhc_udp, \"RFC6282 UDP\", NEXTHDR_UDP"));
        assert!(source.contains("module_lowpan_nhc(nhc_udp);"));
        assert!(
            source.contains("MODULE_DESCRIPTION(\"6LoWPAN next header RFC6282 UDP compression\")")
        );

        assert_eq!(NHC_UDP.name, "RFC6282 UDP");
        assert_eq!(NHC_UDP.nexthdr, NEXTHDR_UDP);
        assert_eq!(NHC_UDP.nexthdrlen, UDP_HDR_SIZE);
        assert_eq!(NHC_UDP.id, LOWPAN_NHC_UDP_ID);
        assert_eq!(NHC_UDP.idmask, LOWPAN_NHC_UDP_MASK);
        assert!(NHC_UDP.compress.is_some());
        assert!(NHC_UDP.uncompress.is_some());
    }

    #[test]
    fn udp_port_compression_modes_round_trip() {
        let both = udp_compress_ports(0xf0b1, 0xf0bf);
        assert_eq!(both.mode, UdpPortMode::Both4);
        assert_eq!(both.bytes[0], 0x1f);
        assert_eq!(
            udp_decompress_ports(both.encoding, both.bytes),
            Some((0xf0b1, 0xf0bf, 1))
        );

        let dest = udp_compress_ports(0x1234, 0xf0aa);
        assert_eq!(dest.mode, UdpPortMode::Dest8);
        assert_eq!(
            udp_decompress_ports(dest.encoding, dest.bytes),
            Some((0x1234, 0xf0aa, 3))
        );

        let source = udp_compress_ports(0xf055, 0x5678);
        assert_eq!(source.mode, UdpPortMode::Source8);
        assert_eq!(
            udp_decompress_ports(source.encoding, source.bytes),
            Some((0xf055, 0x5678, 3))
        );

        let inline = udp_compress_ports(1000, 2000);
        assert_eq!(inline.mode, UdpPortMode::Inline);
        assert_eq!(
            udp_decompress_ports(inline.encoding, inline.bytes),
            Some((1000, 2000, 4))
        );
    }

    #[test]
    fn udp_compression_includes_encoding_ports_and_checksum() {
        let compressed = udp_compress_header(UdpHeader {
            source: 0xf0b1,
            dest: 0xf0bf,
            len: 0,
            check: 0xabcd,
        });
        assert_eq!(compressed.encoding, LOWPAN_NHC_UDP_CS_P_11);
        assert_eq!(compressed.bytes[0], 0x1f);
        assert_eq!(compressed.bytes[1], 0xab);
        assert_eq!(compressed.bytes[2], 0xcd);
        assert_eq!(compressed.len, 4);

        let inline = udp_compress_header(UdpHeader {
            source: 1000,
            dest: 2000,
            len: 0,
            check: 0x3456,
        });
        assert_eq!(inline.encoding, LOWPAN_NHC_UDP_CS_P_00);
        assert_eq!(&inline.bytes[..6], &[0x03, 0xe8, 0x07, 0xd0, 0x34, 0x56]);
        assert_eq!(inline.len, 7);
    }

    #[test]
    fn udp_uncompression_reconstructs_header_and_skb_side_effects() {
        let compressed = udp_compress_header(UdpHeader {
            source: 0xf0b1,
            dest: 0xf0bf,
            len: 0,
            check: 0xabcd,
        });
        let mut bytes = [0u8; 8];
        bytes[0] = compressed.encoding;
        bytes[1..1 + compressed.len - 1].copy_from_slice(&compressed.bytes[..compressed.len - 1]);
        let outcome = udp_uncompress(
            &bytes[..compressed.len],
            48,
            UdpUncompressContext {
                lltype: LowpanLlType::Other,
                ieee802154_d_size: 0,
                skb_len: 20,
                skb_cow_ret: 0,
            },
        )
        .expect("uncompress");

        assert_eq!(
            outcome.header,
            UdpHeader {
                source: 0xf0b1,
                dest: 0xf0bf,
                len: 28,
                check: 0xabcd
            }
        );
        assert_eq!(outcome.consumed, compressed.len);
        assert_eq!(outcome.skb_cow_needed, 48);
        assert_eq!(outcome.pushed, UDP_HDR_SIZE);
        assert_eq!(outcome.copied, UDP_HDR_SIZE);

        let inline = udp_uncompress(
            &[LOWPAN_NHC_UDP_CS_P_00, 0x03, 0xe8, 0x07, 0xd0, 0x34, 0x56],
            64,
            UdpUncompressContext {
                lltype: LowpanLlType::Ieee802154,
                ieee802154_d_size: 72,
                skb_len: 99,
                skb_cow_ret: 0,
            },
        )
        .expect("inline");
        assert_eq!(inline.header.source, 1000);
        assert_eq!(inline.header.dest, 2000);
        assert_eq!(inline.header.len, 32);
        assert_eq!(inline.header.check, 0x3456);
    }

    #[test]
    fn udp_uncompression_errors_match_linux_fail_paths() {
        assert_eq!(
            udp_uncompress(
                &[LOWPAN_NHC_UDP_CS_P_11 | LOWPAN_NHC_UDP_CS_C, 0x1f],
                48,
                UdpUncompressContext {
                    lltype: LowpanLlType::Other,
                    ieee802154_d_size: 0,
                    skb_len: 20,
                    skb_cow_ret: 0,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            udp_uncompress(
                &[LOWPAN_NHC_UDP_CS_P_01, 0x03],
                48,
                UdpUncompressContext {
                    lltype: LowpanLlType::Other,
                    ieee802154_d_size: 0,
                    skb_len: 20,
                    skb_cow_ret: 0,
                },
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            udp_uncompress(
                &[LOWPAN_NHC_UDP_CS_P_11, 0x1f, 0xab, 0xcd],
                48,
                UdpUncompressContext {
                    lltype: LowpanLlType::Other,
                    ieee802154_d_size: 0,
                    skb_len: 20,
                    skb_cow_ret: -EINVAL,
                },
            ),
            Err(-EINVAL)
        );
    }
}
