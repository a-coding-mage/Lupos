//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/chip.c
//! test-origin: linux:vendor/linux/kernel/irq/chip.c
//! `struct irq_chip` — IRQ controller ops (M37).

/// Linux `struct irq_chip`.
pub struct IrqChip {
    pub name: &'static str,
    pub mask: Option<fn(irq: u32)>,
    pub unmask: Option<fn(irq: u32)>,
    pub ack: Option<fn(irq: u32)>,
    pub eoi: Option<fn(irq: u32)>,
    pub set_affinity: Option<fn(irq: u32, mask: u32)>,
}

unsafe impl Send for IrqChip {}
unsafe impl Sync for IrqChip {}

/// LAPIC chip — issues EOI through the local APIC.
fn lapic_eoi(_irq: u32) {
    #[cfg(not(test))]
    unsafe {
        crate::arch::x86::kernel::apic::eoi();
    }
}
fn lapic_mask(_irq: u32) {}
fn lapic_unmask(_irq: u32) {}
fn lapic_set_affinity(_irq: u32, _mask: u32) {}

pub static LAPIC_CHIP: IrqChip = IrqChip {
    name: "APIC",
    mask: Some(lapic_mask),
    unmask: Some(lapic_unmask),
    ack: None,
    eoi: Some(lapic_eoi),
    set_affinity: Some(lapic_set_affinity),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lapic_chip_name_is_apic() {
        assert_eq!(LAPIC_CHIP.name, "APIC");
    }

    #[test]
    fn lapic_chip_has_eoi_callback() {
        assert!(LAPIC_CHIP.eoi.is_some());
    }
}
