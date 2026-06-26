//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/video-bios.c
//! test-origin: linux:vendor/linux/arch/x86/boot/video-bios.c
//! Standard video-BIOS text-mode driver (silent + scanned).
//!
//! Ports / mirrors (1:1, no simplification):
//! - vendor/linux/arch/x86/boot/video-bios.c
//!
//! The BIOS card sets conventional INT 10h modes (`set_bios_mode`) and,
//! because probing them is destructive, exposes a "scan" probe (`bios_probe`)
//! that walks modes 0x14..0x7f, sets each, and keeps only those that verify as
//! text modes via the Attribute/Graphics-controller and CRTC checks. INT 10h
//! goes through [`BiosCaller`]; the VGA register reads use [`PortIoOps`] via
//! `in_idx`/`vga_crtc`; the BIOS-area `rdfs8`/`rdfs16` reads use [`BiosArea`].

use super::biosregs::{BiosCaller, BiosRegs};
use super::io::PortIoOps;
use super::regs::initregs;
use super::video::{CardInfo, ModeInfo, VIDEO_FIRST_BIOS, VideoState, in_idx};
use super::video_mode::mode_defined;
use super::video_vga::vga_crtc;

/// `video_bios` card metadata (video-bios.c:118-126).
pub const BIOS_CARD_NAME: &str = "BIOS";
/// `.unsafe = 1` — scanning sets every mode, so it is only done after "scan".
pub const BIOS_UNSAFE: bool = true;
/// `.xmode_first = VIDEO_FIRST_BIOS`.
pub const BIOS_XMODE_FIRST: u16 = VIDEO_FIRST_BIOS;
/// `.xmode_n = 0x80`.
pub const BIOS_XMODE_N: u16 = 0x80;

/// Seam for the BIOS-data-area reads the scan probe performs (`set_fs(0)`,
/// `rdfs16(0x44a)` = columns, `rdfs8(0x484)` = rows-1).
pub trait BiosArea {
    fn set_fs(&mut self, seg: u16);
    fn rdfs8(&self, addr: u32) -> u8;
    fn rdfs16(&self, addr: u32) -> u16;
}

/// `set_bios_mode(mode)` (video-bios.c:29-59) — INT 10h AH=00h Set Video Mode,
/// then AH=0Fh Get Current Video Mode to verify. On a clean change returns 0;
/// otherwise tries to revert to `orig_video_mode` and returns -1.
///
/// `orig_mode` carries `boot_params.screen_info.orig_video_mode` (the C code
/// reads it from boot_params under `#ifndef _WAKEUP`).
pub fn set_bios_mode<B: BiosCaller>(bios: &B, st: &mut VideoState, mode: u8) -> i32 {
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();

    initregs(&mut ireg);
    ireg.set_al(mode); // AH=0x00 Set Video Mode.
    bios.intcall(0x10, &ireg, None);

    ireg.set_ah(0x0f); // Get Current Video Mode.
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    st.do_restore = 1; // Assume video contents were lost.

    // Not all BIOSes are clean with the top bit.
    let new_mode = oreg.al() & 0x7f;

    if new_mode == mode {
        return 0; // Mode change OK.
    }

    if new_mode != st.screen_info.orig_video_mode {
        // Mode setting failed, but we didn't end up where we started.
        // That's bad. Try to revert to the original video mode.
        ireg.set_ax(st.screen_info.orig_video_mode as u16);
        bios.intcall(0x10, &ireg, None);
    }
    -1
}

/// `bios_set_mode(mi)` (video-bios.c:24-27) — recover the raw BIOS mode byte by
/// stripping `VIDEO_FIRST_BIOS` and delegate to `set_bios_mode`.
pub fn bios_set_mode<B: BiosCaller>(bios: &B, st: &mut VideoState, mi: &ModeInfo) -> i32 {
    set_bios_mode(bios, st, (mi.mode - VIDEO_FIRST_BIOS) as u8)
}

