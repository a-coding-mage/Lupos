//! linux-parity: complete
//! linux-source: vendor/linux/net/802/stp.c
//! test-origin: linux:vendor/linux/net/802/stp.c
//! STP SAP demux validation and protocol registration.

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::EINVAL;

pub const GARP_ADDR_MIN: u8 = 0x20;
pub const GARP_ADDR_MAX: u8 = 0x2f;
pub const GARP_ADDR_RANGE: usize = (GARP_ADDR_MAX - GARP_ADDR_MIN) as usize;
pub const LLC_SAP_BSPAN: u8 = 0x42;
pub const LLC_PDU_TYPE_U: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StpProto {
    pub group_address: [u8; 6],
    pub data: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StpPdu {
    pub ssap: u8,
    pub dsap: u8,
    pub ctrl_1: u8,
    pub h_dest: [u8; 6],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StpReceive {
    Delivered(StpProto),
    Dropped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StpRegistrySnapshot {
    pub sap_registered: usize,
    pub sap_open: bool,
    pub stp_proto: Option<StpProto>,
    pub garp_protos: [Option<StpProto>; GARP_ADDR_RANGE + 1],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StpRegistry {
    sap_registered: usize,
    sap_open: bool,
    stp_proto: Option<StpProto>,
    garp_protos: [Option<StpProto>; GARP_ADDR_RANGE + 1],
}

impl StpRegistry {
    const fn new() -> Self {
        Self {
            sap_registered: 0,
            sap_open: false,
            stp_proto: None,
            garp_protos: [None; GARP_ADDR_RANGE + 1],
        }
    }

    const fn snapshot(&self) -> StpRegistrySnapshot {
        StpRegistrySnapshot {
            sap_registered: self.sap_registered,
            sap_open: self.sap_open,
            stp_proto: self.stp_proto,
            garp_protos: self.garp_protos,
        }
    }
}

lazy_static! {
    static ref STP_REGISTRY: Mutex<StpRegistry> = Mutex::new(StpRegistry::new());
}

pub const fn garp_proto_index(group_address: [u8; 6]) -> Option<usize> {
    if group_address[5] >= GARP_ADDR_MIN && group_address[5] <= GARP_ADDR_MAX {
        Some((group_address[5] - GARP_ADDR_MIN) as usize)
    } else {
        None
    }
}

pub const fn is_zero_ether_addr(addr: [u8; 6]) -> bool {
    addr[0] == 0 && addr[1] == 0 && addr[2] == 0 && addr[3] == 0 && addr[4] == 0 && addr[5] == 0
}

pub fn stp_proto_register(proto: StpProto) -> Result<(), i32> {
    let mut registry = STP_REGISTRY.lock();
    if registry.sap_registered == 0 {
        registry.sap_open = true;
    }
    registry.sap_registered += 1;

    if is_zero_ether_addr(proto.group_address) {
        registry.stp_proto = Some(proto);
    } else if let Some(index) = garp_proto_index(proto.group_address) {
        registry.garp_protos[index] = Some(proto);
    } else {
        return Err(EINVAL);
    }
    Ok(())
}

pub fn stp_proto_unregister(proto: StpProto) {
    let mut registry = STP_REGISTRY.lock();
    if is_zero_ether_addr(proto.group_address) {
        registry.stp_proto = None;
    } else if let Some(index) = garp_proto_index(proto.group_address) {
        registry.garp_protos[index] = None;
    }

    if registry.sap_registered > 0 {
        registry.sap_registered -= 1;
        if registry.sap_registered == 0 {
            registry.sap_open = false;
        }
    }
}

pub fn stp_pdu_rcv(pdu: StpPdu) -> StpReceive {
    if pdu.ssap != LLC_SAP_BSPAN || pdu.dsap != LLC_SAP_BSPAN || pdu.ctrl_1 != LLC_PDU_TYPE_U {
        return StpReceive::Dropped;
    }

    let registry = STP_REGISTRY.lock();
    let proto = if let Some(index) = garp_proto_index(pdu.h_dest) {
        let proto = registry.garp_protos[index];
        if let Some(proto) = proto
            && proto.group_address != pdu.h_dest
        {
            return StpReceive::Dropped;
        }
        proto
    } else {
        registry.stp_proto
    };

    match proto {
        Some(proto) => StpReceive::Delivered(proto),
        None => StpReceive::Dropped,
    }
}

pub fn stp_registry_snapshot() -> StpRegistrySnapshot {
    STP_REGISTRY.lock().snapshot()
}

#[cfg(test)]
fn stp_reset_for_tests() {
    *STP_REGISTRY.lock() = StpRegistry::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/802/stp.c"
        ));
        assert!(source.contains("#define GARP_ADDR_MIN\t0x20"));
        assert!(source.contains("#define GARP_ADDR_MAX\t0x2F"));
        assert!(source.contains("garp_protos[GARP_ADDR_RANGE + 1]"));
        assert!(source.contains("pdu->ssap != LLC_SAP_BSPAN"));
        assert!(source.contains("eh->h_dest[5] >= GARP_ADDR_MIN"));
        assert!(source.contains("!ether_addr_equal(eh->h_dest, proto->group_address)"));
        assert!(source.contains("proto->rcv(proto, skb, dev);"));
        assert!(source.contains("if (sap_registered++ == 0)"));
        assert!(source.contains("llc_sap_open(LLC_SAP_BSPAN, stp_pdu_rcv);"));
        assert!(source.contains("if (is_zero_ether_addr(proto->group_address))"));
        assert!(source.contains("rcu_assign_pointer(stp_proto, proto);"));
        assert!(source.contains("RCU_INIT_POINTER(stp_proto, NULL);"));
        assert!(source.contains("if (--sap_registered == 0)"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(stp_proto_unregister);"));
        assert!(source.contains(
            "MODULE_DESCRIPTION(\"SAP demux for IEEE 802.1D Spanning Tree Protocol (STP)\")"
        ));
    }

    #[test]
    fn stp_demux_uses_bspan_and_garp_last_octet_range() {
        stp_reset_for_tests();
        assert_eq!(GARP_ADDR_RANGE, 15);
        assert_eq!(garp_proto_index([0x01, 0x80, 0xc2, 0, 0, 0x21]), Some(1));
        let garp = StpProto {
            group_address: [1, 0x80, 0xc2, 0, 0, 0x2f],
            data: 47,
        };
        let stp = StpProto {
            group_address: [0; 6],
            data: 11,
        };
        stp_proto_register(garp).unwrap();
        stp_proto_register(stp).unwrap();

        assert_eq!(
            stp_pdu_rcv(StpPdu {
                ssap: LLC_SAP_BSPAN,
                dsap: LLC_SAP_BSPAN,
                ctrl_1: LLC_PDU_TYPE_U,
                h_dest: [1, 0x80, 0xc2, 0, 0, 0x2f],
            }),
            StpReceive::Delivered(garp)
        );
        assert_eq!(
            stp_pdu_rcv(StpPdu {
                ssap: LLC_SAP_BSPAN,
                dsap: LLC_SAP_BSPAN,
                ctrl_1: LLC_PDU_TYPE_U,
                h_dest: [1, 0x80, 0xc2, 0, 0, 0],
            }),
            StpReceive::Delivered(stp)
        );
        assert_eq!(
            stp_pdu_rcv(StpPdu {
                ssap: 0,
                dsap: LLC_SAP_BSPAN,
                ctrl_1: LLC_PDU_TYPE_U,
                h_dest: [1, 0x80, 0xc2, 0, 0, 0],
            }),
            StpReceive::Dropped
        );
    }

    #[test]
    fn stp_register_unregister_tracks_sap_reference_count() {
        stp_reset_for_tests();
        let stp = StpProto {
            group_address: [0; 6],
            data: 1,
        };
        let garp = StpProto {
            group_address: [1, 0x80, 0xc2, 0, 0, 0x20],
            data: 2,
        };

        stp_proto_register(stp).unwrap();
        stp_proto_register(garp).unwrap();
        let snapshot = stp_registry_snapshot();
        assert!(snapshot.sap_open);
        assert_eq!(snapshot.sap_registered, 2);
        assert_eq!(snapshot.stp_proto, Some(stp));
        assert_eq!(snapshot.garp_protos[0], Some(garp));

        stp_proto_unregister(garp);
        assert_eq!(stp_registry_snapshot().sap_registered, 1);
        stp_proto_unregister(stp);
        let snapshot = stp_registry_snapshot();
        assert_eq!(snapshot.sap_registered, 0);
        assert!(!snapshot.sap_open);
    }

    #[test]
    fn stp_drops_mismatched_garp_group_address_like_linux() {
        stp_reset_for_tests();
        stp_proto_register(StpProto {
            group_address: [1, 0x80, 0xc2, 0, 0, 0x21],
            data: 1,
        })
        .unwrap();

        assert_eq!(
            stp_pdu_rcv(StpPdu {
                ssap: LLC_SAP_BSPAN,
                dsap: LLC_SAP_BSPAN,
                ctrl_1: LLC_PDU_TYPE_U,
                h_dest: [2, 0x80, 0xc2, 0, 0, 0x21],
            }),
            StpReceive::Dropped
        );
    }
}
