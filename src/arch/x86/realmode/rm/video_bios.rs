//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/realmode/rm/video-bios.c
//! test-origin: linux:vendor/linux/arch/x86/realmode/rm/video-bios.c
//! Real-mode `_WAKEUP` build of Linux `arch/x86/boot/video-bios.c`.
//!
//! This is deliberately not a wildcard re-export. Linux compiles the included
//! source with `_WAKEUP`, which removes failed-mode reversion and restores
//! hard-coded mode `0x03` after probing instead of reading `screen_info`.

use crate::arch::x86::boot::biosregs::BiosCaller;
use crate::arch::x86::boot::io::PortIoOps;
use crate::arch::x86::boot::video::{CardInfo, ModeInfo, VideoState};
use crate::arch::x86::boot::{video_bios as boot_bios, video_mode};

pub use boot_bios::{BIOS_CARD_NAME, BIOS_UNSAFE, BIOS_XMODE_FIRST, BIOS_XMODE_N, BiosArea};

/// `_WAKEUP` `set_bios_mode`: verify the selected mode but never attempt the
/// normal-boot revert through `boot_params.screen_info.orig_video_mode`.
pub fn set_bios_mode<B: BiosCaller>(bios: &B, st: &mut VideoState, mode: u8) -> i32 {
    boot_bios::set_bios_mode_wakeup(bios, st, mode)
}

pub fn bios_set_mode<B: BiosCaller>(bios: &B, st: &mut VideoState, mi: &ModeInfo) -> i32 {
    boot_bios::bios_set_mode_wakeup(bios, st, mi)
}

/// `_WAKEUP` `bios_probe`: use mode `0x03` as Linux's saved mode and honor
/// the setup-heap boundary before examining each candidate.
pub fn bios_probe<B, A>(
    bios: &B,
    io: &PortIoOps,
    area: &mut A,
    st: &mut VideoState,
    heap_bytes: usize,
    already_defined: &dyn Fn(u16) -> bool,
) -> alloc::vec::Vec<ModeInfo>
where
    B: BiosCaller,
    A: BiosArea,
{
    boot_bios::bios_probe_wakeup(bios, io, area, st, heap_bytes, already_defined)
}

pub fn bios_probe_with_cards<B, A, C>(
    bios: &B,
    io: &PortIoOps,
    area: &mut A,
    st: &mut VideoState,
    heap_bytes: usize,
    cards: &[C],
) -> alloc::vec::Vec<ModeInfo>
where
    B: BiosCaller,
    A: BiosArea,
    C: CardInfo,
{
    bios_probe(bios, io, area, st, heap_bytes, &|mode| {
        video_mode::mode_defined(cards, mode)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::boot::biosregs::BiosRegs;
    use core::cell::RefCell;

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

    struct FailedModeBios {
        calls: RefCell<alloc::vec::Vec<(u8, u8)>>,
    }

    impl BiosCaller for FailedModeBios {
        fn intcall(&self, _int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            self.calls.borrow_mut().push((ireg.ah(), ireg.al()));
            if let Some(out) = oreg {
                *out = BiosRegs::default();
                out.set_al(0x03);
            }
        }
    }

    struct EmptyArea;

    impl BiosArea for EmptyArea {
        fn set_fs(&mut self, _seg: u16) {}
        fn rdfs8(&self, _addr: u32) -> u8 {
            0
        }
        fn rdfs16(&self, _addr: u32) -> u16 {
            0
        }
    }

    fn zero_inb(_port: u16) -> u8 {
        0
    }
    fn discard_outb(_value: u8, _port: u16) {}
    fn discard_outw(_value: u16, _port: u16) {}

    #[test]
    fn wakeup_failed_mode_does_not_use_screen_info_revert() {
        let bios = FailedModeBios {
            calls: RefCell::new(alloc::vec::Vec::new()),
        };
        let mut state = VideoState::default();
        state.screen_info.orig_video_mode = 0x07;

        assert_eq!(set_bios_mode(&bios, &mut state, 0x55), -1);
        // Set 0x55 and query only. Normal boot would make a third call to
        // restore screen_info.orig_video_mode (0x07).
        assert_eq!(
            bios.calls.borrow().as_slice(),
            &[(0x00, 0x55), (0x0f, 0x55)]
        );
    }

    #[test]
    fn wakeup_probe_restores_hard_coded_mode_three() {
        let bios = FailedModeBios {
            calls: RefCell::new(alloc::vec::Vec::new()),
        };
        let io = PortIoOps {
            f_inb: zero_inb,
            f_outb: discard_outb,
            f_outw: discard_outw,
        };
        let mut area = EmptyArea;
        let mut state = VideoState {
            adapter: crate::arch::x86::boot::video::ADAPTER_VGA,
            ..Default::default()
        };
        state.screen_info.orig_video_mode = 0x07;

        // Zero heap makes Linux break before the first candidate, then execute
        // set_bios_mode(saved_mode). `_WAKEUP` defines saved_mode as 0x03.
        let modes = bios_probe(&bios, &io, &mut area, &mut state, 0, &|_| false);

        assert!(modes.is_empty());
        assert_eq!(
            bios.calls.borrow().as_slice(),
            &[(0x00, 0x03), (0x0f, 0x03)]
        );
    }
}
