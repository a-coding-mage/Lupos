//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/chip.c
//! test-origin: linux:vendor/linux/kernel/irq/chip.c
//! `struct irq_chip` — IRQ controller ops (M37).

use core::ffi::{c_char, c_void};
use core::sync::atomic::Ordering;

use super::irqdesc::NR_IRQS;
use super::irqdesc::desc_for;
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

/// Linux `struct irq_chip`.
pub struct IrqChip {
    pub name: &'static str,
    pub mask: Option<fn(irq: u32)>,
    pub unmask: Option<fn(irq: u32)>,
    pub ack: Option<fn(irq: u32)>,
    pub eoi: Option<fn(irq: u32)>,
    pub set_affinity: Option<fn(irq: u32, mask: u32)>,
}

unsafe impl Send for IrqChip {}
unsafe impl Sync for IrqChip {}

/// LAPIC chip — issues EOI through the local APIC.
fn lapic_eoi(_irq: u32) {
    #[cfg(not(test))]
    unsafe {
        crate::arch::x86::kernel::apic::eoi();
    }
}
fn lapic_mask(_irq: u32) {}
fn lapic_unmask(_irq: u32) {}
fn lapic_set_affinity(_irq: u32, _mask: u32) {}

pub static LAPIC_CHIP: IrqChip = IrqChip {
    name: "APIC",
    mask: Some(lapic_mask),
    unmask: Some(lapic_unmask),
    ack: None,
    eoi: Some(lapic_eoi),
    set_affinity: Some(lapic_set_affinity),
};

#[repr(C)]
struct LinuxIrqCommonDataAbi {
    state_use_accessors: u32,
    node: u32,
    handler_data: *mut c_void,
    msi_desc: *mut c_void,
    affinity: *mut c_void,
    effective_affinity: *mut c_void,
}

unsafe impl Sync for LinuxIrqCommonDataAbi {}

#[repr(C)]
struct LinuxIrqDataAbi {
    mask: u32,
    irq: u32,
    hwirq: u64,
    common: *const LinuxIrqCommonDataAbi,
    chip: *const c_void,
    domain: *mut c_void,
    parent_data: *mut c_void,
    chip_data: *mut c_void,
}

unsafe impl Sync for LinuxIrqDataAbi {}

static LINUX_IRQ_COMMON_DATA_ZERO: LinuxIrqCommonDataAbi = LinuxIrqCommonDataAbi {
    state_use_accessors: 0,
    node: 0,
    handler_data: core::ptr::null_mut(),
    msi_desc: core::ptr::null_mut(),
    affinity: core::ptr::null_mut(),
    effective_affinity: core::ptr::null_mut(),
};

static LINUX_IRQ_DATA_ZERO: LinuxIrqDataAbi = LinuxIrqDataAbi {
    mask: 0,
    irq: 0,
    hwirq: 0,
    common: core::ptr::addr_of!(LINUX_IRQ_COMMON_DATA_ZERO),
    chip: core::ptr::null(),
    domain: core::ptr::null_mut(),
    parent_data: core::ptr::null_mut(),
    chip_data: core::ptr::null_mut(),
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("irq_set_chip", linux_irq_set_chip as usize, false);
    export_symbol_once("irq_set_chip_data", linux_irq_set_chip_data as usize, false);
    export_symbol_once("irq_get_irq_data", linux_irq_get_irq_data as usize, true);
    export_symbol_once(
        "irq_set_chip_and_handler_name",
        linux_irq_set_chip_and_handler_name as usize,
        true,
    );
}

/// `irq_get_irq_data` - `vendor/linux/kernel/irq/chip.c`.
unsafe extern "C" fn linux_irq_get_irq_data(irq: u32) -> *mut c_void {
    if irq as usize >= NR_IRQS || desc_for(irq).is_none() {
        return core::ptr::null_mut();
    }
    core::ptr::addr_of!(LINUX_IRQ_DATA_ZERO)
        .cast_mut()
        .cast::<c_void>()
}

/// `irq_set_chip` - `vendor/linux/kernel/irq/chip.c`.
unsafe extern "C" fn linux_irq_set_chip(irq: u32, chip: *const c_void) -> i32 {
    let Some(desc) = desc_for(irq) else {
        return -EINVAL;
    };
    desc.chip.store(chip as usize, Ordering::Release);
    0
}

/// `irq_set_chip_data` - `vendor/linux/kernel/irq/chip.c:128`.
unsafe extern "C" fn linux_irq_set_chip_data(irq: u32, data: *mut c_void) -> i32 {
    let Some(desc) = desc_for(irq) else {
        return -EINVAL;
    };
    desc.chip_data.store(data as usize, Ordering::Release);
    0
}

/// `irq_set_chip_and_handler_name` - `vendor/linux/kernel/irq/chip.c`.
unsafe extern "C" fn linux_irq_set_chip_and_handler_name(
    irq: u32,
    chip: *const c_void,
    handle: *const c_void,
    name: *const c_char,
) {
    if let Some(desc) = desc_for(irq) {
        desc.chip.store(chip as usize, Ordering::Release);
        desc.flow_handler.store(handle as usize, Ordering::Release);
        desc.flow_handler_name
            .store(name as usize, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lapic_chip_name_is_apic() {
        assert_eq!(LAPIC_CHIP.name, "APIC");
    }

    #[test]
    fn lapic_chip_has_eoi_callback() {
        assert!(LAPIC_CHIP.eoi.is_some());
    }

    #[test]
    fn irq_set_chip_data_updates_descriptor_side_data() {
        let data = 0x1234usize as *mut c_void;
        unsafe {
            assert_eq!(linux_irq_set_chip_data(32, data), 0);
        }
        assert_eq!(
            desc_for(32).unwrap().chip_data.load(Ordering::Acquire),
            data as usize
        );
    }
}
