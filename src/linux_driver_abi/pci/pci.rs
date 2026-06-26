//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/pci.c
//! test-origin: linux:vendor/linux/drivers/pci/pci.c
//! Generic PCI helper exports used by Linux-built PCI drivers.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::pci::device::{
    IORESOURCE_IO, IORESOURCE_MEM, PCI_STD_NUM_BARS, linux_pci_bar_resource, linux_pci_config_read,
    linux_pci_config_write, linux_pci_device_state,
};

const PCI_COMMAND: usize = 0x04;
const PCI_COMMAND_IO: u16 = 0x1;
const PCI_COMMAND_MEMORY: u16 = 0x2;
const PCI_COMMAND_MASTER: u16 = 0x4;
const PCI_STATUS: usize = 0x06;
const PCI_STATUS_CAP_LIST: u16 = 0x10;
const PCI_CAPABILITY_LIST: usize = 0x34;
const PCI_CAP_LIST_NEXT: usize = 1;
const PCI_CAP_MIN: u8 = 0x40;
const PCI_FIND_CAP_TTL: usize = 48;
const PCI_CFG_SPACE_SIZE: u16 = 256;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("pci_find_capability", pci_find_capability as usize, false);
    export_symbol_once(
        "pci_find_next_capability",
        pci_find_next_capability as usize,
        true,
    );
    export_symbol_once(
        "pci_find_next_ext_capability",
        pci_find_next_ext_capability as usize,
        true,
    );
    export_symbol_once(
        "pci_find_ext_capability",
        pci_find_ext_capability as usize,
        true,
    );
    export_symbol_once("pci_enable_device", pci_enable_device as usize, false);
    export_symbol_once(
        "pci_enable_device_mem",
        pci_enable_device_mem as usize,
        false,
    );
    export_symbol_once("pci_disable_device", pci_disable_device as usize, false);
    export_symbol_once("pci_set_master", pci_set_master as usize, false);
    export_symbol_once("pci_set_mwi", pci_set_mwi as usize, false);
    export_symbol_once("pci_clear_mwi", pci_clear_mwi as usize, false);
    export_symbol_once("pcix_get_mmrbc", pcix_get_mmrbc as usize, false);
    export_symbol_once("pcix_set_mmrbc", pcix_set_mmrbc as usize, false);
    export_symbol_once("pci_select_bars", pci_select_bars as usize, false);
    export_symbol_once("pci_save_state", pci_save_state as usize, false);
    export_symbol_once("pci_set_power_state", pci_set_power_state as usize, false);
    export_symbol_once("pci_enable_wake", pci_enable_wake as usize, false);
    export_symbol_once("pci_wake_from_d3", pci_wake_from_d3 as usize, false);
    export_symbol_once(
        "pci_device_is_present",
        pci_device_is_present as usize,
        true,
    );
    export_symbol_once(
        "pci_request_selected_regions",
        pci_request_selected_regions as usize,
        false,
    );
    export_symbol_once(
        "pci_release_selected_regions",
        pci_release_selected_regions as usize,
        false,
    );
}

fn find_next_capability_in_config(config: &[u8], mut pos: u8, cap: i32) -> u8 {
    for _ in 0..PCI_FIND_CAP_TTL {
        pos &= !0x3;
        let offset = pos as usize;
        if pos < PCI_CAP_MIN || offset + PCI_CAP_LIST_NEXT >= config.len() {
            break;
        }
        if config[offset] == cap as u8 {
            return pos;
        }
        pos = config[offset + PCI_CAP_LIST_NEXT];
    }
    0
}

/// `pci_find_next_capability` - `vendor/linux/drivers/pci/pci.c:429`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_next_capability(dev: *mut c_void, pos: u8, cap: i32) -> u8 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return 0;
    };
    let next = pos
        .checked_add(PCI_CAP_LIST_NEXT as u8)
        .and_then(|offset| state.config_space.get(offset as usize).copied())
        .unwrap_or(0);
    find_next_capability_in_config(&state.config_space, next, cap)
}

