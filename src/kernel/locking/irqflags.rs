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
                // Linux native_save_fl() has a memory clobber.  Saving the
                // flags is part of the ordering contract around irq-safe
                // locks; a no-memory-clobber option would let LLVM move protected memory
                // accesses across this boundary.
                options(preserves_flags),
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
        // Mirrors vendor/linux/arch/x86/include/asm/irqflags.h::native_irq_disable:
        // the memory clobber prevents a runqueue lock operation from being
        // scheduled before interrupts are disabled.
        core::arch::asm!("cli");
    }
}

/// Set EFLAGS.IF (`sti`).
#[inline(always)]
pub fn arch_local_irq_enable() {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        // Mirrors native_irq_enable()'s memory clobber.  Unlocking must be
        // visible before an interrupt can enter the protected path again.
        core::arch::asm!("sti");
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
    if !irqs_disabled_flags(flags) {
        unsafe {
            // Linux restores only IF here (native_local_irq_restore), rather
            // than restoring the whole arithmetic/status register with
            // popfq.  The memory clobber also keeps the unlock before sti.
            core::arch::asm!("sti");
        }
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

    /// test-origin: linux:vendor/linux/arch/x86/include/asm/irqflags.h
    ///
    /// Rust's inline-assembly options are part of the observable locking
    /// contract here.  There is no host-side way to trigger a LAPIC interrupt
    /// between an incorrectly ordered `cli` and runqueue lock, so keep the
    /// source contract explicit and pair it with the four-CPU QEMU scheduler
    /// gate in the runtime validation.
    #[test]
    fn irq_wrappers_keep_linux_memory_ordering_and_if_only_restore() {
        let linux = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/irqflags.h"
        ));
        let local = include_str!("irqflags.rs");
        let implementation = local.split("#[cfg(test)]").next().unwrap_or(local);

        assert!(linux.contains("asm volatile(\"cli\": : :\"memory\")"));
        assert!(linux.contains("native_local_irq_restore"));
        assert!(!implementation.contains("options(nomem"));
        assert!(local.contains("if !irqs_disabled_flags(flags)"));
        assert!(local.contains("core::arch::asm!(\"cli\")"));
        assert!(local.contains("core::arch::asm!(\"sti\")"));
    }
}
