//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/error-inject.c
//! test-origin: linux:vendor/linux/arch/x86/lib/error-inject.c
//! x86 error-injection trampoline override hook.

pub use crate::arch::x86::kernel::error_inject::{just_return_func, override_function_with_return};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::kernel::ptrace::PtRegs;

    #[test]
    fn lib_error_inject_exports_trampoline_override() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/error-inject.c"
        ));
        assert!(source.contains("asmlinkage void just_return_func(void);"));
        assert!(source.contains("void override_function_with_return(struct pt_regs *regs)"));
        assert!(source.contains("regs->ip = (unsigned long)&just_return_func;"));

        let mut regs: PtRegs = unsafe { core::mem::zeroed() };
        unsafe { override_function_with_return(&mut regs) };
        assert_eq!(regs.rip, just_return_func as usize as u64);
    }
}
