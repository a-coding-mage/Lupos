//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-bios.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-bios.c
//! Real-mode wrapper for Linux `arch/x86/boot/video-bios.c`.

pub use crate::arch::x86::boot::video_bios::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::boot::biosregs::{BiosCaller, BiosRegs};
    use crate::arch::x86::boot::video::VideoState;

    struct SameModeBios;

    impl BiosCaller for SameModeBios {
        fn intcall(&self, _int_no: u8, _ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            if let Some(out) = oreg {
                out.set_ax(0x0003);
            }
        }
    }

    #[test]
    fn wrapper_includes_boot_video_bios_c() {
        assert_eq!(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/vendor/linux/arch/x86/realmode/rm/video-bios.c"
            ))
            .trim(),
            "#include \"../../boot/video-bios.c\""
        );

        let mut state = VideoState::default();
        assert_eq!(set_bios_mode(&SameModeBios, &mut state, 0x03), 0);
    }
}
