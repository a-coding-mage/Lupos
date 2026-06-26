//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/svm/avic.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/svm/avic.c
//! AMD Advanced Virtual Interrupt Controller (AVIC).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/svm/avic.c

// AVIC accelerates interrupt delivery by letting the SVM hardware write
// directly into the guest's LAPIC backing page. The host registers a
// physical_apic_id_table and a logical_apic_id_table. We model the
// table entry layout.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AvicPhysicalEntry {
    pub host_apic_id: u32,
    pub is_running: bool,
    pub valid: bool,
}

pub const AVIC_PHYSICAL_VALID: u64 = 1u64 << 63;
pub const AVIC_PHYSICAL_RUNNING: u64 = 1u64 << 62;
pub const AVIC_PHYSICAL_HOST_APIC_MASK: u64 = 0xff;

pub const fn encode_physical(entry: AvicPhysicalEntry) -> u64 {
    let mut value = (entry.host_apic_id as u64) & AVIC_PHYSICAL_HOST_APIC_MASK;
    if entry.is_running {
        value |= AVIC_PHYSICAL_RUNNING;
    }
    if entry.valid {
        value |= AVIC_PHYSICAL_VALID;
    }
    value
}

pub const fn decode_physical(value: u64) -> AvicPhysicalEntry {
    AvicPhysicalEntry {
        host_apic_id: (value & AVIC_PHYSICAL_HOST_APIC_MASK) as u32,
        is_running: value & AVIC_PHYSICAL_RUNNING != 0,
        valid: value & AVIC_PHYSICAL_VALID != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_entry_round_trips() {
        let entry = AvicPhysicalEntry {
            host_apic_id: 7,
            is_running: true,
            valid: true,
        };
        let value = encode_physical(entry);
        let back = decode_physical(value);
        assert_eq!(back, entry);
    }
}