/// `bios_probe()` (video-bios.c:61-116) — the destructive scan. For each BIOS
/// mode 0x14..0x7f it skips already-defined modes, sets the mode, and verifies
/// it is a text mode by checking:
///   * Attribute Controller index 0x10 bit 0 == 0 (graphics disabled),
///   * Graphics Controller index 0x06 bit 0 == 0 (alpha addressing enabled),
///   * CRTC cursor-location-low (index 0x0f) == 0.
/// Verified modes are appended (text, x=cols, y=rows). The original mode is
/// restored at the end. Returns the number of modes found.
///
/// `cards` is the full card array used by `mode_defined`. The discovered modes
/// are returned to the caller, which installs them on the BIOS card (the C code
/// builds the list on the heap via `GET_HEAP`).
pub fn bios_probe<B, A>(
    bios: &B,
    io: &PortIoOps,
    area: &mut A,
    st: &mut VideoState,
    already_defined: &dyn Fn(u16) -> bool,
) -> alloc::vec::Vec<ModeInfo>
where
    B: BiosCaller,
    A: BiosArea,
{
    let mut out = alloc::vec::Vec::new();

    let saved_mode = st.screen_info.orig_video_mode;

    // The card is only probed for EGA/VGA adapters; the dispatcher already
    // gates on adapter via the safety bucket, but the C code re-checks:
    if st.adapter != super::video::ADAPTER_EGA && st.adapter != super::video::ADAPTER_VGA {
        return out;
    }

    area.set_fs(0);
    let crtc = vga_crtc(io);

    let mut mode: u16 = 0x14;
    while mode <= 0x7f {
        if already_defined(VIDEO_FIRST_BIOS + mode) {
            mode += 1;
            continue;
        }

        if set_bios_mode(bios, st, mode as u8) != 0 {
            mode += 1;
            continue;
        }

        // Try to verify that it's a text mode.

        // Attribute Controller: make sure graphics controller is disabled.
        if in_idx(io, 0x3c0, 0x10) & 0x01 != 0 {
            mode += 1;
            continue;
        }

        // Graphics Controller: verify Alpha addressing enabled.
        if in_idx(io, 0x3ce, 0x06) & 0x01 != 0 {
            mode += 1;
            continue;
        }

        // CRTC cursor location low should be zero(?).
        if in_idx(io, crtc, 0x0f) != 0 {
            mode += 1;
            continue;
        }

        out.push(ModeInfo {
            mode: VIDEO_FIRST_BIOS + mode,
            x: area.rdfs16(0x44a),
            y: area.rdfs8(0x484) as u16 + 1,
            depth: 0, // text
        });

        mode += 1;
    }

    set_bios_mode(bios, st, saved_mode);

    out
}

