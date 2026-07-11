//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-mode.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-mode.c
//! Real-mode `_WAKEUP` build of Linux `arch/x86/boot/video-mode.c`.
//!
//! `_WAKEUP` compiles out the `boot_params.hdr.vid_mode` write after a mode
//! is selected. The wrapper therefore exposes a distinct `set_mode` entry
//! instead of re-exporting the normal-boot implementation.

use crate::arch::x86::boot::video::{CardInfo, VideoState};
use crate::arch::x86::boot::video_mode as boot_mode;

pub use boot_mode::{VgaRecalcIo, mode_defined, probe_cards, raw_set_mode, vga_recalc_vertical};

pub fn set_mode<C: CardInfo, I: VgaRecalcIo>(
    cards: &mut [C],
    st: &mut VideoState,
    recalc_io: &mut I,
    mode: u16,
) -> i32 {
    boot_mode::set_mode_wakeup(cards, st, recalc_io, mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use crate::arch::x86::boot::video::{ModeInfo, NORMAL_VGA, VIDEO_80X25, VIDEO_RECALC};
    use alloc::vec::Vec;

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

    struct WakeCard {
        mode: ModeInfo,
    }

    impl CardInfo for WakeCard {
        fn card_name(&self) -> &str {
            "wake"
        }
        fn set_mode(&mut self, _mode: &ModeInfo) -> i32 {
            0
        }
        fn nmodes(&self) -> i32 {
            1
        }
        fn set_nmodes(&mut self, _n: i32) {}
        fn mode(&self, index: usize) -> Option<ModeInfo> {
            (index == 0).then_some(self.mode)
        }
    }

    struct WakeVga {
        writes: Vec<(u16, u8, u8)>,
    }

    impl VgaRecalcIo for WakeVga {
        fn font_size(&mut self) -> u8 {
            16
        }
        fn text_rows(&self) -> u8 {
            24
        }
        fn vga_crtc(&self) -> u16 {
            0x3d4
        }
        fn in_idx(&mut self, _port: u16, _index: u8) -> u8 {
            0
        }
        fn out_idx(&mut self, value: u8, port: u16, index: u8) {
            self.writes.push((port, index, value));
        }
    }

    #[test]
    fn wakeup_set_mode_recalculates_but_does_not_store_boot_vid_mode() {
        let mut cards = [WakeCard {
            mode: ModeInfo {
                mode: VIDEO_80X25,
                x: 80,
                y: 25,
                depth: 0,
            },
        }];
        let mut state = VideoState {
            video_mode: 0xaaaa,
            force_y: 25,
            ..Default::default()
        };
        let mut vga = WakeVga { writes: Vec::new() };

        assert_eq!(
            set_mode(&mut cards, &mut state, &mut vga, VIDEO_80X25 | VIDEO_RECALC,),
            0
        );
        assert_eq!(vga.writes.len(), 3, "VIDEO_RECALC must not be skipped");
        assert_eq!(
            state.video_mode, 0xaaaa,
            "_WAKEUP excludes boot_params write"
        );
    }
}
