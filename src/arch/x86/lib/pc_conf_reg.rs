//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/pc-conf-reg.c
//! test-origin: linux:vendor/linux/arch/x86/lib/pc-conf-reg.c
//! PC configuration register space (I/O ports 0x22/0x23).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/pc-conf-reg.c
//! - vendor/linux/arch/x86/include/asm/pc-conf-reg.h
//!
//! Indirect access space used by the MP Spec, Cyrix CPUs, and several
//! chipsets. Linux protects the space with a raw spinlock because both
//! halves of the access (index byte, then data byte) must remain coherent
//! against interrupts and other CPUs. We mirror the lock and offer the
//! `pc_conf_get` / `pc_conf_set` helpers from the asm header.

use crate::arch::x86::include::asm::io::{inb, outb};
use crate::kernel::locking::raw_spinlock::RawSpinLock;

/// Index port — Linux `PC_CONF_INDEX`.
pub const PC_CONF_INDEX: u16 = 0x22;

/// Data port — Linux `PC_CONF_DATA`.
pub const PC_CONF_DATA: u16 = 0x23;

/// MP Spec IMCR (Interrupt Mode Control Register) — Linux `PC_CONF_MPS_IMCR`.
pub const PC_CONF_MPS_IMCR: u8 = 0x70;

/// Indirect-register lock. Mirrors `DEFINE_RAW_SPINLOCK(pc_conf_lock)` in
/// `vendor/linux/arch/x86/lib/pc-conf-reg.c`.
pub static PC_CONF_LOCK: RawSpinLock = RawSpinLock::new();

/// Read indirect register `reg`. Caller must hold `PC_CONF_LOCK`.
///
/// Mirrors `pc_conf_get()` in
/// `vendor/linux/arch/x86/include/asm/pc-conf-reg.h`.
///
/// # Safety
/// Performs raw I/O at ports 0x22/0x23. Caller must (a) hold `PC_CONF_LOCK`
/// across the index+data pair to keep the access atomic against other CPUs
/// and IRQs, and (b) ensure the system actually exposes a PC config space
/// (legacy hardware or MP-Spec IMCR systems).
#[inline]
pub unsafe fn pc_conf_get(reg: u8) -> u8 {
    unsafe {
        outb(PC_CONF_INDEX, reg);
        inb(PC_CONF_DATA)
    }
}

/// Write `data` to indirect register `reg`. Caller must hold `PC_CONF_LOCK`.
///
/// Mirrors `pc_conf_set()` in `pc-conf-reg.h`.
///
/// # Safety
/// Same constraints as `pc_conf_get`.
#[inline]
pub unsafe fn pc_conf_set(reg: u8, data: u8) {
    unsafe {
        outb(PC_CONF_INDEX, reg);
        outb(PC_CONF_DATA, data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_constants_match_linux_pc_conf_reg_h() {
        assert_eq!(PC_CONF_INDEX, 0x22);
        assert_eq!(PC_CONF_DATA, 0x23);
        assert_eq!(PC_CONF_MPS_IMCR, 0x70);
    }

    #[test]
    fn lock_is_constructible_at_compile_time() {
        // The lock must be usable as a `static` initializer (matches Linux's
        // `DEFINE_RAW_SPINLOCK`). `RawSpinLock::new()` is `const fn` so this
        // line passes type-check only if that invariant holds.
        static _ASSERT: &RawSpinLock = &PC_CONF_LOCK;
        assert!(!_ASSERT.is_locked());
    }
}
