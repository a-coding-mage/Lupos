//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/access.c
//! test-origin: linux:vendor/linux/drivers/pci/access.c
//! PCI config-space access helpers for Linux-built PCI modules.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::pci::device::{
    LinuxPciBus, LinuxPciDev, linux_pci_config_read, linux_pci_config_write, pci_dev_put,
    pci_get_slot,
};
use crate::linux_driver_abi::pci::pci::pci_find_capability;

pub const PCIBIOS_SUCCESSFUL: i32 = 0x00;
pub const PCIBIOS_DEVICE_NOT_FOUND: i32 = 0x86;
pub const PCIBIOS_BAD_REGISTER_NUMBER: i32 = 0x87;
const PCI_CAP_ID_EXP: i32 = 0x10;
const PCI_EXP_FLAGS: i32 = 0x02;
const PCI_EXP_FLAGS_VERS: u16 = 0x000f;
const PCI_EXP_FLAGS_TYPE: u16 = 0x00f0;
const PCI_EXP_FLAGS_SLOT: u16 = 0x0100;
const PCI_EXP_TYPE_ENDPOINT: u16 = 0x0;
const PCI_EXP_TYPE_LEG_END: u16 = 0x1;
const PCI_EXP_TYPE_ROOT_PORT: u16 = 0x4;
const PCI_EXP_TYPE_UPSTREAM: u16 = 0x5;
const PCI_EXP_TYPE_DOWNSTREAM: u16 = 0x6;
const PCI_EXP_TYPE_PCI_BRIDGE: u16 = 0x7;
const PCI_EXP_TYPE_PCIE_BRIDGE: u16 = 0x8;
const PCI_EXP_TYPE_RC_EC: u16 = 0xa;
const PCI_EXP_DEVCAP: i32 = 0x04;
const PCI_EXP_DEVCTL: i32 = 0x08;
const PCI_EXP_DEVSTA: i32 = 0x0a;
const PCI_EXP_LNKCAP: i32 = 0x0c;
const PCI_EXP_LNKCTL: i32 = 0x10;
const PCI_EXP_LNKSTA: i32 = 0x12;
const PCI_EXP_SLTCAP: i32 = 0x14;
const PCI_EXP_SLTCTL: i32 = 0x18;
const PCI_EXP_SLTSTA: i32 = 0x1a;
const PCI_EXP_SLTSTA_PDS: u16 = 0x0040;
const PCI_EXP_RTCTL: i32 = 0x1c;
const PCI_EXP_RTCAP: i32 = 0x1e;
const PCI_EXP_RTSTA: i32 = 0x20;
const PCI_EXP_DEVCAP2: i32 = 0x24;
const PCI_EXP_DEVCTL2: i32 = 0x28;
const PCI_EXP_LNKCAP2: i32 = 0x2c;
const PCI_EXP_LNKCTL2: i32 = 0x30;
const PCI_EXP_LNKSTA2: i32 = 0x32;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "pci_bus_read_config_byte",
        pci_bus_read_config_byte as usize,
        false,
    );
    export_symbol_once(
        "pci_bus_read_config_word",
        pci_bus_read_config_word as usize,
        false,
    );
    export_symbol_once(
        "pci_bus_read_config_dword",
        pci_bus_read_config_dword as usize,
        false,
    );
    export_symbol_once(
        "pci_bus_write_config_byte",
        pci_bus_write_config_byte as usize,
        false,
    );
    export_symbol_once(
        "pci_bus_write_config_word",
        pci_bus_write_config_word as usize,
        false,
    );
    export_symbol_once(
        "pci_bus_write_config_dword",
        pci_bus_write_config_dword as usize,
        false,
    );
    export_symbol_once("pci_read_config_byte", pci_read_config_byte as usize, false);
    export_symbol_once("pci_read_config_word", pci_read_config_word as usize, false);
    export_symbol_once(
        "pci_read_config_dword",
        pci_read_config_dword as usize,
        false,
    );
    export_symbol_once(
        "pci_write_config_byte",
        pci_write_config_byte as usize,
        false,
    );
    export_symbol_once(
        "pci_write_config_word",
        pci_write_config_word as usize,
        false,
    );
    export_symbol_once(
        "pci_write_config_dword",
        pci_write_config_dword as usize,
        false,
    );
    export_symbol_once(
        "pcie_capability_read_word",
        pcie_capability_read_word as usize,
        false,
    );
    export_symbol_once(
        "pcie_capability_write_word",
        pcie_capability_write_word as usize,
        false,
    );
    export_symbol_once(
        "pcie_capability_clear_and_set_word_unlocked",
        pcie_capability_clear_and_set_word_unlocked as usize,
        false,
    );
    export_symbol_once(
        "pcie_capability_clear_and_set_word_locked",
        pcie_capability_clear_and_set_word_locked as usize,
        false,
    );
}

