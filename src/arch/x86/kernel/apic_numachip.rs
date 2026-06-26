//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! Numachip APIC model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/apic_numachip.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumachipApicId {
    pub node: u16,
    pub local: u16,
}

pub const fn decode_numachip_apic_id(apic_id: u32) -> NumachipApicId {
    NumachipApicId {
        node: ((apic_id >> 16) & 0xffff) as u16,
        local: (apic_id & 0xffff) as u16,
    }
}

pub const fn encode_numachip_apic_id(id: NumachipApicId) -> u32 {
    ((id.node as u32) << 16) | id.local as u32
}

pub const fn numachip_system_detected(oem_id: [u8; 6]) -> bool {
    bytes_eq6(oem_id, *b"NUMASC")
}

const fn bytes_eq6(a: [u8; 6], b: [u8; 6]) -> bool {
    let mut i = 0;
    while i < 6 {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numachip_ids_round_trip_node_and_local_parts() {
        let id = decode_numachip_apic_id(0x0002_0007);
        assert_eq!(id.node, 2);
        assert_eq!(id.local, 7);
        assert_eq!(encode_numachip_apic_id(id), 0x0002_0007);
    }
}
