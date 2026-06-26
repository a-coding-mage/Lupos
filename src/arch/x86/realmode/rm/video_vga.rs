//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-vga.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-vga.c
//! Real-mode wrapper for Linux `arch/x86/boot/video-vga.c`.

pub use crate::arch::x86::boot::video_vga::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_includes_boot_video_vga_c() {
        assert_eq!(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/arch/x86/realmode/rm/video-vga.c"
            ))
            .trim(),
            "#include \"../../boot/video-vga.c\""
        );

        assert_eq!(VGA_MODES.len(), 7);
    }
}