/// `pci_find_capability` - `vendor/linux/drivers/pci/pci.c:475`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_capability(dev: *mut c_void, cap: i32) -> u8 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return 0;
    };
    let status = u16::from_le_bytes([
        state.config_space[PCI_STATUS],
        state.config_space[PCI_STATUS + 1],
    ]);
    if status & PCI_STATUS_CAP_LIST == 0 {
        return 0;
    }
    let pos = state.config_space[PCI_CAPABILITY_LIST];
    find_next_capability_in_config(&state.config_space, pos, cap)
}

/// `pci_find_next_ext_capability` - `vendor/linux/drivers/pci/pci.c:525`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_next_ext_capability(
    dev: *mut c_void,
    start: u16,
    _cap: i32,
) -> u16 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return 0;
    };
    if state.config_space.len() <= PCI_CFG_SPACE_SIZE as usize {
        return 0;
    }
    if start < PCI_CFG_SPACE_SIZE {
        PCI_CFG_SPACE_SIZE
    } else {
        0
    }
}

/// `pci_find_ext_capability` - `vendor/linux/drivers/pci/pci.c:549`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_ext_capability(dev: *mut c_void, cap: i32) -> u16 {
    unsafe { pci_find_next_ext_capability(dev, 0, cap) }
}

/// `pci_enable_device` - `vendor/linux/drivers/pci/pci.c:2122`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_enable_device(dev: *mut c_void) -> i32 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return -EINVAL;
    };

    let mut enable = 0u16;
    for resource in state.bars.iter().flatten() {
        if resource.flags & IORESOURCE_IO != 0 {
            enable |= PCI_COMMAND_IO;
        }
        if resource.flags & IORESOURCE_MEM != 0 {
            enable |= PCI_COMMAND_MEMORY;
        }
    }
    if enable == 0 {
        return 0;
    }

    update_command_bits(dev.cast_const(), enable)
}

/// `pci_enable_device_mem` - `vendor/linux/drivers/pci/pci.c:2105`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_enable_device_mem(dev: *mut c_void) -> i32 {
    unsafe { pci_enable_device(dev) }
}

/// `pci_disable_device` - `vendor/linux/drivers/pci/pci.c:2165`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_disable_device(_dev: *mut c_void) {}

/// `pci_set_master` - `vendor/linux/drivers/pci/pci.c:4140`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_set_master(dev: *mut c_void) {
    let _ = update_command_bits(dev.cast_const(), PCI_COMMAND_MASTER);
}

/// `pci_set_mwi` - `vendor/linux/drivers/pci/pci.c:4203`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_set_mwi(_dev: *mut c_void) -> i32 {
    0
}

/// `pci_clear_mwi` - `vendor/linux/drivers/pci/pci.c:4251`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_clear_mwi(_dev: *mut c_void) {}

/// `pcix_get_mmrbc` - `vendor/linux/drivers/pci/pci.c:5719`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcix_get_mmrbc(_dev: *mut c_void) -> i32 {
    512
}

/// `pcix_set_mmrbc` - `vendor/linux/drivers/pci/pci.c:5744`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcix_set_mmrbc(_dev: *mut c_void, _mmrbc: i32) -> i32 {
    0
}

/// `pci_select_bars` - `vendor/linux/drivers/pci/pci.c:6130`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_select_bars(dev: *mut c_void, flags: usize) -> i32 {
    let mut bars = 0i32;
    for bar in 0..PCI_STD_NUM_BARS {
        if let Some(resource) = linux_pci_bar_resource(dev.cast_const(), bar) {
            if resource.flags & flags != 0 {
                bars |= 1 << bar;
            }
        }
    }
    bars
}

/// `pci_save_state` - `vendor/linux/drivers/pci/pci.c:1739`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_save_state(_dev: *mut c_void) -> i32 {
    0
}

/// `pci_set_power_state` - `vendor/linux/drivers/pci/pci.c:1596`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_set_power_state(_dev: *mut c_void, _state: i32) -> i32 {
    0
}

