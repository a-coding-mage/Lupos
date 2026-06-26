//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/irq.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/irq.c
//! KVM interrupt routing common helpers.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/irq.c

// `irq.c` defines the in-kernel IRQ delivery loop used by both PIC and
// IOAPIC. The function `kvm_get_apic_interrupt()` finds the highest
// priority pending vector. We model the priority predicate.

pub const fn highest_priority(pending: u32) -> Option<u8> {
    if pending == 0 {
        return None;
    }
    Some((31 - pending.leading_zeros()) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_msb_of_pending_bitmap() {
        assert_eq!(highest_priority(0b0000_0110), Some(2));
        assert_eq!(highest_priority(0), None);
        assert_eq!(highest_priority(0x8000_0000), Some(31));
    }
}
