//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/dummychip.c
//! test-origin: linux:vendor/linux/kernel/irq/dummychip.c
//! Generic no-controller and dummy IRQ chip implementations.
//!
//! Mirrors `vendor/linux/kernel/irq/dummychip.c`.

use core::ffi::{c_char, c_void};

use super::chip::IrqChip;
use crate::kernel::module::{export_symbol, find_symbol};

pub const IRQCHIP_SKIP_SET_WAKE: u32 = 1 << 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DummyIrqAction {
    Noop,
    AckBad { irq: u32 },
    StartupReturn(u32),
}

#[derive(Clone, Copy)]
pub struct LinuxDummyIrqChip {
    pub name: &'static str,
    pub irq_startup: Option<fn(u32) -> DummyIrqAction>,
    pub irq_shutdown: Option<fn(u32) -> DummyIrqAction>,
    pub irq_enable: Option<fn(u32) -> DummyIrqAction>,
    pub irq_disable: Option<fn(u32) -> DummyIrqAction>,
    pub irq_ack: Option<fn(u32) -> DummyIrqAction>,
    pub irq_mask: Option<fn(u32) -> DummyIrqAction>,
    pub irq_unmask: Option<fn(u32) -> DummyIrqAction>,
    pub flags: u32,
}

unsafe impl Send for LinuxDummyIrqChip {}
unsafe impl Sync for LinuxDummyIrqChip {}

pub fn ack_bad(irq: u32) -> DummyIrqAction {
    DummyIrqAction::AckBad { irq }
}

pub fn noop(_irq: u32) -> DummyIrqAction {
    DummyIrqAction::Noop
}

pub fn noop_ret(_irq: u32) -> DummyIrqAction {
    DummyIrqAction::StartupReturn(0)
}

pub static NO_IRQ_CHIP: LinuxDummyIrqChip = LinuxDummyIrqChip {
    name: "none",
    irq_startup: Some(noop_ret),
    irq_shutdown: Some(noop),
    irq_enable: Some(noop),
    irq_disable: Some(noop),
    irq_ack: Some(ack_bad),
    irq_mask: None,
    irq_unmask: None,
    flags: IRQCHIP_SKIP_SET_WAKE,
};

pub static DUMMY_IRQ_CHIP_MODEL: LinuxDummyIrqChip = LinuxDummyIrqChip {
    name: "dummy",
    irq_startup: Some(noop_ret),
    irq_shutdown: Some(noop),
    irq_enable: Some(noop),
    irq_disable: Some(noop),
    irq_ack: Some(noop),
    irq_mask: Some(noop),
    irq_unmask: Some(noop),
    flags: IRQCHIP_SKIP_SET_WAKE,
};

fn irq_chip_noop(_irq: u32) {}

pub static DUMMY_IRQ_CHIP: IrqChip = IrqChip {
    name: "dummy",
    mask: Some(irq_chip_noop),
    unmask: Some(irq_chip_noop),
    ack: Some(irq_chip_noop),
    eoi: None,
    set_affinity: None,
};

type LinuxIrqData = c_void;
type LinuxCpumask = c_void;
type LinuxMsiMsg = c_void;
type LinuxSeqFile = c_void;

/// `struct irq_chip` - `vendor/linux/include/linux/irq.h:501`.
#[repr(C)]
pub struct LinuxIrqChipAbi {
    pub name: *const c_char,
    pub irq_startup: Option<unsafe extern "C" fn(*mut LinuxIrqData) -> u32>,
    pub irq_shutdown: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_enable: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_disable: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_ack: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_mask: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_mask_ack: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_unmask: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_eoi: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_set_affinity:
        Option<unsafe extern "C" fn(*mut LinuxIrqData, *const LinuxCpumask, bool) -> i32>,
    pub irq_pre_redirect: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_retrigger: Option<unsafe extern "C" fn(*mut LinuxIrqData) -> i32>,
    pub irq_set_type: Option<unsafe extern "C" fn(*mut LinuxIrqData, u32) -> i32>,
    pub irq_set_wake: Option<unsafe extern "C" fn(*mut LinuxIrqData, u32) -> i32>,
    pub irq_bus_lock: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_bus_sync_unlock: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_suspend: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_resume: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_pm_shutdown: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_calc_mask: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_print_chip: Option<unsafe extern "C" fn(*mut LinuxIrqData, *mut LinuxSeqFile)>,
    pub irq_request_resources: Option<unsafe extern "C" fn(*mut LinuxIrqData) -> i32>,
    pub irq_release_resources: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_compose_msi_msg: Option<unsafe extern "C" fn(*mut LinuxIrqData, *mut LinuxMsiMsg)>,
    pub irq_write_msi_msg: Option<unsafe extern "C" fn(*mut LinuxIrqData, *mut LinuxMsiMsg)>,
    pub irq_get_irqchip_state:
        Option<unsafe extern "C" fn(*mut LinuxIrqData, u32, *mut bool) -> i32>,
    pub irq_set_irqchip_state: Option<unsafe extern "C" fn(*mut LinuxIrqData, u32, bool) -> i32>,
    pub irq_set_vcpu_affinity: Option<unsafe extern "C" fn(*mut LinuxIrqData, *mut c_void) -> i32>,
    pub ipi_send_single: Option<unsafe extern "C" fn(*mut LinuxIrqData, u32)>,
    pub ipi_send_mask: Option<unsafe extern "C" fn(*mut LinuxIrqData, *const LinuxCpumask)>,
    pub irq_nmi_setup: Option<unsafe extern "C" fn(*mut LinuxIrqData) -> i32>,
    pub irq_nmi_teardown: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub irq_force_complete_move: Option<unsafe extern "C" fn(*mut LinuxIrqData)>,
    pub flags: usize,
}

