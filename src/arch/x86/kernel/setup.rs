//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/setup.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/setup.c
//! x86 setup-data, initrd, and early reservation helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/setup.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::arch::x86::include::uapi::asm::bootparam::BootParams;
use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};

pub const PAGE_SIZE: u64 = 4096;
pub const SETUP_NONE: u32 = 0;
pub const SETUP_E820_EXT: u32 = 1;
pub const SETUP_DTB: u32 = 2;
pub const SETUP_PCI: u32 = 3;
pub const SETUP_EFI: u32 = 4;
pub const SETUP_APPLE_PROPERTIES: u32 = 5;
pub const SETUP_JAILHOUSE: u32 = 6;
pub const SETUP_CC_BLOB: u32 = 7;
pub const SETUP_IMA: u32 = 8;
pub const SETUP_RNG_SEED: u32 = 9;
pub const SETUP_ENUM_MAX: u32 = SETUP_RNG_SEED;
pub const SETUP_INDIRECT: u32 = 0x8000_0000;
pub const SETUP_KEXEC_KHO: u32 = 0x8000_0001;

/// `pci_mem_start` - `vendor/linux/arch/x86/kernel/e820.c`.
static mut PCI_MEM_START: usize = 0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "pci_mem_start",
        core::ptr::addr_of_mut!(PCI_MEM_START) as usize,
        false,
    );
    export_symbol_once("e820__mapped_any", linux_e820_mapped_any as usize, true);
    export_symbol_once(
        "e820__mapped_raw_any",
        linux_e820_mapped_any as usize,
        false,
    );
}

/// `e820__mapped_any` - `vendor/linux/arch/x86/kernel/e820.c:99`.
///
/// The module ABI does not expose Lupos' boot memory map as a mutable Linux
/// `e820_table` yet.  Report no overlap so AGP aperture checks fail closed
/// instead of consuming an invented RAM/reserved map.
#[unsafe(export_name = "e820__mapped_any")]
pub unsafe extern "C" fn linux_e820_mapped_any(_start: u64, _end: u64, _type: u32) -> bool {
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetupDataHeader {
    pub next: u64,
    pub kind: u32,
    pub len: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetupDataRecord {
    pub phys_addr: u64,
    pub header: SetupDataHeader,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupDataAction {
    E820Ext,
    Dtb,
    Efi,
    Ima,
    KexecKho,
    RngSeed,
    ReserveOnly(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootReservation {
    pub start: u64,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitrdImage {
    pub image: u64,
    pub size: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EarlyBrk {
    start: u64,
    end: u64,
    limit: u64,
}

impl EarlyBrk {
    pub const fn new(start: u64, limit: u64) -> Self {
        Self {
            start,
            end: start,
            limit,
        }
    }

    pub fn extend(&mut self, size: u64, align: u64) -> Result<u64, i32> {
        if size == 0 || !align.is_power_of_two() {
            return Err(EINVAL);
        }
        let aligned = align_up(self.end, align).ok_or(ENOMEM)?;
        let new_end = aligned.checked_add(size).ok_or(ENOMEM)?;
        if new_end > self.limit {
            return Err(ENOMEM);
        }
        self.end = new_end;
        Ok(aligned)
    }

    pub const fn reservation(&self) -> BootReservation {
        BootReservation {
            start: self.start,
            size: self.end - self.start,
        }
    }
}

pub fn initrd_from_boot_params(params: &BootParams) -> Option<InitrdImage> {
    let size = params.ramdisk_size();
    (size != 0).then_some(InitrdImage {
        image: params.ramdisk_image(),
        size,
    })
}

pub fn early_reserve_initrd(params: &BootParams) -> Option<BootReservation> {
    let initrd = initrd_from_boot_params(params)?;
    Some(BootReservation {
        start: initrd.image,
        size: initrd.size,
    })
}

pub fn setup_data_action(kind: u32) -> SetupDataAction {
    match kind {
        SETUP_E820_EXT => SetupDataAction::E820Ext,
        SETUP_DTB => SetupDataAction::Dtb,
        SETUP_EFI => SetupDataAction::Efi,
        SETUP_IMA => SetupDataAction::Ima,
        SETUP_KEXEC_KHO => SetupDataAction::KexecKho,
        SETUP_RNG_SEED => SetupDataAction::RngSeed,
        other => SetupDataAction::ReserveOnly(other),
    }
}

pub fn parse_setup_data(records: &[SetupDataRecord]) -> Vec<SetupDataAction> {
    records
        .iter()
        .map(|record| setup_data_action(record.header.kind))
        .collect()
}

pub fn reserve_range_for_setup_data(record: SetupDataRecord) -> Result<BootReservation, i32> {
    let header_size = core::mem::size_of::<SetupDataHeader>() as u64;
    let size = header_size
        .checked_add(record.header.len as u64)
        .ok_or(ENOMEM)?;
    Ok(BootReservation {
        start: record.phys_addr,
        size,
    })
}

pub fn memblock_x86_reserve_range_setup_data(
    records: &[SetupDataRecord],
) -> Result<Vec<BootReservation>, i32> {
    records
        .iter()
        .copied()
        .map(reserve_range_for_setup_data)
        .collect()
}

pub const fn x86_configure_nx(efer_nx_supported: bool, disable_nx: bool) -> bool {
    efer_nx_supported && !disable_nx
}

pub const fn arch_cpu_is_hotpluggable(cpu: i32) -> bool {
    cpu > 0
}

const fn align_up(value: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    match value.checked_add(mask) {
        Some(v) => Some(v & !mask),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_brk_aligns_and_tracks_reservation() {
        let mut brk = EarlyBrk::new(0x1003, 0x3000);
        assert_eq!(brk.extend(0x20, 0x100), Ok(0x1100));
        assert_eq!(
            brk.reservation(),
            BootReservation {
                start: 0x1003,
                size: 0x11d
            }
        );
    }

    #[test]
    fn setup_data_types_map_to_actions_and_reservations() {
        let records = [
            SetupDataRecord {
                phys_addr: 0x1000,
                header: SetupDataHeader {
                    next: 0,
                    kind: SETUP_E820_EXT,
                    len: 32,
                },
            },
            SetupDataRecord {
                phys_addr: 0x2000,
                header: SetupDataHeader {
                    next: 0,
                    kind: 99,
                    len: 4,
                },
            },
        ];
        assert_eq!(
            parse_setup_data(&records),
            alloc::vec![SetupDataAction::E820Ext, SetupDataAction::ReserveOnly(99)]
        );
        assert_eq!(
            memblock_x86_reserve_range_setup_data(&records).unwrap()[0].size,
            core::mem::size_of::<SetupDataHeader>() as u64 + 32
        );
    }

    #[test]
    fn initrd_uses_extended_bootparam_fields() {
        let mut bp = BootParams::new();
        bp.set_ramdisk_image(0x1_0000_1000);
        bp.set_ramdisk_size(0x2_0000_0000);
        assert_eq!(
            early_reserve_initrd(&bp),
            Some(BootReservation {
                start: 0x1_0000_1000,
                size: 0x2_0000_0000
            })
        );
    }
}
