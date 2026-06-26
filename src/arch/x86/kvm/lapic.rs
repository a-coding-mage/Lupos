//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/lapic.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/lapic.c
//! KVM-emulated Local APIC.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/lapic.c

// Each vCPU has a 4 KiB LAPIC backing page indexed by 16-bit register
// offset. Common offsets: 0x020 = ID, 0x030 = VERSION, 0x080 = TPR,
// 0x100..=0x170 = ISR, 0x200..=0x270 = IRR. We model the offset map
// and a small write/read helper.

use crate::include::uapi::errno::EINVAL;

pub const APIC_ID: u16 = 0x020;
pub const APIC_VERSION: u16 = 0x030;
pub const APIC_TPR: u16 = 0x080;
pub const APIC_EOI: u16 = 0x0b0;
pub const APIC_LDR: u16 = 0x0d0;
pub const APIC_DFR: u16 = 0x0e0;
pub const APIC_ICR_LOW: u16 = 0x300;
pub const APIC_ICR_HIGH: u16 = 0x310;

#[derive(Debug)]
pub struct LapicPage {
    bytes: [u32; 1024],
}

impl LapicPage {
    pub const fn new() -> Self {
        Self { bytes: [0; 1024] }
    }

    pub fn read(&self, offset: u16) -> Result<u32, i32> {
        if offset & 0xf != 0 || offset >= 0x1000 {
            return Err(EINVAL);
        }
        Ok(self.bytes[(offset as usize) / 4])
    }

    pub fn write(&mut self, offset: u16, value: u32) -> Result<(), i32> {
        if offset & 0xf != 0 || offset >= 0x1000 {
            return Err(EINVAL);
        }
        self.bytes[(offset as usize) / 4] = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misaligned_offset_returns_einval() {
        let mut p = LapicPage::new();
        assert_eq!(p.write(0x021, 0), Err(EINVAL));
        assert_eq!(p.read(0x021), Err(EINVAL));
    }

    #[test]
    fn round_trips_simple_register_writes() {
        let mut p = LapicPage::new();
        p.write(APIC_ID, 0xdead).unwrap();
        assert_eq!(p.read(APIC_ID).unwrap(), 0xdead);
    }
}
