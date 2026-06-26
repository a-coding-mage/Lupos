//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/ioapic.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/ioapic.c
//! KVM-emulated I/O APIC.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/ioapic.c

// The I/O APIC has 24 RTEs (redirection table entries), each a 64-bit
// register split into low/high halves. Field layout:
//   bits[7:0]   vector
//   bits[10:8]  delivery mode (fixed/lowest/smi/nmi/init/extint)
//   bits[11]    dest mode (0=physical, 1=logical)
//   bits[16]    mask
//   bits[55:48] destination (or destination field for logical)
// We model the encoder/decoder.

pub const IOAPIC_RTE_COUNT: usize = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct IoApicRte {
    pub vector: u8,
    pub delivery_mode: u8,
    pub logical_dest: bool,
    pub masked: bool,
    pub destination: u8,
}

pub const fn encode_rte(rte: IoApicRte) -> u64 {
    let mut value = rte.vector as u64;
    value |= ((rte.delivery_mode as u64) & 0x7) << 8;
    if rte.logical_dest {
        value |= 1u64 << 11;
    }
    if rte.masked {
        value |= 1u64 << 16;
    }
    value |= (rte.destination as u64) << 56;
    value
}

pub const fn decode_rte(value: u64) -> IoApicRte {
    IoApicRte {
        vector: (value & 0xff) as u8,
        delivery_mode: ((value >> 8) & 0x7) as u8,
        logical_dest: (value >> 11) & 1 != 0,
        masked: (value >> 16) & 1 != 0,
        destination: ((value >> 56) & 0xff) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rte_round_trips_through_encode_decode() {
        let rte = IoApicRte {
            vector: 0x33,
            delivery_mode: 0x4,
            logical_dest: true,
            masked: true,
            destination: 0xab,
        };
        let value = encode_rte(rte);
        let back = decode_rte(value);
        assert_eq!(back, rte);
    }
}
