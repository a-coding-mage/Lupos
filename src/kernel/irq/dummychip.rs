//! linux-parity: complete
//! linux-source: vendor/linux/kernel/irq/dummychip.c
//! test-origin: linux:vendor/linux/kernel/irq/dummychip.c
//! Generic no-controller and dummy IRQ chip implementations.
//!
//! Mirrors `vendor/linux/kernel/irq/dummychip.c`.

use super::chip::IrqChip;

pub const IRQCHIP_SKIP_SET_WAKE: u32 = 1 << 0;

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
}