/// `mode_defined`-backed convenience: build the "already defined" predicate
/// from a card slice so `bios_probe` matches the C call
/// `mode_defined(VIDEO_FIRST_BIOS+mode)`.
pub fn bios_probe_with_cards<B, A, C>(
    bios: &B,
    io: &PortIoOps,
    area: &mut A,
    st: &mut VideoState,
    cards: &[C],
) -> alloc::vec::Vec<ModeInfo>
where
    B: BiosCaller,
    A: BiosArea,
    C: CardInfo,
{
    bios_probe(bios, io, area, st, &|m| mode_defined(cards, m))
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    // A BIOS that returns a configurable "current mode" from AH=0F, and
    // records every (int_no, ah, al) tuple.
    struct ModeBios {
        get_mode_al: RefCell<u8>,
        calls: RefCell<Vec<(u8, u8, u8)>>,
    }
    impl ModeBios {
        fn new(get_mode_al: u8) -> Self {
            ModeBios {
                get_mode_al: RefCell::new(get_mode_al),
                calls: RefCell::new(Vec::new()),
            }
        }
    }
    impl BiosCaller for ModeBios {
        fn intcall(&self, int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            self.calls.borrow_mut().push((int_no, ireg.ah(), ireg.al()));
            if let Some(o) = oreg {
                *o = BiosRegs::default();
                o.set_al(*self.get_mode_al.borrow());
            }
        }
    }

    #[test]
    fn set_bios_mode_returns_zero_when_new_matches_requested() {
        let bios = ModeBios::new(0x03);
        let mut st = VideoState::default();
        assert_eq!(set_bios_mode(&bios, &mut st, 0x03), 0);
        assert_eq!(st.do_restore, 1);
    }

    #[test]
    fn set_bios_mode_masks_top_bit_of_response() {
        // BIOS returns 0x83; Linux masks 0x7f => 0x03 matches request.
        let bios = ModeBios::new(0x83);
        let mut st = VideoState::default();
        assert_eq!(set_bios_mode(&bios, &mut st, 0x03), 0);
    }

    #[test]
    fn set_bios_mode_reverts_to_orig_on_failure() {
        // Request 0x55, BIOS reports 0x03; orig is 0x07 so it reverts.
        let bios = ModeBios::new(0x03);
        let mut st = VideoState::default();
        st.screen_info.orig_video_mode = 0x07;
        assert_eq!(set_bios_mode(&bios, &mut st, 0x55), -1);
        let calls = bios.calls.borrow();
        // set 0x55, get, revert-set 0x07.
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].2, 0x55); // AL of first set
        assert_eq!(calls[2].2, 0x07); // AL of revert
    }

    #[test]
    fn set_bios_mode_no_revert_when_landed_on_orig() {
        // Request 0x55, BIOS reports orig (0x03): failure but no revert.
        let bios = ModeBios::new(0x03);
        let mut st = VideoState::default();
        st.screen_info.orig_video_mode = 0x03;
        assert_eq!(set_bios_mode(&bios, &mut st, 0x55), -1);
        // Only two calls: set + get, no revert.
        assert_eq!(bios.calls.borrow().len(), 2);
    }

    #[test]
    fn bios_set_mode_strips_video_first_bios_base() {
        let bios = ModeBios::new(0x14);
        let mut st = VideoState::default();
        let mi = ModeInfo {
            mode: VIDEO_FIRST_BIOS + 0x14,
            x: 80,
            y: 25,
            depth: 0,
        };
        assert_eq!(bios_set_mode(&bios, &mut st, &mi), 0);
        // First call's AL was the stripped mode byte 0x14.
        assert_eq!(bios.calls.borrow()[0].2, 0x14);
    }

    #[test]
    fn card_metadata_matches_video_bios_c() {
        assert_eq!(BIOS_CARD_NAME, "BIOS");
        assert!(BIOS_UNSAFE);
        assert_eq!(BIOS_XMODE_FIRST, VIDEO_FIRST_BIOS);
        assert_eq!(BIOS_XMODE_N, 0x80);
    }

    // ---- bios_probe scan ---------------------------------------------

    // A BIOS that always succeeds on set, reporting back the mode it was told
    // to set (so set_bios_mode returns 0 for every mode).
    struct ScanBios {
        calls: RefCell<Vec<(u8, u8)>>, // (ah, al)
        last_set: RefCell<u8>,
    }
    impl ScanBios {
        fn new() -> Self {
            ScanBios {
                calls: RefCell::new(Vec::new()),
                last_set: RefCell::new(0),
            }
        }
    }
    impl BiosCaller for ScanBios {
        fn intcall(&self, _int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            let ah = ireg.ah();
            self.calls.borrow_mut().push((ah, ireg.al()));
            if ah == 0x00 {
                *self.last_set.borrow_mut() = ireg.al();
            }
            if let Some(o) = oreg {
                // AH=0F: report the mode we last set so verification passes.
                *o = BiosRegs::default();
                o.set_al(*self.last_set.borrow());
            }
        }
    }

    struct FakeArea {
        cols: u16,
        rows: u8,
    }
    impl BiosArea for FakeArea {
        fn set_fs(&mut self, _seg: u16) {}
        fn rdfs8(&self, addr: u32) -> u8 {
            if addr == 0x484 { self.rows } else { 0 }
        }
        fn rdfs16(&self, addr: u32) -> u16 {
            if addr == 0x44a { self.cols } else { 0 }
        }
    }

    // Port IO returning text-mode-passing register values:
    //  Attribute(0x3c0,0x10)&1 == 0, Graphics(0x3ce,0x06)&1 == 0, crtc 0x0f == 0.
    fn scan_inb_text(_port: u16) -> u8 {
        0
    }
    fn scan_inb_graphics_attr(port: u16) -> u8 {
        if port == 0x3c1 { 0x01 } else { 0 }
    }
    fn scan_outb(_v: u8, _port: u16) {}
    fn scan_outw(_v: u16, _port: u16) {}
    fn scan_io_text() -> PortIoOps {
        PortIoOps {
            f_inb: scan_inb_text,
            f_outb: scan_outb,
            f_outw: scan_outw,
        }
    }
    fn scan_io_graphics_attr() -> PortIoOps {
        PortIoOps {
            f_inb: scan_inb_graphics_attr,
            f_outb: scan_outb,
            f_outw: scan_outw,
        }
    }

    #[test]
    fn bios_probe_returns_empty_for_cga_adapter() {
        let bios = ScanBios::new();
        let io = scan_io_text();
        let mut area = FakeArea { cols: 80, rows: 24 };
        let mut st = VideoState {
            adapter: super::super::video::ADAPTER_CGA,
            ..Default::default()
        };
        let modes = bios_probe(&bios, &io, &mut area, &mut st, &|_| false);
        assert!(modes.is_empty());
    }

    #[test]
    fn bios_probe_collects_verified_text_modes_for_vga() {
        let bios = ScanBios::new();
        let io = scan_io_text();
        let mut area = FakeArea { cols: 80, rows: 24 };
        let mut st = VideoState {
            adapter: super::super::video::ADAPTER_VGA,
            ..Default::default()
        };
        // Nothing is already defined => every mode 0x14..0x7f is collected.
        let modes = bios_probe(&bios, &io, &mut area, &mut st, &|_| false);
        let expected = (0x7f - 0x14 + 1) as usize;
        assert_eq!(modes.len(), expected);
        // First mode is VIDEO_FIRST_BIOS + 0x14, x=80, y=25.
        assert_eq!(
            modes[0],
            ModeInfo {
                mode: VIDEO_FIRST_BIOS + 0x14,
                x: 80,
                y: 25,
                depth: 0
            }
        );
        // Last mode is VIDEO_FIRST_BIOS + 0x7f.
        assert_eq!(modes.last().unwrap().mode, VIDEO_FIRST_BIOS + 0x7f);
    }

    #[test]
    fn bios_probe_skips_already_defined_modes() {
        let bios = ScanBios::new();
        let io = scan_io_text();
        let mut area = FakeArea { cols: 80, rows: 24 };
        let mut st = VideoState {
            adapter: super::super::video::ADAPTER_VGA,
            ..Default::default()
        };
        // Pretend 0x14 is already defined => it should be skipped.
        let modes = bios_probe(&bios, &io, &mut area, &mut st, &|m| {
            m == VIDEO_FIRST_BIOS + 0x14
        });
        assert!(!modes.iter().any(|m| m.mode == VIDEO_FIRST_BIOS + 0x14));
        assert!(modes.iter().any(|m| m.mode == VIDEO_FIRST_BIOS + 0x15));
    }

    #[test]
    fn bios_probe_rejects_modes_failing_text_checks() {
        // Attribute controller bit 0 set => every mode rejected as graphics.
        let bios = ScanBios::new();
        let io = scan_io_graphics_attr();
        let mut area = FakeArea { cols: 80, rows: 24 };
        let mut st = VideoState {
            adapter: super::super::video::ADAPTER_VGA,
            ..Default::default()
        };
        let modes = bios_probe(&bios, &io, &mut area, &mut st, &|_| false);
        assert!(modes.is_empty());
    }
}
