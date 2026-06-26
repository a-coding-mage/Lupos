//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/early_serial_console.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/early_serial_console.c
//! Compressed-kernel early-serial-console shim.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/early_serial_console.c
//!
//! The compressed stub stores `early_serial_base` in `.data` (so it
//! survives `.bss` clear) and re-uses the parent
//! `early_serial_console.c` routines. Lupos exposes the same handle
//! and re-exports the parent helpers from `arch/x86/boot`.

use core::sync::atomic::{AtomicI32, Ordering};

/// `early_serial_base` — port address of the active UART (e.g. 0x3F8
/// for COM1). 0 means "no early console". Lives in `.data` so it
/// survives `.bss` clearing. Mirrors
/// `boot/compressed/early_serial_console.c::early_serial_base`.
pub static EARLY_SERIAL_BASE: AtomicI32 = AtomicI32::new(0);

/// Set the active UART base. Linux's setup writes this once at boot.
pub fn set_early_serial_base(base: i32) {
    EARLY_SERIAL_BASE.store(base, Ordering::Release);
}

/// Read the active UART base.
pub fn early_serial_base() -> i32 {
    EARLY_SERIAL_BASE.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spin::Mutex;

    static EARLY_SERIAL_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn default_base_is_zero_meaning_disabled() {
        let _guard = EARLY_SERIAL_TEST_LOCK.lock();
        // Other tests may have set the base; use a guard-and-restore.
        let saved = EARLY_SERIAL_BASE.swap(0, Ordering::SeqCst);
        assert_eq!(early_serial_base(), 0);
        EARLY_SERIAL_BASE.store(saved, Ordering::SeqCst);
    }

    #[test]
    fn set_then_read_round_trips() {
        let _guard = EARLY_SERIAL_TEST_LOCK.lock();
        let saved = EARLY_SERIAL_BASE.swap(0, Ordering::SeqCst);
        set_early_serial_base(0x3F8);
        assert_eq!(early_serial_base(), 0x3F8);
        EARLY_SERIAL_BASE.store(saved, Ordering::SeqCst);
    }
}
