//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot/startup
//! Startup-stage (`boot/startup/`) ports — runs after decompression
//! but before the kernel's protected-mode entry. Contains GDT/IDT,
//! page-table installer, SEV/SME bring-up.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/startup/

pub mod gdt_idt;
pub mod map_kernel;
pub mod sev_shared;
pub mod sev_startup;
pub mod sme;