/// `pci_enable_wake` - `vendor/linux/drivers/pci/pci.c:2574`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_enable_wake(_dev: *mut c_void, _state: i32, _enable: bool) -> i32 {
    0
}

/// `pci_wake_from_d3` - `vendor/linux/drivers/pci/pci.c:2597`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_wake_from_d3(_dev: *mut c_void, _enable: bool) -> i32 {
    0
}

fn update_command_bits(dev: *const c_void, bits: u16) -> i32 {
    let Some(command) = linux_pci_config_read(dev, PCI_COMMAND, 2) else {
        return -EINVAL;
    };
    let command = (command as u16) | bits;
    if linux_pci_config_write(dev, PCI_COMMAND, 2, command as u32) {
        0
    } else {
        -EINVAL
    }
}

/// `pci_device_is_present` - `vendor/linux/drivers/pci/pci.c:6296`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_device_is_present(dev: *mut c_void) -> bool {
    linux_pci_device_state(dev.cast_const()).is_some()
}

/// `pci_request_selected_regions` - `vendor/linux/drivers/pci/pci.c:3884`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_request_selected_regions(
    dev: *mut c_void,
    _bars: i32,
    _name: *const c_char,
) -> i32 {
    if linux_pci_device_state(dev.cast_const()).is_some() {
        0
    } else {
        -EINVAL
    }
}

/// `pci_release_selected_regions` - `vendor/linux/drivers/pci/pci.c:3846`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_release_selected_regions(_dev: *mut c_void, _bars: i32) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::pci::device::{
        IORESOURCE_MEM, LinuxPciBarResource, LinuxPciDeviceAbiState, PCI_CONFIG_SPACE_SIZE,
        PCI_STD_NUM_BARS, register_linux_pci_device_state, unregister_linux_pci_device_state,
    };

    #[test]
    fn pci_helper_exports_module_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("pci_find_capability"),
            Some(pci_find_capability as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_request_selected_regions"),
            Some(pci_request_selected_regions as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_find_ext_capability"),
            Some(pci_find_ext_capability as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_enable_device"),
            Some(pci_enable_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_device_is_present"),
            Some(pci_device_is_present as usize)
        );
    }

    #[test]
    fn pci_capability_walk_matches_standard_list_layout() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        state.config_space[PCI_STATUS..PCI_STATUS + 2]
            .copy_from_slice(&PCI_STATUS_CAP_LIST.to_le_bytes());
        state.config_space[PCI_CAPABILITY_LIST] = 0x40;
        state.config_space[0x40] = 0x09;
        state.config_space[0x41] = 0x50;
        state.config_space[0x50] = 0x05;
        state.config_space[0x51] = 0;
        register_linux_pci_device_state(dev, state);

        unsafe {
            assert_eq!(pci_find_capability(dev, 0x09), 0x40);
            assert_eq!(pci_find_next_capability(dev, 0x40, 0x05), 0x50);
            assert_eq!(pci_find_ext_capability(dev, 0x10), 0);
            assert_eq!(pci_enable_device(dev), 0);
            assert!(pci_device_is_present(dev));
            pci_set_master(dev);
            pci_disable_device(dev);
            assert_eq!(pci_find_capability(dev, 0xff), 0);
            assert_eq!(
                pci_request_selected_regions(dev, 0x3f, core::ptr::null()),
                0
            );
        }

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pci_enable_device_and_set_master_update_command_bits() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        state.bars[5] = Some(LinuxPciBarResource {
            start: 0xfebd_0000,
            len: 0x1000,
            flags: IORESOURCE_MEM,
        });
        register_linux_pci_device_state(dev, state);

        unsafe {
            assert_eq!(pci_enable_device(dev), 0);
            assert_eq!(
                linux_pci_config_read(dev, PCI_COMMAND, 2),
                Some(PCI_COMMAND_MEMORY as u32)
            );
            pci_set_master(dev);
            assert_eq!(
                linux_pci_config_read(dev, PCI_COMMAND, 2),
                Some((PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER) as u32)
            );
        }

        unregister_linux_pci_device_state(dev);
    }
}
