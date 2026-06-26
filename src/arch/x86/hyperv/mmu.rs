//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/hyperv/mmu.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/mmu.c
//! Hyper-V MMU and TLB hypercall model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/mmu.c

pub const HV_FLUSH_ALL_PROCESSORS: u64 = 1 << 0;
pub const HV_FLUSH_ALL_VIRTUAL_ADDRESS_SPACES: u64 = 1 << 1;
pub const HV_FLUSH_NON_GLOBAL_MAPPINGS_ONLY: u64 = 1 << 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvTlbFlush {
    pub flags: u64,
    pub processor_mask: u64,
    pub address_space: u64,
    pub start_gva: u64,
    pub address_count: u32,
}

impl HvTlbFlush {
    pub const fn all(address_space: u64) -> Self {
        Self {
            flags: HV_FLUSH_ALL_PROCESSORS | HV_FLUSH_ALL_VIRTUAL_ADDRESS_SPACES,
            processor_mask: u64::MAX,
            address_space,
            start_gva: 0,
            address_count: 0,
        }
    }

    pub const fn single_address(address_space: u64, gva: u64) -> Self {
        Self {
            flags: HV_FLUSH_NON_GLOBAL_MAPPINGS_ONLY,
            processor_mask: 1,
            address_space,
            start_gva: gva,
            address_count: 1,
        }
    }
}

pub const fn flush_covers_all(flush: HvTlbFlush) -> bool {
    flush.flags & HV_FLUSH_ALL_VIRTUAL_ADDRESS_SPACES != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_flush_sets_all_processor_and_address_space_bits() {
        let flush = HvTlbFlush::all(0);
        assert!(flush_covers_all(flush));
        assert_eq!(flush.processor_mask, u64::MAX);
    }
}