unsafe impl Sync for LinuxIrqChipAbi {}

static DUMMY_IRQ_CHIP_NAME: [u8; 6] = *b"dummy\0";

unsafe extern "C" fn linux_irq_noop(_data: *mut LinuxIrqData) {}

unsafe extern "C" fn linux_irq_noop_ret(_data: *mut LinuxIrqData) -> u32 {
    0
}

pub static DUMMY_IRQ_CHIP_ABI: LinuxIrqChipAbi = LinuxIrqChipAbi {
    name: DUMMY_IRQ_CHIP_NAME.as_ptr().cast::<c_char>(),
    irq_startup: Some(linux_irq_noop_ret),
    irq_shutdown: Some(linux_irq_noop),
    irq_enable: Some(linux_irq_noop),
    irq_disable: Some(linux_irq_noop),
    irq_ack: Some(linux_irq_noop),
    irq_mask: Some(linux_irq_noop),
    irq_mask_ack: None,
    irq_unmask: Some(linux_irq_noop),
    irq_eoi: None,
    irq_set_affinity: None,
    irq_pre_redirect: None,
    irq_retrigger: None,
    irq_set_type: None,
    irq_set_wake: None,
    irq_bus_lock: None,
    irq_bus_sync_unlock: None,
    irq_suspend: None,
    irq_resume: None,
    irq_pm_shutdown: None,
    irq_calc_mask: None,
    irq_print_chip: None,
    irq_request_resources: None,
    irq_release_resources: None,
    irq_compose_msi_msg: None,
    irq_write_msi_msg: None,
    irq_get_irqchip_state: None,
    irq_set_irqchip_state: None,
    irq_set_vcpu_affinity: None,
    ipi_send_single: None,
    ipi_send_mask: None,
    irq_nmi_setup: None,
    irq_nmi_teardown: None,
    irq_force_complete_move: None,
    flags: IRQCHIP_SKIP_SET_WAKE as usize,
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "dummy_irq_chip",
        core::ptr::addr_of!(DUMMY_IRQ_CHIP_ABI) as usize,
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummychip_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/irq/dummychip.c"
        ));
        assert!(source.contains("static void ack_bad(struct irq_data *data)"));
        assert!(source.contains("print_irq_desc(data->irq, desc);"));
        assert!(source.contains("ack_bad_irq(data->irq);"));
        assert!(source.contains("static void noop(struct irq_data *data) { }"));
        assert!(source.contains("static unsigned int noop_ret(struct irq_data *data)"));
        assert!(source.contains("struct irq_chip no_irq_chip = {"));
        assert!(source.contains(".name\t\t= \"none\""));
        assert!(source.contains(".irq_ack\t= ack_bad"));
        assert!(source.contains("struct irq_chip dummy_irq_chip = {"));
        assert!(source.contains(".name\t\t= \"dummy\""));
        assert!(source.contains(".irq_mask\t= noop"));
        assert!(source.contains(".irq_unmask\t= noop"));
        assert!(source.contains(".flags\t\t= IRQCHIP_SKIP_SET_WAKE"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(dummy_irq_chip);"));
    }

    #[test]
    fn no_irq_chip_matches_linux_no_controller_shape() {
        assert_eq!(NO_IRQ_CHIP.name, "none");
        assert_eq!(NO_IRQ_CHIP.flags, IRQCHIP_SKIP_SET_WAKE);
        assert_eq!(
            (NO_IRQ_CHIP.irq_startup.unwrap())(9),
            DummyIrqAction::StartupReturn(0)
        );
        assert_eq!((NO_IRQ_CHIP.irq_shutdown.unwrap())(9), DummyIrqAction::Noop);
        assert_eq!((NO_IRQ_CHIP.irq_enable.unwrap())(9), DummyIrqAction::Noop);
        assert_eq!((NO_IRQ_CHIP.irq_disable.unwrap())(9), DummyIrqAction::Noop);
        assert_eq!(
            (NO_IRQ_CHIP.irq_ack.unwrap())(9),
            DummyIrqAction::AckBad { irq: 9 }
        );
        assert!(NO_IRQ_CHIP.irq_mask.is_none());
        assert!(NO_IRQ_CHIP.irq_unmask.is_none());
    }

    #[test]
    fn dummy_chip_model_matches_linux_exported_shape() {
        assert_eq!(DUMMY_IRQ_CHIP_MODEL.name, "dummy");
        assert_eq!(DUMMY_IRQ_CHIP_MODEL.flags, IRQCHIP_SKIP_SET_WAKE);
        assert_eq!(
            (DUMMY_IRQ_CHIP_MODEL.irq_startup.unwrap())(11),
            DummyIrqAction::StartupReturn(0)
        );
        assert_eq!(
            (DUMMY_IRQ_CHIP_MODEL.irq_shutdown.unwrap())(11),
            DummyIrqAction::Noop
        );
        assert_eq!(
            (DUMMY_IRQ_CHIP_MODEL.irq_enable.unwrap())(11),
            DummyIrqAction::Noop
        );
        assert_eq!(
            (DUMMY_IRQ_CHIP_MODEL.irq_disable.unwrap())(11),
            DummyIrqAction::Noop
        );
        assert_eq!(
            (DUMMY_IRQ_CHIP_MODEL.irq_ack.unwrap())(11),
            DummyIrqAction::Noop
        );
        assert_eq!(
            (DUMMY_IRQ_CHIP_MODEL.irq_mask.unwrap())(11),
            DummyIrqAction::Noop
        );
        assert_eq!(
            (DUMMY_IRQ_CHIP_MODEL.irq_unmask.unwrap())(11),
            DummyIrqAction::Noop
        );
    }

    #[test]
    fn legacy_irq_chip_export_remains_usable() {
        assert_eq!(DUMMY_IRQ_CHIP.name, "dummy");
        assert!(DUMMY_IRQ_CHIP.mask.is_some());
        assert!(DUMMY_IRQ_CHIP.unmask.is_some());
        assert!(DUMMY_IRQ_CHIP.ack.is_some());
        assert!(DUMMY_IRQ_CHIP.eoi.is_none());
    }

    #[test]
    fn dummy_irq_chip_abi_layout_matches_vendor_header() {
        use core::mem::{offset_of, size_of};

        assert_eq!(offset_of!(LinuxIrqChipAbi, name), 0);
        assert_eq!(offset_of!(LinuxIrqChipAbi, irq_startup), 8);
        assert_eq!(offset_of!(LinuxIrqChipAbi, irq_ack), 40);
        assert_eq!(offset_of!(LinuxIrqChipAbi, irq_set_affinity), 80);
        assert_eq!(offset_of!(LinuxIrqChipAbi, irq_suspend), 136);
        assert_eq!(offset_of!(LinuxIrqChipAbi, irq_compose_msi_msg), 192);
        assert_eq!(offset_of!(LinuxIrqChipAbi, irq_force_complete_move), 264);
        assert_eq!(offset_of!(LinuxIrqChipAbi, flags), 272);
        assert_eq!(size_of::<LinuxIrqChipAbi>(), 280);
    }

    #[test]
    fn dummy_irq_chip_export_registers_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("dummy_irq_chip"),
            Some(core::ptr::addr_of!(DUMMY_IRQ_CHIP_ABI) as usize)
        );
        assert_eq!(DUMMY_IRQ_CHIP_ABI.flags, IRQCHIP_SKIP_SET_WAKE as usize);
        assert!(DUMMY_IRQ_CHIP_ABI.irq_mask.is_some());
        assert!(DUMMY_IRQ_CHIP_ABI.irq_unmask.is_some());
    }
}
