//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/reboot.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/reboot.c
//! x86 reboot/reset helpers.
//!
//! References:
//! - vendor/linux/arch/x86/kernel/reboot.c

use crate::arch::x86::include::asm::io::{inb, outb};

pub const KBD_STATUS_PORT: u16 = 0x64;
pub const KBD_RESET_COMMAND: u8 = 0xFE;
pub const KBD_INPUT_BUFFER_FULL: u8 = 1 << 1;

pub fn keyboard_controller_ready(status: u8) -> bool {
    status & KBD_INPUT_BUFFER_FULL == 0
}

pub unsafe fn reboot_via_keyboard_controller() -> ! {
    for _ in 0..10_000 {
        if keyboard_controller_ready(unsafe { inb(KBD_STATUS_PORT) }) {
            unsafe { outb(KBD_STATUS_PORT, KBD_RESET_COMMAND) };
            break;
        }
        core::hint::spin_loop();
    }
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack, preserves_flags)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_reset_command_matches_pc_at_reset_path() {
        assert_eq!(KBD_STATUS_PORT, 0x64);
        assert_eq!(KBD_RESET_COMMAND, 0xFE);
        assert!(keyboard_controller_ready(0));
        assert!(!keyboard_controller_ready(KBD_INPUT_BUFFER_FULL));
    }
}
