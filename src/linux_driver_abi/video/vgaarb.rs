//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/vgaarb.c
//! VGA arbitration ABI used by native PCI graphics drivers.
//!
//! The state machine below follows Linux's resource ownership, decode, and
//! nested-lock accounting. Lupos's current PCI model has no parent-bridge
//! topology, so cross-bridge VGA forwarding and blocking wait queues remain
//! outside this translation; the flat generic x86 PCI case is preserved.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, ENODEV};
use crate::linux_driver_abi::pci::device::{
    IORESOURCE_MEM, LinuxPciDev, linux_pci_config_read, linux_pci_config_write,
    registered_linux_pci_raw_devices,
};

pub const VGA_RSRC_NONE: u32 = 0x00;
pub const VGA_RSRC_LEGACY_IO: u32 = 0x01;
pub const VGA_RSRC_LEGACY_MEM: u32 = 0x02;
pub const VGA_RSRC_LEGACY_MASK: u32 = VGA_RSRC_LEGACY_IO | VGA_RSRC_LEGACY_MEM;
pub const VGA_RSRC_NORMAL_IO: u32 = 0x04;
pub const VGA_RSRC_NORMAL_MEM: u32 = 0x08;

const PCI_COMMAND: usize = 0x04;
const PCI_COMMAND_IO: u32 = 0x01;
const PCI_COMMAND_MEMORY: u32 = 0x02;

type SetDecodeFn = unsafe extern "C" fn(*mut LinuxPciDev, bool) -> u32;

#[derive(Clone, Copy)]
struct VgaDevice {
    pdev: usize,
    decodes: u32,
    owns: u32,
    locks: u32,
    io_lock_cnt: u32,
    mem_lock_cnt: u32,
    io_norm_cnt: u32,
    mem_norm_cnt: u32,
    set_decode: usize,
}

#[derive(Default)]
struct VgaArbiter {
    devices: Vec<VgaDevice>,
    default: usize,
    used: bool,
}

lazy_static! {
    static ref VGA_ARBITER: Mutex<VgaArbiter> = Mutex::new(VgaArbiter::default());
}

fn pci_is_vga(pdev: *const LinuxPciDev) -> bool {
    !pdev.is_null() && unsafe { (*pdev).class >> 8 == 0x0300 }
}

fn contains_firmware_framebuffer(pdev: *const LinuxPciDev) -> bool {
    let Some((base, size)) = crate::arch::x86::video::primary_display_resource() else {
        return false;
    };
    let Some(end) = base.checked_add(size - 1) else {
        return false;
    };
    unsafe {
        (*pdev).resource.iter().take(6).any(|resource| {
            resource.flags & IORESOURCE_MEM != 0
                && resource.start != 0
                && resource.start <= base
                && resource.end >= end
        })
    }
}

fn initial_owns(pdev: *const LinuxPciDev) -> u32 {
    let command = linux_pci_config_read(pdev.cast::<c_void>(), PCI_COMMAND, 2).unwrap_or(0);
    let mut owns = 0;
    if command & PCI_COMMAND_IO != 0 {
        owns |= VGA_RSRC_LEGACY_IO;
    }
    if command & PCI_COMMAND_MEMORY != 0 {
        owns |= VGA_RSRC_LEGACY_MEM;
    }
    owns
}

fn sync_devices(arbiter: &mut VgaArbiter) {
    for pdev in registered_linux_pci_raw_devices() {
        if !pci_is_vga(pdev)
            || arbiter
                .devices
                .iter()
                .any(|device| device.pdev == pdev as usize)
        {
            continue;
        }
        let owns = initial_owns(pdev);
        arbiter.devices.push(VgaDevice {
            pdev: pdev as usize,
            decodes: VGA_RSRC_LEGACY_MASK | VGA_RSRC_NORMAL_IO | VGA_RSRC_NORMAL_MEM,
            owns,
            locks: 0,
            io_lock_cnt: 0,
            mem_lock_cnt: 0,
            io_norm_cnt: 0,
            mem_norm_cnt: 0,
            set_decode: 0,
        });

        let is_firmware_default = contains_firmware_framebuffer(pdev);
        if is_firmware_default || arbiter.default == 0 {
            arbiter.default = pdev as usize;
        }
    }
}