fn read_config(dev: *const c_void, offset: i32, width: usize) -> Result<u32, i32> {
    if dev.is_null() {
        return Err(PCIBIOS_DEVICE_NOT_FOUND);
    }
    let offset = usize::try_from(offset).map_err(|_| PCIBIOS_BAD_REGISTER_NUMBER)?;
    linux_pci_config_read(dev, offset, width).ok_or(PCIBIOS_BAD_REGISTER_NUMBER)
}

fn write_config(dev: *const c_void, offset: i32, width: usize, value: u32) -> i32 {
    if dev.is_null() {
        return PCIBIOS_DEVICE_NOT_FOUND;
    }
    let Ok(offset) = usize::try_from(offset) else {
        return PCIBIOS_BAD_REGISTER_NUMBER;
    };
    if linux_pci_config_write(dev, offset, width, value) {
        PCIBIOS_SUCCESSFUL
    } else {
        PCIBIOS_BAD_REGISTER_NUMBER
    }
}

fn pcie_cap_offset(dev: *const c_void) -> Option<i32> {
    if dev.is_null() {
        return None;
    }
    let cap = unsafe { pci_find_capability(dev.cast_mut(), PCI_CAP_ID_EXP) };
    (cap != 0).then_some(cap as i32)
}

fn pcie_flags(dev: *const c_void) -> Option<u16> {
    let cap = pcie_cap_offset(dev)?;
    read_config(dev, cap + PCI_EXP_FLAGS, 2)
        .ok()
        .map(|value| value as u16)
}

fn pcie_cap_version(dev: *const c_void) -> u16 {
    pcie_flags(dev).unwrap_or(0) & PCI_EXP_FLAGS_VERS
}

fn pcie_type(dev: *const c_void) -> u16 {
    (pcie_flags(dev).unwrap_or(0) & PCI_EXP_FLAGS_TYPE) >> 4
}

fn pcie_cap_has_lnkctl(dev: *const c_void) -> bool {
    matches!(
        pcie_type(dev),
        PCI_EXP_TYPE_ENDPOINT
            | PCI_EXP_TYPE_LEG_END
            | PCI_EXP_TYPE_ROOT_PORT
            | PCI_EXP_TYPE_UPSTREAM
            | PCI_EXP_TYPE_DOWNSTREAM
            | PCI_EXP_TYPE_PCI_BRIDGE
            | PCI_EXP_TYPE_PCIE_BRIDGE
    )
}

fn pcie_downstream_port(dev: *const c_void) -> bool {
    matches!(
        pcie_type(dev),
        PCI_EXP_TYPE_ROOT_PORT | PCI_EXP_TYPE_DOWNSTREAM
    )
}

fn pcie_cap_has_sltctl(dev: *const c_void) -> bool {
    pcie_downstream_port(dev)
        && pcie_flags(dev).is_some_and(|flags| flags & PCI_EXP_FLAGS_SLOT != 0)
}

fn pcie_cap_has_rtctl(dev: *const c_void) -> bool {
    matches!(pcie_type(dev), PCI_EXP_TYPE_ROOT_PORT | PCI_EXP_TYPE_RC_EC)
}

fn pcie_capability_reg_implemented(dev: *const c_void, pos: i32) -> bool {
    if pcie_cap_offset(dev).is_none() {
        return false;
    }
    match pos {
        PCI_EXP_FLAGS | PCI_EXP_DEVCAP | PCI_EXP_DEVCTL | PCI_EXP_DEVSTA => true,
        PCI_EXP_LNKCAP | PCI_EXP_LNKCTL | PCI_EXP_LNKSTA => pcie_cap_has_lnkctl(dev),
        PCI_EXP_SLTCAP | PCI_EXP_SLTCTL | PCI_EXP_SLTSTA => pcie_cap_has_sltctl(dev),
        PCI_EXP_RTCTL | PCI_EXP_RTCAP | PCI_EXP_RTSTA => pcie_cap_has_rtctl(dev),
        PCI_EXP_DEVCAP2 | PCI_EXP_DEVCTL2 => pcie_cap_version(dev) > 1,
        PCI_EXP_LNKCAP2 | PCI_EXP_LNKCTL2 | PCI_EXP_LNKSTA2 => {
            pcie_cap_has_lnkctl(dev) && pcie_cap_version(dev) > 1
        }
        _ => false,
    }
}

