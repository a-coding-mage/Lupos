//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/reboot_fixups_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/reboot_fixups_32.c
//! 32-bit x86 reboot fixup action model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/reboot_fixups_32.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RebootDevice {
    pub vendor: u16,
    pub device: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebootFixup {
    CyrixCs5530,
    AmdCs5536,
    NatSemiSc1100,
    RdcR6030,
    IntelCe4100,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebootFixupAction {
    PciWrite8 { reg: u16, value: u8 },
    IoWrite8 { port: u16, value: u8 },
    MsrWrite { msr: u32, value: u64 },
}

pub fn select_fixup(dev: RebootDevice) -> Option<RebootFixup> {
    match (dev.vendor, dev.device) {
        (0x1078, 0x0100) => Some(RebootFixup::CyrixCs5530),
        (0x1022, 0x2090) => Some(RebootFixup::AmdCs5536),
        (0x100b, 0x0510) => Some(RebootFixup::NatSemiSc1100),
        (0x17f3, 0x6030) => Some(RebootFixup::RdcR6030),
        (0x8086, 0x0708) => Some(RebootFixup::IntelCe4100),
        _ => None,
    }
}

pub fn actions_for_fixup(fixup: RebootFixup) -> Vec<RebootFixupAction> {
    match fixup {
        RebootFixup::CyrixCs5530 => alloc::vec![RebootFixupAction::PciWrite8 {
            reg: 0x44,
            value: 0x01,
        }],
        RebootFixup::AmdCs5536 => alloc::vec![RebootFixupAction::MsrWrite {
            msr: 0x5140_0017,
            value: 1,
        }],
        RebootFixup::NatSemiSc1100 => alloc::vec![RebootFixupAction::IoWrite8 {
            port: 0xcf9,
            value: 0x06,
        }],
        RebootFixup::RdcR6030 => alloc::vec![RebootFixupAction::PciWrite8 {
            reg: 0x40,
            value: 0x01,
        }],
        RebootFixup::IntelCe4100 => alloc::vec![RebootFixupAction::IoWrite8 {
            port: 0xcf9,
            value: 0x06,
        }],
    }
}

pub fn reboot_fixups_32(in_interrupt: bool, devices: &[RebootDevice]) -> Vec<RebootFixupAction> {
    if in_interrupt {
        return Vec::new();
    }
    for dev in devices {
        if let Some(fixup) = select_fixup(*dev) {
            return actions_for_fixup(fixup);
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_known_fixups_and_skips_interrupt_context() {
        let dev = RebootDevice {
            vendor: 0x1022,
            device: 0x2090,
        };
        assert_eq!(select_fixup(dev), Some(RebootFixup::AmdCs5536));
        assert!(reboot_fixups_32(true, &[dev]).is_empty());
        assert!(!reboot_fixups_32(false, &[dev]).is_empty());
    }
}
