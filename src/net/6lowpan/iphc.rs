//! linux-parity: partial
//! linux-source: vendor/linux/net/6lowpan/iphc.c
//! test-origin: linux:vendor/linux/net/6lowpan/iphc.c
//! RFC6282 IPHC dispatch, traffic-class, hop-limit, CID, and multicast helpers.

pub const LOWPAN_DISPATCH_IPV6: u8 = 0x41;
pub const LOWPAN_DISPATCH_IPHC: u8 = 0x60;
pub const LOWPAN_DISPATCH_IPHC_MASK: u8 = 0xe0;

pub const LOWPAN_IPHC_TF_MASK: u8 = 0x18;
pub const LOWPAN_IPHC_TF_00: u8 = 0x00;
pub const LOWPAN_IPHC_TF_01: u8 = 0x08;
pub const LOWPAN_IPHC_TF_10: u8 = 0x10;
pub const LOWPAN_IPHC_TF_11: u8 = 0x18;
pub const LOWPAN_IPHC_NH: u8 = 0x04;
pub const LOWPAN_IPHC_HLIM_MASK: u8 = 0x03;
pub const LOWPAN_IPHC_HLIM_00: u8 = 0x00;
pub const LOWPAN_IPHC_HLIM_01: u8 = 0x01;
pub const LOWPAN_IPHC_HLIM_10: u8 = 0x02;
pub const LOWPAN_IPHC_HLIM_11: u8 = 0x03;
pub const LOWPAN_IPHC_CID: u8 = 0x80;
pub const LOWPAN_IPHC_SAC: u8 = 0x40;
pub const LOWPAN_IPHC_SAM_MASK: u8 = 0x30;
pub const LOWPAN_IPHC_SAM_00: u8 = 0x00;
pub const LOWPAN_IPHC_SAM_01: u8 = 0x10;
pub const LOWPAN_IPHC_SAM_10: u8 = 0x20;
pub const LOWPAN_IPHC_SAM_11: u8 = 0x30;
pub const LOWPAN_IPHC_M: u8 = 0x08;
pub const LOWPAN_IPHC_DAC: u8 = 0x04;
pub const LOWPAN_IPHC_DAM_MASK: u8 = 0x03;
pub const LOWPAN_IPHC_DAM_00: u8 = 0x00;
pub const LOWPAN_IPHC_DAM_01: u8 = 0x01;
pub const LOWPAN_IPHC_DAM_10: u8 = 0x02;
pub const LOWPAN_IPHC_DAM_11: u8 = 0x03;