fn pci_dev_for_bus_devfn(bus: *mut LinuxPciBus, devfn: u32) -> Result<*mut LinuxPciDev, i32> {
    if bus.is_null() {
        return Err(PCIBIOS_DEVICE_NOT_FOUND);
    }
    let dev = unsafe { pci_get_slot(bus, devfn) };
    if dev.is_null() {
        Err(PCIBIOS_DEVICE_NOT_FOUND)
    } else {
        Ok(dev)
    }
}

fn read_bus_config(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    width: usize,
) -> Result<u32, i32> {
    let dev = pci_dev_for_bus_devfn(bus, devfn)?;
    let ret = read_config(dev.cast_const().cast(), offset, width);
    unsafe { pci_dev_put(dev) };
    ret
}

fn write_bus_config(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    width: usize,
    value: u32,
) -> i32 {
    let Ok(dev) = pci_dev_for_bus_devfn(bus, devfn) else {
        return PCIBIOS_DEVICE_NOT_FOUND;
    };
    let ret = write_config(dev.cast_const().cast(), offset, width, value);
    unsafe { pci_dev_put(dev) };
    ret
}

/// `pci_bus_read_config_byte` - `vendor/linux/drivers/pci/access.c:81`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_bus_read_config_byte(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    value: *mut u8,
) -> i32 {
    match read_bus_config(bus, devfn, offset, 1) {
        Ok(read) => {
            if !value.is_null() {
                unsafe { *value = read as u8 };
            }
            PCIBIOS_SUCCESSFUL
        }
        Err(err) => {
            if !value.is_null() {
                unsafe { *value = u8::MAX };
            }
            err
        }
    }
}

/// `pci_bus_read_config_word` - `vendor/linux/drivers/pci/access.c:82`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_bus_read_config_word(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    value: *mut u16,
) -> i32 {
    match read_bus_config(bus, devfn, offset, 2) {
        Ok(read) => {
            if !value.is_null() {
                unsafe { *value = read as u16 };
            }
            PCIBIOS_SUCCESSFUL
        }
        Err(err) => {
            if !value.is_null() {
                unsafe { *value = u16::MAX };
            }
            err
        }
    }
}

/// `pci_bus_read_config_dword` - `vendor/linux/drivers/pci/access.c:83`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_bus_read_config_dword(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    value: *mut u32,
) -> i32 {
    match read_bus_config(bus, devfn, offset, 4) {
        Ok(read) => {
            if !value.is_null() {
                unsafe { *value = read };
            }
            PCIBIOS_SUCCESSFUL
        }
        Err(err) => {
            if !value.is_null() {
                unsafe { *value = u32::MAX };
            }
            err
        }
    }
}

/// `pci_bus_write_config_byte` - `vendor/linux/drivers/pci/access.c:84`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_bus_write_config_byte(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    value: u8,
) -> i32 {
    write_bus_config(bus, devfn, offset, 1, value as u32)
}

/// `pci_bus_write_config_word` - `vendor/linux/drivers/pci/access.c:85`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_bus_write_config_word(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    value: u16,
) -> i32 {
    write_bus_config(bus, devfn, offset, 2, value as u32)
}

/// `pci_bus_write_config_dword` - `vendor/linux/drivers/pci/access.c:86`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_bus_write_config_dword(
    bus: *mut LinuxPciBus,
    devfn: u32,
    offset: i32,
    value: u32,
) -> i32 {
    write_bus_config(bus, devfn, offset, 4, value)
}

/// `pci_read_config_byte` - `vendor/linux/drivers/pci/access.c:560`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_read_config_byte(
    dev: *const c_void,
    offset: i32,
    value: *mut u8,
) -> i32 {
    match read_config(dev, offset, 1) {
        Ok(read) => {
            if !value.is_null() {
                unsafe { *value = read as u8 };
            }
            PCIBIOS_SUCCESSFUL
        }
        Err(err) => {
            if !value.is_null() {
                unsafe { *value = u8::MAX };
            }
            err
        }
    }
}

