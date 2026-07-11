//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-vga.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-vga.c
//! Real-mode `_WAKEUP` build of Linux `arch/x86/boot/video-vga.c`.
//!
//! Adapter detection still updates the shared `adapter` global, but `_WAKEUP`
//! compiles out both writes to `boot_params.screen_info`.

use crate::arch::x86::boot::biosregs::BiosCaller;
use crate::arch::x86::boot::video::VideoState;
use crate::arch::x86::boot::video_vga as boot_vga;

pub use boot_vga::{
    CGA_MODES, EGA_MODES, VGA_CARD_NAMES, VGA_MODES, VgaModeTable, vga_crtc, vga_set_8font,
    vga_set_14font, vga_set_80x30, vga_set_80x34, vga_set_80x43, vga_set_80x60,
    vga_set_480_scanlines, vga_set_basic_mode, vga_set_mode, vga_set_vertical_end,
};

pub fn vga_probe<B: BiosCaller>(bios: &B, st: &mut VideoState) -> (VgaModeTable, i32) {
    boot_vga::vga_probe_wakeup(bios, st)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::boot::biosregs::BiosRegs;
    use crate::arch::x86::boot::video::ADAPTER_VGA;

    struct VgaBios;

    impl BiosCaller for VgaBios {
        fn intcall(&self, _int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            if let Some(out) = oreg {
                *out = BiosRegs::default();
                match ireg.ax() {
                    0x1200 => out.ebx = 0x0003,
                    0x1a00 => out.set_al(0x1a),
                    _ => {}
                }
            }
        }
    }

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

    #[test]
    fn wakeup_probe_detects_vga_without_writing_screen_info() {
        let mut state = VideoState::default();
        state.screen_info.orig_video_ega_bx = 0xbeef;
        state.screen_info.orig_video_isvga = 0x7a;

        let (table, count) = vga_probe(&VgaBios, &mut state);

        assert_eq!(state.adapter, ADAPTER_VGA);
        assert_eq!(table, VgaModeTable::Vga);
        assert_eq!(count, VGA_MODES.len() as i32);
        assert_eq!(state.screen_info.orig_video_ega_bx, 0xbeef);
        assert_eq!(state.screen_info.orig_video_isvga, 0x7a);
    }
}