pub const LOWPAN_TTL_VALUES: [u8; 4] = [0, 1, 64, 255];
pub const LOWPAN_IPHC_DAM_TO_SAM_VALUE: [u8; 4] = [
    LOWPAN_IPHC_SAM_00,
    LOWPAN_IPHC_SAM_01,
    LOWPAN_IPHC_SAM_10,
    LOWPAN_IPHC_SAM_11,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrafficFlowEncoding {
    pub mode: u8,
    pub bytes: [u8; 4],
    pub len: usize,
}

pub const fn lowpan_is_ipv6(dispatch: u8) -> bool {
    dispatch == LOWPAN_DISPATCH_IPV6
}

pub const fn lowpan_is_iphc(dispatch: u8) -> bool {
    dispatch & LOWPAN_DISPATCH_IPHC_MASK == LOWPAN_DISPATCH_IPHC
}

pub const fn lowpan_iphc_cid_dci(cid: u8) -> u8 {
    cid & 0x0f
}

pub const fn lowpan_iphc_cid_sci(cid: u8) -> u8 {
    (cid & 0xf0) >> 4
}

pub const fn lowpan_iphc_get_tc(priority: u8, flow_lbl0: u8) -> u8 {
    let dscp = (priority << 2) | ((flow_lbl0 & 0xc0) >> 6);
    let ecn = flow_lbl0 & 0x30;
    (ecn << 2) | dscp
}

pub const fn lowpan_iphc_is_flow_lbl_zero(flow_lbl: [u8; 3]) -> bool {
    (flow_lbl[0] & 0x0f) == 0 && flow_lbl[1] == 0 && flow_lbl[2] == 0
}

pub const fn lowpan_iphc_tf_compress(priority: u8, flow_lbl: [u8; 3]) -> TrafficFlowEncoding {
    let tc = lowpan_iphc_get_tc(priority, flow_lbl[0]);
    if lowpan_iphc_is_flow_lbl_zero(flow_lbl) {
        if tc == 0 {
            TrafficFlowEncoding {
                mode: LOWPAN_IPHC_TF_11,
                bytes: [0; 4],
                len: 0,
            }
        } else {
            TrafficFlowEncoding {
                mode: LOWPAN_IPHC_TF_10,
                bytes: [tc, 0, 0, 0],
                len: 1,
            }
        }
    } else if (tc & 0x3f) == 0 {
        TrafficFlowEncoding {
            mode: LOWPAN_IPHC_TF_01,
            bytes: [
                (flow_lbl[0] & !0xf0) | (tc & 0xc0),
                flow_lbl[1],
                flow_lbl[2],
                0,
            ],
            len: 3,
        }
    } else {
        TrafficFlowEncoding {
            mode: LOWPAN_IPHC_TF_00,
            bytes: [tc, flow_lbl[0] & !0xf0, flow_lbl[1], flow_lbl[2]],
            len: 4,
        }
    }
}

pub const fn hop_limit_encoding(hop_limit: u8) -> (u8, Option<u8>) {
    match hop_limit {
        1 => (LOWPAN_IPHC_HLIM_01, None),
        64 => (LOWPAN_IPHC_HLIM_10, None),
        255 => (LOWPAN_IPHC_HLIM_11, None),
        other => (LOWPAN_IPHC_HLIM_00, Some(other)),
    }
}

pub const fn mcast_addr_compress_mode(addr: [u8; 16]) -> u8 {
    if lowpan_is_mcast_addr_compressable8(addr) {
        LOWPAN_IPHC_DAM_11
    } else if lowpan_is_mcast_addr_compressable32(addr) {
        LOWPAN_IPHC_DAM_10
    } else if lowpan_is_mcast_addr_compressable48(addr) {
        LOWPAN_IPHC_DAM_01
    } else {
        LOWPAN_IPHC_DAM_00
    }
}

pub const fn lowpan_is_mcast_addr_compressable8(addr: [u8; 16]) -> bool {
    addr[1] == 2
        && addr[2] == 0
        && addr[3] == 0
        && addr[4] == 0
        && addr[5] == 0
        && addr[6] == 0
        && addr[7] == 0
        && addr[8] == 0
        && addr[9] == 0
        && addr[10] == 0
        && addr[11] == 0
        && addr[12] == 0
        && addr[13] == 0
        && addr[14] == 0
}

pub const fn lowpan_is_mcast_addr_compressable32(addr: [u8; 16]) -> bool {
    addr[2] == 0
        && addr[3] == 0
        && addr[4] == 0
        && addr[5] == 0
        && addr[6] == 0
        && addr[7] == 0
        && addr[8] == 0
        && addr[9] == 0
        && addr[10] == 0
        && addr[11] == 0
        && addr[12] == 0
}

pub const fn lowpan_is_mcast_addr_compressable48(addr: [u8; 16]) -> bool {
    addr[2] == 0
        && addr[3] == 0
        && addr[4] == 0
        && addr[5] == 0
        && addr[6] == 0
        && addr[7] == 0
        && addr[8] == 0
        && addr[9] == 0
        && addr[10] == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpan_iphc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/iphc.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/net/6lowpan.h"
        ));
        assert!(source.contains("#define LOWPAN_IPHC_TF_MASK\t0x18"));
        assert!(source.contains("#define LOWPAN_IPHC_CID_DCI(cid)\t(cid & 0x0f)"));
        assert!(source.contains("#define LOWPAN_IPHC_CID_SCI(cid)\t((cid & 0xf0) >> 4)"));
        assert!(source.contains("static const u8 lowpan_ttl_values[]"));
        assert!(source.contains("[LOWPAN_IPHC_HLIM_10] = 64"));
        assert!(source.contains("lowpan_iphc_get_tc"));
        assert!(source.contains("return (ecn << 2) | dscp;"));
        assert!(source.contains("lowpan_iphc_tf_compress"));
        assert!(source.contains("lowpan_iphc_mcast_addr_compress"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(lowpan_header_compress);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(lowpan_header_decompress);"));
        assert!(header.contains("#define LOWPAN_DISPATCH_IPV6\t\t0x41"));
        assert!(header.contains("#define LOWPAN_DISPATCH_IPHC\t\t0x60"));
    }

    #[test]
    fn traffic_flow_and_hop_limit_follow_rfc6282_modes() {
        assert!(lowpan_is_ipv6(LOWPAN_DISPATCH_IPV6));
        assert!(lowpan_is_iphc(0x7a));
        assert_eq!(lowpan_iphc_cid_dci(0xab), 0x0b);
        assert_eq!(lowpan_iphc_cid_sci(0xab), 0x0a);
        assert_eq!(LOWPAN_TTL_VALUES[LOWPAN_IPHC_HLIM_10 as usize], 64);

        assert_eq!(
            lowpan_iphc_tf_compress(0, [0, 0, 0]),
            TrafficFlowEncoding {
                mode: LOWPAN_IPHC_TF_11,
                bytes: [0; 4],
                len: 0
            }
        );
        assert_eq!(
            lowpan_iphc_tf_compress(1, [0, 0, 0]).mode,
            LOWPAN_IPHC_TF_10
        );
        assert_eq!(
            lowpan_iphc_tf_compress(0, [0x30, 0x12, 0x34]),
            TrafficFlowEncoding {
                mode: LOWPAN_IPHC_TF_01,
                bytes: [0xc0, 0x12, 0x34, 0],
                len: 3
            }
        );
        assert_eq!(
            lowpan_iphc_tf_compress(1, [0x01, 0x02, 0x03]).mode,
            LOWPAN_IPHC_TF_00
        );
        assert_eq!(hop_limit_encoding(64), (LOWPAN_IPHC_HLIM_10, None));
        assert_eq!(hop_limit_encoding(42), (LOWPAN_IPHC_HLIM_00, Some(42)));
    }

    #[test]
    fn multicast_address_modes_match_linux_predicates() {
        let mut one = [0u8; 16];
        one[0] = 0xff;
        one[1] = 0x02;
        one[15] = 1;
        assert_eq!(mcast_addr_compress_mode(one), LOWPAN_IPHC_DAM_11);

        let mut four = [0u8; 16];
        four[0] = 0xff;
        four[1] = 0x05;
        four[13] = 1;
        assert_eq!(mcast_addr_compress_mode(four), LOWPAN_IPHC_DAM_10);

        let mut six = [0u8; 16];
        six[0] = 0xff;
        six[1] = 0x0e;
        six[11] = 1;
        assert_eq!(mcast_addr_compress_mode(six), LOWPAN_IPHC_DAM_01);
    }
}
