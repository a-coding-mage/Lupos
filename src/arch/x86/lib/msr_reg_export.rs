//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/msr-reg-export.c
//! test-origin: linux:vendor/linux/arch/x86/lib/msr-reg-export.c
//! Export wrapper for fault-safe MSR register helpers.

pub use crate::arch::x86::kernel::msr::{rdmsr_safe_regs, wrmsr_safe_regs};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_exports_safe_reg_helpers() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/msr-reg-export.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL(rdmsr_safe_regs);"));
        assert!(source.contains("EXPORT_SYMBOL(wrmsr_safe_regs);"));

        let _read: fn(&mut [u32; 8]) -> Result<(), i32> = rdmsr_safe_regs;
        let _write: fn(&mut [u32; 8]) -> Result<(), i32> = wrmsr_safe_regs;
    }
}