fn update_decodes(device: &mut VgaDevice, new_decodes: u32) {
    let new_decodes =
        new_decodes & (VGA_RSRC_LEGACY_MASK | VGA_RSRC_NORMAL_IO | VGA_RSRC_NORMAL_MEM);
    let removed_locked = device.locks & !new_decodes;
    device.decodes = new_decodes;
    if removed_locked & VGA_RSRC_LEGACY_IO != 0 {
        device.io_lock_cnt = 0;
        device.locks &= !VGA_RSRC_LEGACY_IO;
    }
    if removed_locked & VGA_RSRC_LEGACY_MEM != 0 {
        device.mem_lock_cnt = 0;
        device.locks &= !VGA_RSRC_LEGACY_MEM;
    }
}

fn notify_clients_first_use(arbiter: &mut VgaArbiter) {
    if arbiter.used {
        return;
    }
    arbiter.used = true;
    let enable_decode = arbiter.devices.len() <= 1;
    for device in &mut arbiter.devices {
        if device.set_decode == 0 {
            continue;
        }
        let callback: SetDecodeFn = unsafe { core::mem::transmute(device.set_decode) };
        let decodes = unsafe { callback(device.pdev as *mut LinuxPciDev, enable_decode) };
        update_decodes(device, decodes);
    }
}

fn set_command_bits(pdev: *const LinuxPciDev, bits: u32, enable: bool) -> bool {
    let Some(mut command) = linux_pci_config_read(pdev.cast::<c_void>(), PCI_COMMAND, 2) else {
        return false;
    };
    if enable {
        command |= bits;
    } else {
        command &= !bits;
    }
    linux_pci_config_write(pdev.cast::<c_void>(), PCI_COMMAND, 2, command)
}

/// Linux `vga_default_device()`.
#[unsafe(no_mangle)]
pub extern "C" fn vga_default_device() -> *mut LinuxPciDev {
    let mut arbiter = VGA_ARBITER.lock();
    sync_devices(&mut arbiter);
    arbiter.default as *mut LinuxPciDev
}

/// Linux `vga_remove_vgacon()` for the registered x86 VGA console.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vga_remove_vgacon(pdev: *mut LinuxPciDev) -> i32 {
    if pdev == vga_default_device() {
        crate::linux_driver_abi::video::console::vgacon::detach();
    }
    0
}

/// Linux `vga_client_register()`; a null callback unregisters the client.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vga_client_register(
    pdev: *mut LinuxPciDev,
    set_decode: *const c_void,
) -> i32 {
    let mut arbiter = VGA_ARBITER.lock();
    sync_devices(&mut arbiter);
    let Some(device) = arbiter
        .devices
        .iter_mut()
        .find(|device| device.pdev == pdev as usize)
    else {
        return -ENODEV;
    };
    device.set_decode = set_decode as usize;
    0
}

/// Linux `vga_set_legacy_decoding()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vga_set_legacy_decoding(pdev: *mut LinuxPciDev, decodes: u32) {
    let mut arbiter = VGA_ARBITER.lock();
    sync_devices(&mut arbiter);
    if let Some(device) = arbiter
        .devices
        .iter_mut()
        .find(|device| device.pdev == pdev as usize)
    {
        // `__vga_set_legacy_decoding()` masks the caller's value before
        // replacing `vgadev->decodes`; it does not preserve the initial
        // normal-resource bits.
        update_decodes(device, decodes & VGA_RSRC_LEGACY_MASK);
    }
}