/// `pci_write_config_byte` - `vendor/linux/drivers/pci/access.c:591`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_write_config_byte(dev: *const c_void, offset: i32, value: u8) -> i32 {
    write_config(dev, offset, 1, value as u32)
}

/// `pci_write_config_word` - `vendor/linux/drivers/pci/access.c:599`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_write_config_word(dev: *const c_void, offset: i32, value: u16) -> i32 {
    write_config(dev, offset, 2, value as u32)
}

/// `pci_write_config_dword` - `vendor/linux/drivers/pci/access.c:607`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_write_config_dword(
    dev: *const c_void,
    offset: i32,
    value: u32,
) -> i32 {
    write_config(dev, offset, 4, value)
}

/// `pci_read_config_word` - `vendor/linux/drivers/pci/access.c:570`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_read_config_word(
    dev: *const c_void,
    offset: i32,
    value: *mut u16,
) -> i32 {
    match read_config(dev, offset, 2) {
        Ok(read) => {
            if !value.is_null() {
                unsafe { *value = read as u16 };
            }
            PCIBIOS_SUCCESSFUL
        }
        Err(err) => {
            if !value.is_null() {
                unsafe { *value = u16::MAX };
            }
            err
        }
    }
}

/// `pci_read_config_dword` - `vendor/linux/drivers/pci/access.c:580`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_read_config_dword(
    dev: *const c_void,
    offset: i32,
    value: *mut u32,
) -> i32 {
    match read_config(dev, offset, 4) {
        Ok(read) => {
            if !value.is_null() {
                unsafe { *value = read };
            }
            PCIBIOS_SUCCESSFUL
        }
        Err(err) => {
            if !value.is_null() {
                unsafe { *value = u32::MAX };
            }
            err
        }
    }
}

/// `pcie_capability_read_word` - `vendor/linux/drivers/pci/access.c:427`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcie_capability_read_word(
    dev: *const c_void,
    pos: i32,
    value: *mut u16,
) -> i32 {
    if !value.is_null() {
        unsafe { *value = 0 };
    }
    if pos & 1 != 0 {
        return PCIBIOS_BAD_REGISTER_NUMBER;
    }
    if pcie_capability_reg_implemented(dev, pos) {
        let Some(cap) = pcie_cap_offset(dev) else {
            return PCIBIOS_DEVICE_NOT_FOUND;
        };
        let ret = unsafe { pci_read_config_word(dev, cap + pos, value) };
        if ret != PCIBIOS_SUCCESSFUL && !value.is_null() {
            unsafe { *value = 0 };
        }
        return ret;
    }
    if pcie_cap_offset(dev).is_some()
        && pcie_downstream_port(dev)
        && pos == PCI_EXP_SLTSTA
        && !value.is_null()
    {
        unsafe { *value = PCI_EXP_SLTSTA_PDS };
    }
    PCIBIOS_SUCCESSFUL
}

/// `pcie_capability_write_word` - `vendor/linux/drivers/pci/access.c:490`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcie_capability_write_word(
    dev: *const c_void,
    pos: i32,
    value: u16,
) -> i32 {
    if pos & 1 != 0 {
        return PCIBIOS_BAD_REGISTER_NUMBER;
    }
    if !pcie_capability_reg_implemented(dev, pos) {
        return PCIBIOS_SUCCESSFUL;
    }
    let Some(cap) = pcie_cap_offset(dev) else {
        return PCIBIOS_DEVICE_NOT_FOUND;
    };
    unsafe { pci_write_config_word(dev, cap + pos, value) }
}

/// `pcie_capability_clear_and_set_word_unlocked` - `vendor/linux/drivers/pci/access.c:514`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcie_capability_clear_and_set_word_unlocked(
    dev: *const c_void,
    pos: i32,
    clear: u16,
    set: u16,
) -> i32 {
    let mut value = 0u16;
    let ret = unsafe { pcie_capability_read_word(dev, pos, &mut value) };
    if ret != PCIBIOS_SUCCESSFUL {
        return ret;
    }
    value &= !clear;
    value |= set;
    unsafe { pcie_capability_write_word(dev, pos, value) }
}

