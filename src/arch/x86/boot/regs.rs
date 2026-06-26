//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/regs.c
//! test-origin: linux:vendor/linux/arch/x86/boot/regs.c
//! Initialise a `biosregs` for a real-mode INT call.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/regs.c

use super::biosregs::{BiosRegs, X86_EFLAGS_CF};

/// Linux `initregs(reg)` — zero the register set, then prime CF so the
/// caller can detect "BIOS did nothing". `DS`/`ES`/`FS`/`GS` start out
/// as the current real-mode DS in the Linux code; lupos has no real-mode
/// segment registers at runtime, so we leave them at 0 — the layout and
/// the CF bit are what matter for ABI parity.
pub fn initregs(reg: &mut BiosRegs) {
    *reg = BiosRegs::default();
    reg.eflags |= X86_EFLAGS_CF;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initregs_zeros_struct_and_sets_carry_flag() {
        let mut r = BiosRegs {
            eax: 0xdead,
            ..Default::default()
        };
        initregs(&mut r);
        assert_eq!(r.eax, 0);
        assert_eq!(r.eflags & X86_EFLAGS_CF, X86_EFLAGS_CF);
    }
}
