//! linux-parity: partial
//! linux-source: vendor/linux/kernel/irq
//! Generic IRQ — M37.
//!
//! Mirrors `vendor/linux/kernel/irq/`.  Lupos M37 lifts the existing direct-
//! IDT routing into a Linux-shaped `irq_desc[]` framework so drivers can call
//! `request_irq` / `request_threaded_irq` unmodified once M55+ lands them.

pub mod affinity;
pub mod autoprobe;
pub mod chip;
pub mod cpuhotplug;
pub mod debugfs;
pub mod devres;
pub mod dummychip;
pub mod generic_chip;
pub mod handle;
pub mod ipi;
pub mod ipi_mux;
pub mod irq_sim;
pub mod irq_test;
pub mod irqdesc;
pub mod irqdomain;
pub mod kexec;
pub mod manage;
pub mod matrix;
pub mod migration;
pub mod msi;
pub mod pm;
pub mod proc;
pub mod resend;
pub mod spurious;
pub mod threaded;

pub use chip::IrqChip;
pub use handle::generic_handle_irq;
pub use irqdesc::{
    IRQ_DISABLED, IRQ_HANDLED, IRQ_NONE, IRQ_WAKE_THREAD, IrqAction, IrqDesc, IrqReturn, NR_IRQS,
    desc_for,
};
pub use irqdomain::IrqDomain;
pub use manage::{
    IRQF_ONESHOT, IRQF_SHARED, IRQF_TRIGGER_FALLING, IRQF_TRIGGER_HIGH, IRQF_TRIGGER_LOW,
    IRQF_TRIGGER_RISING, disable_irq, enable_irq, free_irq, irq_set_affinity, request_irq,
    request_threaded_irq,
};

pub fn register_module_exports() {
    irqdesc::register_module_exports();
    chip::register_module_exports();
    dummychip::register_module_exports();
    handle::register_module_exports();
    irqdomain::register_module_exports();
    manage::register_module_exports();
}