/// Linux `vga_get()` for the flat PCI topology used by the generic x86 target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vga_get(
    mut pdev: *mut LinuxPciDev,
    mut rsrc: u32,
    _interruptible: i32,
) -> i32 {
    let mut arbiter = VGA_ARBITER.lock();
    sync_devices(&mut arbiter);
    notify_clients_first_use(&mut arbiter);
    if pdev.is_null() {
        pdev = arbiter.default as *mut LinuxPciDev;
    }
    if pdev.is_null() {
        return 0;
    }
    let Some(target_index) = arbiter
        .devices
        .iter()
        .position(|device| device.pdev == pdev as usize)
    else {
        return -ENODEV;
    };

    let target_decodes = arbiter.devices[target_index].decodes;
    if rsrc & VGA_RSRC_NORMAL_IO != 0 && target_decodes & VGA_RSRC_LEGACY_IO != 0 {
        rsrc |= VGA_RSRC_LEGACY_IO;
    }
    if rsrc & VGA_RSRC_NORMAL_MEM != 0 && target_decodes & VGA_RSRC_LEGACY_MEM != 0 {
        rsrc |= VGA_RSRC_LEGACY_MEM;
    }

    let wants = rsrc & !arbiter.devices[target_index].owns;
    let legacy_wants = wants & VGA_RSRC_LEGACY_MASK;
    for index in 0..arbiter.devices.len() {
        if index == target_index {
            continue;
        }
        let conflict = &arbiter.devices[index];
        if conflict.locks & legacy_wants != 0 {
            return -EBUSY;
        }
    }

    for index in 0..arbiter.devices.len() {
        if index == target_index {
            continue;
        }
        let conflict = &mut arbiter.devices[index];
        let stolen = conflict.owns & legacy_wants;
        if stolen == 0 {
            continue;
        }
        let mut command_bits = 0;
        if stolen & conflict.decodes & VGA_RSRC_LEGACY_IO != 0 {
            command_bits |= PCI_COMMAND_IO;
        }
        if stolen & conflict.decodes & VGA_RSRC_LEGACY_MEM != 0 {
            command_bits |= PCI_COMMAND_MEMORY;
        }
        if command_bits != 0
            && !set_command_bits(conflict.pdev as *const LinuxPciDev, command_bits, false)
        {
            return -ENODEV;
        }
        conflict.owns &= !stolen;
    }

    let mut command_bits = 0;
    if wants & (VGA_RSRC_LEGACY_IO | VGA_RSRC_NORMAL_IO) != 0 {
        command_bits |= PCI_COMMAND_IO;
    }
    if wants & (VGA_RSRC_LEGACY_MEM | VGA_RSRC_NORMAL_MEM) != 0 {
        command_bits |= PCI_COMMAND_MEMORY;
    }
    if command_bits != 0 && !set_command_bits(pdev, command_bits, true) {
        return -ENODEV;
    }

    let device = &mut arbiter.devices[target_index];
    device.owns |= wants;
    device.locks |= rsrc & VGA_RSRC_LEGACY_MASK;
    if rsrc & VGA_RSRC_LEGACY_IO != 0 {
        device.io_lock_cnt += 1;
    }
    if rsrc & VGA_RSRC_LEGACY_MEM != 0 {
        device.mem_lock_cnt += 1;
    }
    if rsrc & VGA_RSRC_NORMAL_IO != 0 {
        device.io_norm_cnt += 1;
    }
    if rsrc & VGA_RSRC_NORMAL_MEM != 0 {
        device.mem_norm_cnt += 1;
    }
    0
}

/// Linux `vga_put()` nested-resource accounting.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vga_put(mut pdev: *mut LinuxPciDev, mut rsrc: u32) {
    let mut arbiter = VGA_ARBITER.lock();
    sync_devices(&mut arbiter);
    if pdev.is_null() {
        pdev = arbiter.default as *mut LinuxPciDev;
    }
    let Some(device) = arbiter
        .devices
        .iter_mut()
        .find(|device| device.pdev == pdev as usize)
    else {
        return;
    };

    if rsrc & VGA_RSRC_NORMAL_IO != 0 && device.io_norm_cnt > 0 {
        device.io_norm_cnt -= 1;
        if device.decodes & VGA_RSRC_LEGACY_IO != 0 {
            rsrc |= VGA_RSRC_LEGACY_IO;
        }
    }
    if rsrc & VGA_RSRC_NORMAL_MEM != 0 && device.mem_norm_cnt > 0 {
        device.mem_norm_cnt -= 1;
        if device.decodes & VGA_RSRC_LEGACY_MEM != 0 {
            rsrc |= VGA_RSRC_LEGACY_MEM;
        }
    }
    if rsrc & VGA_RSRC_LEGACY_IO != 0 && device.io_lock_cnt > 0 {
        device.io_lock_cnt -= 1;
    }
    if rsrc & VGA_RSRC_LEGACY_MEM != 0 && device.mem_lock_cnt > 0 {
        device.mem_lock_cnt -= 1;
    }
    if device.io_lock_cnt == 0 {
        device.locks &= !VGA_RSRC_LEGACY_IO;
    }
    if device.mem_lock_cnt == 0 {
        device.locks &= !VGA_RSRC_LEGACY_MEM;
    }
}
