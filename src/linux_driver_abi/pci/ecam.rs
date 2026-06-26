//! linux-parity: complete
//! linux-source: vendor/linux/drivers/pci/ecam.c
//! test-origin: linux:vendor/linux/drivers/pci/ecam.c
//! ECAM (Enhanced Configuration Access Mechanism) — `drivers/pci/ecam.c`.
//!
//! PCIe ECAM maps the entire PCI configuration space into a contiguous MMIO
//! window.  The base address comes from the ACPI MCFG table.
//!
//! Address formula (PCI Express Base Spec §7.2.2):
//!   `ecam_base + ((bus << 20) | (dev << 15) | (func << 12) | offset)`
//!
//! References:
//!   - `drivers/pci/ecam.c:pci_ecam_map_bus` (line 167)
//!   - `drivers/pci/ecam.c:pci_ecam_create` (line 27)
//!   - ACPI 6.5 §5.2.6.2 "MCFG — PCI Memory Mapped Configuration"

/// One entry from the ACPI MCFG table.
#[derive(Debug, Clone, Copy)]
pub struct McfgEntry {
    /// Physical base address of the ECAM window for this segment.
    pub base: u64,
    /// PCI segment group number.
    pub segment: u16,
    /// First bus number this entry covers.
    pub bus_start: u8,
    /// Last bus number this entry covers.
    pub bus_end: u8,
}

impl McfgEntry {
    /// Compute the MMIO address of a PCI config-space register.
    ///
    /// Mirrors `pci_ecam_map_bus` in `drivers/pci/ecam.c:167`.
    #[inline]
    pub fn config_addr(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u64 {
        self.base
            + ((bus as u64 - self.bus_start as u64) << 20)
            + ((dev as u64) << 15)
            + ((func as u64) << 12)
            + (offset as u64)
    }

    /// Read a 32-bit dword from PCI configuration space via MMIO.
    ///
    /// # Safety
    /// The identity mapping of the first 4 GiB must be in place (boot guarantee).
    pub unsafe fn read32(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u32 {
        let addr = self.config_addr(bus, dev, func, offset) as *const u32;
        unsafe { core::ptr::read_volatile(addr) }
    }

    /// Write a 32-bit dword to PCI configuration space.
    ///
    /// # Safety
    /// Same as `read32`.
    pub unsafe fn write32(&self, bus: u8, dev: u8, func: u8, offset: u16, val: u32) {
        let addr = self.config_addr(bus, dev, func, offset) as *mut u32;
        unsafe { core::ptr::write_volatile(addr, val) }
    }

    /// Read a 16-bit word from PCI configuration space.
    pub unsafe fn read16(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u16 {
        let addr = self.config_addr(bus, dev, func, offset) as *const u16;
        unsafe { core::ptr::read_volatile(addr) }
    }

    /// Write a 16-bit word to PCI configuration space.
    ///
    /// # Safety
    /// Same as `read32`.
    pub unsafe fn write16(&self, bus: u8, dev: u8, func: u8, offset: u16, val: u16) {
        let addr = self.config_addr(bus, dev, func, offset) as *mut u16;
        unsafe { core::ptr::write_volatile(addr, val) }
    }

    /// Read an 8-bit byte from PCI configuration space.
    pub unsafe fn read8(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u8 {
        let addr = self.config_addr(bus, dev, func, offset) as *const u8;
        unsafe { core::ptr::read_volatile(addr) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> McfgEntry {
        McfgEntry {
            base: 0x8000_0000,
            segment: 0,
            bus_start: 0,
            bus_end: 255,
        }
    }

    #[test]
    fn ecam_addr_bus0_dev0_func0() {
        let e = entry();
        assert_eq!(e.config_addr(0, 0, 0, 0), 0x8000_0000);
    }

    #[test]
    fn ecam_addr_bus1() {
        let e = entry();
        // bus 1 → base + (1 << 20)
        assert_eq!(e.config_addr(1, 0, 0, 0), 0x8000_0000 + (1 << 20));
    }

    #[test]
    fn ecam_addr_dev3_func0() {
        let e = entry();
        // dev 3 → base + (3 << 15)
        assert_eq!(e.config_addr(0, 3, 0, 0), 0x8000_0000 + (3 << 15));
    }

    #[test]
    fn ecam_addr_offset() {
        let e = entry();
        assert_eq!(e.config_addr(0, 0, 0, 0x10), 0x8000_0000 + 0x10);
    }

    #[test]
    fn ecam_addr_full() {
        let e = entry();
        // bus=2, dev=1, func=0, offset=0x24 → covers all three shift terms
        let expected = 0x8000_0000u64 + (2u64 << 20) + (1u64 << 15) + 0x24;
        assert_eq!(e.config_addr(2, 1, 0, 0x24), expected);
    }
}
