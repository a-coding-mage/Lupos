//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/realmode/rm/regs.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/regs.c
//! Real-mode wrapper for Linux `arch/x86/boot/regs.c`.

pub use crate::arch::x86::boot::regs::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::boot::biosregs::{BiosRegs, X86_EFLAGS_CF};

    #[test]
    fn wrapper_includes_boot_regs_c() {
        assert_eq!(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/arch/x86/realmode/rm/regs.c"
            ))
            .trim(),
            "#include \"../../boot/regs.c\""
        );

        let mut regs = BiosRegs {
            eax: 0xbeef,
            ..Default::default()
        };
        initregs(&mut regs);
        assert_eq!(regs.eax, 0);
        assert_eq!(regs.eflags & X86_EFLAGS_CF, X86_EFLAGS_CF);
    }
}
