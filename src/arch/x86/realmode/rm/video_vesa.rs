//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-vesa.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-vesa.c
//! Real-mode wrapper for Linux `arch/x86/boot/video-vesa.c`.

pub use crate::arch::x86::boot::video_vesa::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::boot::video::VIDEO_FIRST_VESA;

    #[test]
    fn wrapper_includes_boot_video_vesa_c() {
        assert_eq!(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/arch/x86/realmode/rm/video-vesa.c"
            ))
            .trim(),
            "#include \"../../boot/video-vesa.c\""
        );

        assert_eq!(VESA_XMODE_FIRST, VIDEO_FIRST_VESA);
    }
}
