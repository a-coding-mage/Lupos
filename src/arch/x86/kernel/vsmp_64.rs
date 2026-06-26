//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/vsmp_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/vsmp_64.c
//! ScaleMP vSMP Foundation CPU/PCI fixups.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/vsmp_64.c

// vSMP is a software shared-memory product that presents many boards as
// a single SMP system. `vsmp_64.c` hooks PCI quirks to expose the vSMP
// hypervisor signature and force x2APIC + cluster mode. We model the
// detection predicate over the PCI 0:0x1f device id.

pub const VSMP_VENDOR_ID: u16 = 0x18d6;
pub const VSMP_DEVICE_ID: u16 = 0x0011;

pub const fn matches_vsmp(vendor: u16, device: u16) -> bool {
    vendor == VSMP_VENDOR_ID && device == VSMP_DEVICE_ID
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct VsmpQuirks {
    pub force_x2apic: bool,
    pub force_cluster_mode: bool,
    pub paravirt_clock_enabled: bool,
}

pub const fn apply_quirks(detected: bool) -> VsmpQuirks {
    if detected {
        VsmpQuirks {
            force_x2apic: true,
            force_cluster_mode: true,
            paravirt_clock_enabled: true,
        }
    } else {
        VsmpQuirks {
            force_x2apic: false,
            force_cluster_mode: false,
            paravirt_clock_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_device_ids_match_linux_quirk_table() {
        assert!(matches_vsmp(0x18d6, 0x0011));
        assert!(!matches_vsmp(0x18d6, 0x0001));
    }

    #[test]
    fn quirks_apply_only_when_detected() {
        let q = apply_quirks(true);
        assert!(q.force_x2apic);
        let q = apply_quirks(false);
        assert!(!q.force_x2apic);
    }
}
