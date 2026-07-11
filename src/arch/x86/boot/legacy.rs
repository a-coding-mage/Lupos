//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot
//! test-origin: linux:vendor/linux/arch/x86/boot
//! Linux x86 legacy boot option helpers.
//!
//! Lupos is entered by GRUB's Linux boot-protocol loader with setup already done,
//! so real-mode BIOS callbacks, A20 toggling, EDD/APM queries, and compressed
//! kernel relocation are not live boot paths. The implemented pieces here are
//! the loader-visible option parsers still consumed after the bzImage handoff.
//!
//! References:
//! - `vendor/linux/arch/x86/boot/early_serial_console.c`
//! - `vendor/linux/arch/x86/boot/video-mode.c`

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirmwareThunk {
    A20Gate,
    ApmBios,
    EddBios,
    VesaBios,
    RealModeWake,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootVideoMode {
    Text80x25,
    Vga(u16),
    Vesa(u16),
}

/// Linux default text mode when no loader-selected graphics mode is accepted.
pub const DEFAULT_TEXT_MODE: BootVideoMode = BootVideoMode::Text80x25;

/// Return whether a BIOS/real-mode thunk exists after the bzImage handoff.
pub const fn firmware_thunk_available(_thunk: FirmwareThunk) -> bool {
    false
}

/// Linux-compatible error for legacy firmware calls when no real-mode thunk is installed.
pub const fn firmware_thunk_errno(thunk: FirmwareThunk) -> i32 {
    match thunk {
        FirmwareThunk::A20Gate | FirmwareThunk::RealModeWake => ENODEV,
        FirmwareThunk::ApmBios | FirmwareThunk::EddBios | FirmwareThunk::VesaBios => EOPNOTSUPP,
    }
}

/// Parse `vga=` values used by Linux boot/video-mode code.
pub fn parse_vga_option(value: &[u8]) -> Option<BootVideoMode> {
    if value.eq_ignore_ascii_case(b"normal") {
        return Some(BootVideoMode::Text80x25);
    }
    let mut n = 0u16;
    for &b in value {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as u16)?;
    }
    Some(BootVideoMode::Vga(n))
}

/// Select the boot video mode from a Linux command line.
pub fn select_video_mode(cmdline: &[u8]) -> BootVideoMode {
    crate::arch::x86::boot::cmdline_find_option(cmdline, "vga")
        .and_then(parse_vga_option)
        .unwrap_or(DEFAULT_TEXT_MODE)
}

/// Return true when the command line requests Linux KASLR to stay disabled.
pub fn kaslr_disabled(cmdline: &[u8]) -> bool {
    crate::arch::x86::boot::cmdline_has_option(cmdline, "nokaslr")
}

/// Return true if the decompressor should use early serial output.
pub fn early_serial_requested(cmdline: &[u8]) -> bool {
    crate::arch::x86::boot::cmdline_has_option(cmdline, "earlyprintk")
        || crate::arch::x86::boot::cmdline_has_option(cmdline, "earlycon")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_thunks_fail_closed_after_bzimage_handoff() {
        assert!(!firmware_thunk_available(FirmwareThunk::A20Gate));
        assert_eq!(firmware_thunk_errno(FirmwareThunk::ApmBios), EOPNOTSUPP);
    }

    #[test]
    fn vga_option_parser_matches_linux_forms() {
        assert_eq!(parse_vga_option(b"normal"), Some(BootVideoMode::Text80x25));
        assert_eq!(parse_vga_option(b"791"), Some(BootVideoMode::Vga(791)));
        assert_eq!(parse_vga_option(b"ask"), None);
    }

    #[test]
    fn command_line_controls_boot_policies() {
        let cmdline = b"root=/dev/vda1 vga=791 nokaslr earlyprintk=serial\0";
        assert_eq!(select_video_mode(cmdline), BootVideoMode::Vga(791));
        assert!(kaslr_disabled(cmdline));
        assert!(early_serial_requested(cmdline));
    }
}
