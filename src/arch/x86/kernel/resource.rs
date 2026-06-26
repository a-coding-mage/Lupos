//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/resource.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/resource.c
//! x86 resource reservation clipping.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/resource.c

#![allow(dead_code)]

use crate::arch::x86::include::uapi::asm::bootparam::BootE820Entry;

pub const IORESOURCE_MEM: u32 = 0x0000_0200;
pub const BIOS_ROM_BASE: u64 = 0x000c_0000;
pub const BIOS_ROM_END: u64 = 0x000f_ffff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resource {
    pub start: u64,
    pub end: u64,
    pub flags: u32,
}

impl Resource {
    pub const fn size(self) -> u64 {
        if self.end >= self.start {
            self.end - self.start + 1
        } else {
            0
        }
    }
}

pub fn resource_clip(res: &mut Resource, start: u64, end: u64) {
    if res.end < start || res.start > end {
        return;
    }

    let low = if res.start < start {
        start - res.start
    } else {
        0
    };
    let high = if res.end > end { res.end - end } else { 0 };

    if low > high {
        res.end = start.saturating_sub(1);
    } else {
        res.start = end.saturating_add(1);
    }
}

pub fn remove_e820_regions(avail: &mut Resource, e820: &[BootE820Entry], pci_use_e820: bool) {
    if !pci_use_e820 {
        return;
    }
    for entry in e820 {
        if entry.length == 0 {
            continue;
        }
        resource_clip(
            avail,
            entry.base_addr,
            entry.base_addr.saturating_add(entry.length - 1),
        );
    }
}

pub fn arch_remove_reservations(avail: &mut Resource, e820: &[BootE820Entry], pci_use_e820: bool) {
    if avail.flags & IORESOURCE_MEM == 0 {
        return;
    }
    resource_clip(avail, BIOS_ROM_BASE, BIOS_ROM_END);
    remove_e820_regions(avail, e820, pci_use_e820);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_keeps_larger_side_of_conflict() {
        let mut r = Resource {
            start: 0x1000,
            end: 0x9fff,
            flags: IORESOURCE_MEM,
        };
        resource_clip(&mut r, 0x3000, 0x3fff);
        assert_eq!(r.start, 0x4000);

        let mut r = Resource {
            start: 0x1000,
            end: 0x9fff,
            flags: IORESOURCE_MEM,
        };
        resource_clip(&mut r, 0x8000, 0x8fff);
        assert_eq!(r.end, 0x7fff);
    }

    #[test]
    fn arch_remove_reservations_trims_bios_and_e820_memory() {
        let mut r = Resource {
            start: 0xa0000,
            end: 0x1fffff,
            flags: IORESOURCE_MEM,
        };
        let e820 = [BootE820Entry {
            base_addr: 0x100000,
            length: 0x40000,
            region_type: 2,
        }];
        arch_remove_reservations(&mut r, &e820, true);
        assert_eq!(r.start, 0x140000);
    }
}
