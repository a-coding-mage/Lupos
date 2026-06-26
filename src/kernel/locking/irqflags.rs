//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! Local IRQ-flag management (M33).
//!
//! Mirrors `vendor/linux/arch/x86/include/asm/irqflags.h`.  On x86, the IF
//! (interrupt enable) bit lives in EFLAGS and is manipulated by
//! `cli`/`sti`/`pushfq`/`popfq`.

/// EFLAGS.IF = bit 9.  Set when interrupts are enabled.
pub const X86_EFLAGS_IF: u64 = 1u64 << 9;

/// Linux `unsigned long flags;` parameter type.
pub type IrqFlags = u64;

/// Read EFLAGS without modifying it.
#[inline(always)]
pub fn arch_local_save_flags() -> IrqFlags {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    {
        let flags: u64;
        unsafe {
            core::arch::asm!(
                "pushfq",
                "pop {0}",
                out(reg) flags,
                options(nomem, preserves_flags),
            );
        }
        return flags;
    }
    #[cfg(any(not(target_arch = "x86_64"), test))]
    return 0;
}

/// Clear EFLAGS.IF (`cli`).
#[inline(always)]
pub fn arch_local_irq_disable() {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

/// Set EFLAGS.IF (`sti`).
#[inline(always)]
pub fn arch_local_irq_enable() {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

/// Save EFLAGS into `*flags` and disable interrupts.
#[inline(always)]
pub fn local_irq_save() -> IrqFlags {
    let flags = arch_local_save_flags();
    arch_local_irq_disable();
    flags
}

/// Restore EFLAGS from `flags`.
#[inline(always)]
pub fn local_irq_restore(flags: IrqFlags) {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        core::arch::asm!(
            "push {0}",
            "popfq",
            in(reg) flags,
            options(nomem),
        );
    }
    let _ = flags;
}

#[inline(always)]
pub fn local_irq_disable() {
    arch_local_irq_disable();
}

#[inline(always)]
pub fn local_irq_enable() {
    arch_local_irq_enable();
}

/// Predicate: returns true if interrupts were enabled in `flags`.
#[inline]
pub fn irqs_disabled_flags(flags: IrqFlags) -> bool {
    flags & X86_EFLAGS_IF == 0
}

/// Predicate: are interrupts currently disabled?
#[inline]
pub fn irqs_disabled() -> bool {
    irqs_disabled_flags(arch_local_save_flags())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eflags_if_bit_is_bit_9() {
        assert_eq!(X86_EFLAGS_IF, 0x200);
    }

    #[test]
    fn irqs_disabled_flags_inverts_if_bit() {
        assert!(irqs_disabled_flags(0));
        assert!(!irqs_disabled_flags(X86_EFLAGS_IF));
    }
}
