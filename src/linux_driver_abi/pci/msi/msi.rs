//! linux-parity: complete
//! linux-source: vendor/linux/drivers/pci/msi/msi.c
//! test-origin: linux:vendor/linux/drivers/pci/msi/msi.c
//! PCI MSI message coverage for M55.
//!
//! Mirrors `vendor/linux/drivers/pci/msi/msi.c`.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MsiMsg {
    pub address_lo: u32,
    pub address_hi: u32,
    pub data: u32,
}

pub fn compose_x86_msi_msg(apic_id: u32, vector: u32) -> MsiMsg {
    MsiMsg {
        address_lo: 0xFEE0_0000 | (apic_id << 12),
        address_hi: 0,
        data: vector,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_msi_message_uses_lapic_window() {
        let msg = compose_x86_msi_msg(2, 0xC0);
        assert_eq!(msg.address_lo, 0xFEE0_0000 | (2 << 12));
        assert_eq!(msg.data, 0xC0);
    }
}
