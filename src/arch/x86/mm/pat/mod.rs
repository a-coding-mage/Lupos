//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pat
//! x86 PAT memory-type helpers.
//!
//! This nested module mirrors the files under `vendor/linux/arch/x86/mm/pat/`
//! while leaving the existing flat `crate::arch::x86::mm::pat` cache-mode helpers
//! as the live flag encoder.

pub mod memtype_interval;
pub mod set_memory;

pub mod cachemode;
pub use cachemode::*;

pub fn register_module_exports() {
    cachemode::register_module_exports();
    set_memory::register_module_exports();
}