/// `pcie_capability_clear_and_set_word_locked` - `vendor/linux/drivers/pci/access.c:530`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcie_capability_clear_and_set_word_locked(
    dev: *const c_void,
    pos: i32,
    clear: u16,
    set: u16,
) -> i32 {
    unsafe { pcie_capability_clear_and_set_word_unlocked(dev, pos, clear, set) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::pci::device::{
        LinuxPciDeviceAbiState, register_linux_pci_device_state, unregister_linux_pci_device_state,
    };

    #[test]
    fn pci_config_exports_module_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("pci_read_config_byte"),
            Some(pci_read_config_byte as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_bus_read_config_byte"),
            Some(pci_bus_read_config_byte as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_read_config_dword"),
            Some(pci_read_config_dword as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_write_config_dword"),
            Some(pci_write_config_dword as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pcie_capability_read_word"),
            Some(pcie_capability_read_word as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pcie_capability_write_word"),
            Some(pcie_capability_write_word as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pcie_capability_clear_and_set_word_locked"),
            Some(pcie_capability_clear_and_set_word_locked as usize)
        );
    }

    #[test]
    fn pci_config_reads_and_writes_registered_raw_device_state() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; crate::linux_driver_abi::pci::device::PCI_CONFIG_SPACE_SIZE],
            bars: [None; crate::linux_driver_abi::pci::device::PCI_STD_NUM_BARS],
        };
        state.config_space[0x10..0x14].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        register_linux_pci_device_state(dev, state);

        let mut dword = 0;
        let mut byte = 0;
        unsafe {
            assert_eq!(pci_read_config_dword(dev, 0x10, &mut dword), 0);
            assert_eq!(pci_read_config_byte(dev, 0x11, &mut byte), 0);
        }
        assert_eq!(dword, 0x1234_5678);
        assert_eq!(byte, 0x56);
        unsafe {
            assert_eq!(pci_write_config_word(dev, 0x12, 0xabcd), 0);
            assert_eq!(pci_read_config_dword(dev, 0x10, &mut dword), 0);
        }
        assert_eq!(dword, 0xabcd_5678);

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pcie_capability_word_access_uses_capability_offset() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/pci/access.c"
        ));
        for export in [
            "EXPORT_SYMBOL(pcie_capability_read_word);",
            "EXPORT_SYMBOL(pcie_capability_write_word);",
            "EXPORT_SYMBOL(pcie_capability_clear_and_set_word_unlocked);",
            "EXPORT_SYMBOL(pcie_capability_clear_and_set_word_locked);",
        ] {
            assert!(source.contains(export));
        }

        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; crate::linux_driver_abi::pci::device::PCI_CONFIG_SPACE_SIZE],
            bars: [None; crate::linux_driver_abi::pci::device::PCI_STD_NUM_BARS],
        };
        state.config_space[0x06..0x08].copy_from_slice(&0x0010u16.to_le_bytes());
        state.config_space[0x34] = 0x40;
        state.config_space[0x40] = PCI_CAP_ID_EXP as u8;
        state.config_space[0x42..0x44].copy_from_slice(&0x0002u16.to_le_bytes());
        state.config_space[0x40 + PCI_EXP_DEVCTL as usize..0x42 + PCI_EXP_DEVCTL as usize]
            .copy_from_slice(&0x00f0u16.to_le_bytes());
        register_linux_pci_device_state(dev, state);

        let mut value = u16::MAX;
        unsafe {
            assert_eq!(
                pcie_capability_read_word(dev, PCI_EXP_DEVCTL, &mut value),
                PCIBIOS_SUCCESSFUL
            );
        }
        assert_eq!(value, 0x00f0);
        unsafe {
            assert_eq!(
                pcie_capability_clear_and_set_word_locked(dev, PCI_EXP_DEVCTL, 0x00f0, 0x0005),
                PCIBIOS_SUCCESSFUL
            );
            assert_eq!(
                pcie_capability_read_word(dev, PCI_EXP_DEVCTL, &mut value),
                PCIBIOS_SUCCESSFUL
            );
        }
        assert_eq!(value, 0x0005);
        unsafe {
            assert_eq!(
                pcie_capability_read_word(dev, PCI_EXP_DEVCTL + 1, &mut value),
                PCIBIOS_BAD_REGISTER_NUMBER
            );
        }

        unregister_linux_pci_device_state(dev);
    }
}
