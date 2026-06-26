//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-mode.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-mode.c
//! Real-mode wrapper for Linux `arch/x86/boot/video-mode.c`.

pub use crate::arch::x86::boot::video_mode::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::boot::video::{NORMAL_VGA, VIDEO_RECALC};

    #[test]
    fn wrapper_includes_boot_video_mode_c() {
        assert_eq!(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/arch/x86/realmode/rm/video-mode.c"
            ))
            .trim(),
            "#include \"../../boot/video-mode.c\""
        );

        assert_eq!(NORMAL_VGA, 0xffff);
        assert_eq!(VIDEO_RECALC, 0x8000);
    }
}
