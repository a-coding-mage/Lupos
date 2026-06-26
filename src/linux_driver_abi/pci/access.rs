//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/access.c
//! test-origin: linux:vendor/linux/drivers/pci/access.c
//! PCI config-space access helpers for Linux-built PCI modules.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::pci::device::linux_pci_config_read;

pub const PCIBIOS_SUCCESSFUL: i32 = 0x00;
pub const PCIBIOS_DEVICE_NOT_FOUND: i32 = 0x86;
pub const PCIBIOS_BAD_REGISTER_NUMBER: i32 = 0x87;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("pci_read_config_byte", pci_read_config_byte as usize, false);
    export_symbol_once("pci_read_config_word", pci_read_config_word as usize, false);
    export_symbol_once(
        "pci_read_config_dword",
        pci_read_config_dword as usize,
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
            crate::kernel::module::find_symbol("pci_read_config_dword"),
            Some(pci_read_config_dword as usize)
        );
    }

    #[test]
    fn pci_config_reads_registered_raw_device_state() {
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

        unregister_linux_pci_device_state(dev);
    }
}
