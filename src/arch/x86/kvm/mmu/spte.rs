//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/mmu/spte.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/mmu/spte.c
//! Shadow page table entry encoder.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/mmu/spte.c

// Each SPTE encodes: present, writable, user, accessed, dirty, plus the
// host physical address shifted to bit[51:12]. We model the encoder
// over a small SpteFields struct.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SpteFields {
    pub host_pfn: u64,
    pub writable: bool,
    pub user: bool,
    pub accessed: bool,
    pub dirty: bool,
    pub no_exec: bool,
}

pub const SPTE_PRESENT: u64 = 1 << 0;
pub const SPTE_WRITABLE: u64 = 1 << 1;
pub const SPTE_USER: u64 = 1 << 2;
pub const SPTE_ACCESSED: u64 = 1 << 5;
pub const SPTE_DIRTY: u64 = 1 << 6;
pub const SPTE_NX: u64 = 1u64 << 63;

pub const fn encode_spte(fields: SpteFields) -> u64 {
    let mut value = SPTE_PRESENT;
    if fields.writable {
        value |= SPTE_WRITABLE;
    }
    if fields.user {
        value |= SPTE_USER;
    }
    if fields.accessed {
        value |= SPTE_ACCESSED;
    }
    if fields.dirty {
        value |= SPTE_DIRTY;
    }
    if fields.no_exec {
        value |= SPTE_NX;
    }
    value |= (fields.host_pfn & 0x000f_ffff_ffff_f000) << 0;
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writable_dirty_spte_sets_bits_1_and_6() {
        let f = SpteFields {
            host_pfn: 0,
            writable: true,
            user: false,
            accessed: false,
            dirty: true,
            no_exec: false,
        };
        let v = encode_spte(f);
        assert!(v & SPTE_PRESENT != 0);
        assert!(v & SPTE_WRITABLE != 0);
        assert!(v & SPTE_DIRTY != 0);
    }

    #[test]
    fn no_exec_uses_bit_63() {
        let f = SpteFields {
            host_pfn: 0,
            writable: false,
            user: false,
            accessed: false,
            dirty: false,
            no_exec: true,
        };
        assert!(encode_spte(f) & (1u64 << 63) != 0);
    }
}
