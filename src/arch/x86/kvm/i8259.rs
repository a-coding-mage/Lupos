//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/i8259.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/i8259.c
//! KVM-emulated 8259A cascaded PIC.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/i8259.c

// Two 8259A chips are wired master/slave (IRQ2 cascade). Each chip holds
// IRR (interrupt request), ISR (in-service), IMR (mask) bytes, plus
// the priority rotation state. We model the priority-encoded
// next-pending lookup.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Pic {
    pub irr: u8,
    pub isr: u8,
    pub imr: u8,
}

pub const fn next_pending(pic: Pic) -> Option<u8> {
    let pending = pic.irr & !pic.imr;
    if pending == 0 {
        return None;
    }
    Some(pending.trailing_zeros() as u8)
}

pub fn raise(pic: &mut Pic, irq: u8) {
    pic.irr |= 1 << (irq & 0x07);
}

pub fn acknowledge(pic: &mut Pic, irq: u8) {
    let bit = 1u8 << (irq & 0x07);
    pic.irr &= !bit;
    pic.isr |= bit;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_irq_does_not_become_pending() {
        let pic = Pic {
            irr: 0x02,
            isr: 0,
            imr: 0x02,
        };
        assert_eq!(next_pending(pic), None);
    }

    #[test]
    fn lowest_numbered_unmasked_irq_wins() {
        let pic = Pic {
            irr: 0b0001_0100,
            isr: 0,
            imr: 0,
        };
        assert_eq!(next_pending(pic), Some(2));
    }
}
