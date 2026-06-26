//! linux-parity: partial
//! linux-source: vendor/linux/drivers/platform
/// QEMU exit device — allows the kernel to terminate the emulator.
///
/// QEMU's `isa-debug-exit` device maps an I/O port that, when written,
/// causes QEMU to exit with a computed status code:
///
///     exit_code = (value << 1) | 1
///
/// We use I/O port `0xF4` (configured via `-device isa-debug-exit,iobase=0xf4`).
///
/// | Written value | QEMU exit code | Meaning |
/// |---------------|---------------|---------|
/// | 0x10          | 0x21 (33)     | Success |
/// | 0x00          | 0x01 (1)      | Failure |
///
/// Ref: https://wiki.osdev.org/Shutdown#QEMU
/// Ref: vendor/linux/arch/x86/kernel/reboot.c (kbd reboot path: `outb(0xfe, 0x64)`)

const ISA_DEBUG_EXIT_PORT: u16 = 0xF4;

/// PS/2 8042 keyboard controller command port.  Writing `0xFE` toggles the
/// CPU reset line on real hardware and triggers a machine reset under QEMU.
/// This matches the canonical Linux `BOOT_KBD` reboot path in
/// `vendor/linux/arch/x86/kernel/reboot.c`.
const KBD_COMMAND_PORT: u16 = 0x64;
const KBD_RESET_CMD: u8 = 0xFE;

/// Exit QEMU indicating success (exit code 33 / 0x21).
pub fn exit_success() -> ! {
    // Boot tests assert serial banners and then leave through isa-debug-exit.
    // A full fbcon repaint can be much slower than the test-exit contract.
    crate::kernel::console::flush_serial_budgeted();
    unsafe {
        crate::arch::x86::include::asm::io::outb(ISA_DEBUG_EXIT_PORT, 0x10);
    }
    halt()
}

/// Exit QEMU indicating failure (exit code 1).
pub fn exit_failure() -> ! {
    crate::kernel::console::flush_serial_budgeted();
    unsafe {
        crate::arch::x86::include::asm::io::outb(ISA_DEBUG_EXIT_PORT, 0x00);
    }
    halt()
}

/// Hand control back to the firmware/hypervisor and force a machine restart.
///
/// First pulses the PS/2 keyboard controller reset line (QEMU honours this
/// and resets the machine), then falls through to the `isa-debug-exit`
/// success port so a `-no-reboot` QEMU invocation still terminates cleanly
/// rather than spinning in the halt loop.  On real hardware the second
/// `outb` is a harmless write to an unused I/O port.
pub fn machine_restart() -> ! {
    crate::kernel::console::flush_all_blocking();
    unsafe {
        crate::arch::x86::include::asm::io::outb(KBD_COMMAND_PORT, KBD_RESET_CMD);
        crate::arch::x86::include::asm::io::outb(ISA_DEBUG_EXIT_PORT, 0x10);
    }
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}
