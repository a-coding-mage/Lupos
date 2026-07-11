//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot/video-mode.c
//! test-origin: linux:vendor/linux/arch/x86/boot/video-mode.c
//! Set the video mode (shared with the ACPI wakeup code).
//!
//! Ports / mirrors (1:1, no simplification):
//! - vendor/linux/arch/x86/boot/video-mode.c
//! - vendor/linux/arch/x86/boot/video.h (mode constants, card vtable)
//! - vendor/linux/arch/x86/include/uapi/asm/boot.h (NORMAL/EXTENDED_VGA)
//!
//! `probe_cards`, `mode_defined`, `raw_set_mode`, `vga_recalc_vertical` and
//! `set_mode` operate over a slice of [`CardInfo`] drivers. The canonical
//! `ModeInfo`/`VideoState` and all mode-id constants live in `video.rs`; this
//! file imports them so there is exactly one definition.

use super::video::{
    CardInfo, EXTENDED_VGA, ModeInfo, NORMAL_VGA, VIDEO_8POINT, VIDEO_80X25, VIDEO_CURRENT_MODE,
    VIDEO_RECALC, VideoState,
};

/// Real-mode VGA-register seam used by `vga_recalc_vertical`: `set_fs(0)` plus
/// `rdfs8`, the CRTC base from `vga_crtc()`, and the indexed `in_idx`/`out_idx`
/// register access. Threaded so the recalc can run against either real ports
/// in a setup-compatible path or deterministic test stubs.
pub trait VgaRecalcIo {
    /// `set_fs(0)` then `rdfs8(0x485)` — BIOS font size in pixels.
    fn font_size(&mut self) -> u8;
    /// `rdfs8(0x484)` — text-row count minus one.
    fn text_rows(&self) -> u8;
    /// `vga_crtc()` — CRTC base port.
    fn vga_crtc(&self) -> u16;
    /// `in_idx(port, index)`.
    fn in_idx(&mut self, port: u16, index: u8) -> u8;
    /// `out_idx(value, port, index)`.
    fn out_idx(&mut self, value: u8, port: u16, index: u8);
}

/// `probe_cards(unsafe)` (video-mode.c:31-49) — run each card's probe once per
/// safety bucket, recording the returned mode count. `probed[2]` is the
/// caller's persistent "already probed this bucket" memo.
pub fn probe_cards<C: CardInfo>(cards: &mut [C], unsafe_probe: bool, probed: &mut [bool; 2]) {
    let bucket = unsafe_probe as usize;
    if probed[bucket] {
        return;
    }
    probed[bucket] = true;

    for card in cards.iter_mut() {
        if card.is_unsafe() == unsafe_probe {
            // A driver with no probe yields 0 modes; CardInfo::probe defaults
            // to 0, matching the `card->probe ? ... : 0` branch.
            let n = card.probe();
            card.set_nmodes(n);
        }
    }
}

/// `mode_defined(mode)` (video-mode.c:52-67) — true if any card already has a
/// mode with this id.
pub fn mode_defined<C: CardInfo>(cards: &[C], mode: u16) -> bool {
    for card in cards.iter() {
        for i in 0..card.nmodes() as usize {
            if let Some(mi) = card.mode(i) {
                if mi.mode == mode {
                    return true;
                }
            }
        }
    }
    false
}

/// `raw_set_mode(mode, *real_mode)` (video-mode.c:70-111) — locate a mode by
/// fixed id, menu position, or resolution and set it; failing that, try an
/// "exceptional" (unprobed) range. Returns the driver's `set_mode` result, or
/// -1 if nothing matched.
pub fn raw_set_mode<C: CardInfo>(cards: &mut [C], mut mode: u16, real_mode: &mut u16) -> i32 {
    // Drop the recalc bit if set.
    mode &= !VIDEO_RECALC;

    // Scan for mode based on fixed ID, position, or resolution.
    let mut nmode: u16 = 0;
    for card in cards.iter_mut() {
        for i in 0..card.nmodes() as usize {
            let Some(mi) = card.mode(i) else { continue };
            let visible = mi.x != 0 || mi.y != 0;

            if (mode == nmode && visible)
                || mode == mi.mode
                || mode == (mi.y << 8).wrapping_add(mi.x)
            {
                *real_mode = mi.mode;
                return card.set_mode(&mi);
            }

            if visible {
                nmode = nmode.wrapping_add(1);
            }
        }
    }

    // Nothing found? Is it an "exceptional" (unprobed) mode?
    for card in cards.iter_mut() {
        let first = card.xmode_first() as u32;
        let n = card.xmode_n() as u32;
        if (mode as u32) >= first && (mode as u32) < first + n {
            // struct mode_info mix; *real_mode = mix.mode = mode; mix.x=mix.y=0;
            let mix = ModeInfo {
                mode,
                x: 0,
                y: 0,
                depth: 0,
            };
            *real_mode = mode;
            return card.set_mode(&mix);
        }
    }

    // Otherwise, failure...
    -1
}

/// `vga_recalc_vertical()` (video-mode.c:116-142) — recompute the vertical
/// cutoff register from the live font size and text-row count.
pub fn vga_recalc_vertical<I: VgaRecalcIo>(io: &mut I, force_y: i32) {
    let font_size = io.font_size() as u32; // set_fs(0); rdfs8(0x485)
    let mut rows: u32 = if force_y != 0 {
        force_y as u32
    } else {
        io.text_rows() as u32 + 1 // rdfs8(0x484)+1
    };

    rows *= font_size; // Visible scan lines.
    rows = rows.wrapping_sub(1); // ... minus one.

    let crtc = io.vga_crtc();

    let mut pt = io.in_idx(crtc, 0x11);
    pt &= !0x80; // Unlock CR0-7.
    io.out_idx(pt, crtc, 0x11);

    io.out_idx(rows as u8, crtc, 0x12); // Lower height register.

    let mut ov = io.in_idx(crtc, 0x07); // Overflow register.
    ov &= 0xbd;
    ov |= ((rows >> (8 - 1)) & 0x02) as u8;
    ov |= ((rows >> (9 - 6)) & 0x40) as u8;
    io.out_idx(ov, crtc, 0x07);
}

/// `set_mode(mode)` (video-mode.c:145-171) — translate the special mode
/// aliases, call `raw_set_mode`, optionally recalc, and store the canonical
/// (non-alias) mode number into `st`/`boot_params`. Returns 0 on success.
pub fn set_mode<C: CardInfo, I: VgaRecalcIo>(
    cards: &mut [C],
    st: &mut VideoState,
    recalc_io: &mut I,
    mode: u16,
) -> i32 {
    set_mode_for_build(cards, st, recalc_io, mode, true)
}

/// `_WAKEUP` build of `set_mode()`.
///
/// `arch/x86/realmode/rm/video-mode.c` includes this C file with `_WAKEUP`
/// defined.  The register programming is identical, but the
/// `boot_params.hdr.vid_mode = real_mode` store is compiled out.  Keeping a
/// distinct entry point prevents the real-mode wrapper from accidentally
/// mutating the normal-boot `VideoState::video_mode` mirror.
pub(crate) fn set_mode_wakeup<C: CardInfo, I: VgaRecalcIo>(
    cards: &mut [C],
    st: &mut VideoState,
    recalc_io: &mut I,
    mode: u16,
) -> i32 {
    set_mode_for_build(cards, st, recalc_io, mode, false)
}

fn set_mode_for_build<C: CardInfo, I: VgaRecalcIo>(
    cards: &mut [C],
    st: &mut VideoState,
    recalc_io: &mut I,
    mut mode: u16,
    store_canonical_mode: bool,
) -> i32 {
    // Very special mode numbers...
    if mode == VIDEO_CURRENT_MODE {
        return 0; // Nothing to do...
    } else if mode == NORMAL_VGA {
        mode = VIDEO_80X25;
    } else if mode == EXTENDED_VGA {
        mode = VIDEO_8POINT;
    }

    let mut real_mode: u16 = 0;
    let rv = raw_set_mode(cards, mode, &mut real_mode);
    if rv != 0 {
        return rv;
    }

    if mode & VIDEO_RECALC != 0 {
        // Linux cannot silently omit this operation: a VIDEO_RECALC request
        // always executes vga_recalc_vertical().  Requiring the I/O seam also
        // makes that invariant true in the Rust translation.
        vga_recalc_vertical(recalc_io, st.force_y);
    }

    // Save the canonical mode number for the kernel, not an alias, size
    // specification or menu position. (boot_params.hdr.vid_mode = real_mode)
    // The store is excluded from Linux's `_WAKEUP` build.
    if store_canonical_mode {
        st.video_mode = real_mode;
    }
    rv
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    struct TestCard {
        modes: Vec<ModeInfo>,
        nmodes: i32,
        probe_result: i32,
        is_unsafe: bool,
        xmode_first: u16,
        xmode_n: u16,
        set_modes: RefCell<Vec<ModeInfo>>,
        // Per-mode return code for set_mode; default 0.
        set_rv: RefCell<i32>,
    }
    impl TestCard {
        fn new(modes: Vec<ModeInfo>) -> Self {
            let n = modes.len() as i32;
            TestCard {
                modes,
                nmodes: n,
                probe_result: n,
                is_unsafe: false,
                xmode_first: 0,
                xmode_n: 0,
                set_modes: RefCell::new(Vec::new()),
                set_rv: RefCell::new(0),
            }
        }
    }
    impl CardInfo for TestCard {
        fn card_name(&self) -> &str {
            "TEST"
        }
        fn set_mode(&mut self, mode: &ModeInfo) -> i32 {
            self.set_modes.borrow_mut().push(*mode);
            *self.set_rv.borrow()
        }
        fn probe(&mut self) -> i32 {
            self.probe_result
        }
        fn nmodes(&self) -> i32 {
            self.nmodes
        }
        fn set_nmodes(&mut self, n: i32) {
            self.nmodes = n;
        }
        fn mode(&self, index: usize) -> Option<ModeInfo> {
            self.modes.get(index).copied()
        }
        fn is_unsafe(&self) -> bool {
            self.is_unsafe
        }
        fn xmode_first(&self) -> u16 {
            self.xmode_first
        }
        fn xmode_n(&self) -> u16 {
            self.xmode_n
        }
    }

    // Recalc IO stub: serves CR registers and records writes.
    struct TestVga {
        font_size: u8,
        text_rows: u8,
        crtc: u16,
        cr11: u8,
        cr07: u8,
        writes: Vec<(u16, u8, u8)>, // (port, index, value)
    }
    impl VgaRecalcIo for TestVga {
        fn font_size(&mut self) -> u8 {
            self.font_size
        }
        fn text_rows(&self) -> u8 {
            self.text_rows
        }
        fn vga_crtc(&self) -> u16 {
            self.crtc
        }
        fn in_idx(&mut self, _port: u16, index: u8) -> u8 {
            match index {
                0x11 => self.cr11,
                0x07 => self.cr07,
                _ => 0,
            }
        }
        fn out_idx(&mut self, value: u8, port: u16, index: u8) {
            self.writes.push((port, index, value));
        }
    }

    fn unused_recalc_io() -> TestVga {
        TestVga {
            font_size: 0,
            text_rows: 0,
            crtc: 0x3d4,
            cr11: 0,
            cr07: 0,
            writes: Vec::new(),
        }
    }

    #[test]
    fn probe_cards_runs_each_bucket_once() {
        let mut a = TestCard::new(Vec::new());
        a.probe_result = 2;
        a.nmodes = 0;
        let mut b = TestCard::new(Vec::new());
        b.is_unsafe = true;
        b.probe_result = 3;
        b.nmodes = 0;
        let mut cards = [a, b];
        let mut probed = [false; 2];

        probe_cards(&mut cards, false, &mut probed);
        probe_cards(&mut cards, false, &mut probed); // second call: no-op
        probe_cards(&mut cards, true, &mut probed);

        assert_eq!(cards[0].nmodes(), 2);
        assert_eq!(cards[1].nmodes(), 3);
        assert_eq!(probed, [true, true]);
    }

    #[test]
    fn mode_defined_finds_registered_mode() {
        let cards = [TestCard::new(alloc::vec![
            ModeInfo {
                mode: 0x0114,
                x: 800,
                y: 600,
                depth: 16
            },
            ModeInfo {
                mode: 0x0117,
                x: 1024,
                y: 768,
                depth: 16
            },
        ])];
        assert!(mode_defined(&cards, 0x0117));
        assert!(!mode_defined(&cards, 0x0199));
    }

    #[test]
    fn raw_set_mode_matches_menu_position() {
        // video-mode.c:86: (mode == nmode && visible).
        let mut cards = [TestCard::new(alloc::vec![
            ModeInfo {
                mode: 0x1234,
                x: 80,
                y: 25,
                depth: 0
            }, // nmode 0
            ModeInfo {
                mode: 0x2222,
                x: 0,
                y: 0,
                depth: 0
            }, // hidden, not counted
            ModeInfo {
                mode: 0x0117,
                x: 1024,
                y: 768,
                depth: 32
            }, // nmode 1
        ])];
        let mut real = 0u16;
        assert_eq!(raw_set_mode(&mut cards, 1, &mut real), 0);
        assert_eq!(real, 0x0117);
    }

    #[test]
    fn raw_set_mode_matches_fixed_id_and_resolution() {
        let mut cards = [TestCard::new(alloc::vec![ModeInfo {
            mode: 0x1234,
            x: 80,
            y: 25,
            depth: 0,
        }])];
        let mut real = 0u16;
        // Fixed id.
        assert_eq!(raw_set_mode(&mut cards, 0x1234, &mut real), 0);
        assert_eq!(real, 0x1234);
        // (y << 8) + x resolution form: (25 << 8) + 80.
        assert_eq!(raw_set_mode(&mut cards, (25 << 8) + 80, &mut real), 0);
        assert_eq!(real, 0x1234);
    }

    #[test]
    fn raw_set_mode_uses_exceptional_unprobed_range() {
        let mut card = TestCard::new(Vec::new());
        card.xmode_first = 0x0100;
        card.xmode_n = 0x80;
        let mut cards = [card];
        let mut real = 0u16;
        assert_eq!(raw_set_mode(&mut cards, 0x0114, &mut real), 0);
        assert_eq!(real, 0x0114);
        let recorded = cards[0].set_modes.borrow();
        assert_eq!(
            recorded[0],
            ModeInfo {
                mode: 0x0114,
                x: 0,
                y: 0,
                depth: 0
            }
        );
    }

    #[test]
    fn raw_set_mode_returns_minus_one_for_unknown_mode() {
        let mut cards = [TestCard::new(alloc::vec![ModeInfo {
            mode: 0x1234,
            x: 80,
            y: 25,
            depth: 0,
        }])];
        let mut real = 0u16;
        assert_eq!(raw_set_mode(&mut cards, 0xBEEF, &mut real), -1);
    }

    #[test]
    fn raw_set_mode_strips_recalc_bit_before_matching() {
        let mut cards = [TestCard::new(alloc::vec![ModeInfo {
            mode: VIDEO_80X25,
            x: 80,
            y: 25,
            depth: 0,
        }])];
        let mut real = 0u16;
        assert_eq!(
            raw_set_mode(&mut cards, VIDEO_80X25 | VIDEO_RECALC, &mut real),
            0
        );
        assert_eq!(real, VIDEO_80X25);
    }

    #[test]
    fn vga_recalc_vertical_programs_height_and_overflow() {
        // force_y=25, font=16 => rows = 25*16 - 1 = 399 = 0x18F.
        // CR12 = 0x8F. ov bits: (399>>7)&0x02 = (3)&0x02 = 0x02;
        //                       (399>>3)&0x40 = (49)&0x40 = 0x00.
        // start ov=0xff => &0xbd = 0xbd, |0x02 = 0xbf.
        let mut vga = TestVga {
            font_size: 16,
            text_rows: 0,
            crtc: 0x3d4,
            cr11: 0x80, // top bit set; recalc must clear it
            cr07: 0xff,
            writes: Vec::new(),
        };
        vga_recalc_vertical(&mut vga, 25);
        // First write unlocks CR0-7: CR11 with bit 7 cleared (0x80 & !0x80 = 0).
        assert_eq!(vga.writes[0], (0x3d4, 0x11, 0x00));
        // CR12 lower height = 0x8F.
        assert_eq!(vga.writes[1], (0x3d4, 0x12, 0x8f));
        // CR07 overflow = 0xbf.
        assert_eq!(vga.writes[2], (0x3d4, 0x07, 0xbf));
    }

    #[test]
    fn set_mode_current_mode_is_noop() {
        let mut cards = [TestCard::new(Vec::new())];
        let mut st = VideoState::default();
        let mut vga = unused_recalc_io();
        assert_eq!(
            set_mode(&mut cards, &mut st, &mut vga, VIDEO_CURRENT_MODE),
            0
        );
        assert!(cards[0].set_modes.borrow().is_empty());
    }

    #[test]
    fn set_mode_translates_normal_vga_alias_and_stores_canonical() {
        let mut cards = [TestCard::new(alloc::vec![ModeInfo {
            mode: VIDEO_80X25,
            x: 80,
            y: 25,
            depth: 0,
        }])];
        let mut st = VideoState::default();
        let mut vga = unused_recalc_io();
        assert_eq!(set_mode(&mut cards, &mut st, &mut vga, NORMAL_VGA), 0);
        // Canonical mode stored, not the NORMAL_VGA alias.
        assert_eq!(st.video_mode, VIDEO_80X25);
        assert_eq!(cards[0].set_modes.borrow()[0].mode, VIDEO_80X25);
    }

    #[test]
    fn set_mode_with_recalc_runs_vga_recalc_vertical() {
        let mut cards = [TestCard::new(alloc::vec![ModeInfo {
            mode: VIDEO_8POINT,
            x: 80,
            y: 50,
            depth: 0,
        }])];
        let mut st = VideoState {
            force_y: 50,
            ..Default::default()
        };
        let mut vga = TestVga {
            font_size: 8,
            text_rows: 0,
            crtc: 0x3d4,
            cr11: 0x80,
            cr07: 0xff,
            writes: Vec::new(),
        };
        assert_eq!(
            set_mode(&mut cards, &mut st, &mut vga, VIDEO_8POINT | VIDEO_RECALC),
            0
        );
        assert_eq!(st.video_mode, VIDEO_8POINT);
        // recalc ran: at least the three documented writes happened.
        assert_eq!(vga.writes.len(), 3);
    }

    #[test]
    fn set_mode_propagates_raw_set_mode_failure() {
        let mut cards = [TestCard::new(Vec::new())];
        let mut st = VideoState::default();
        let mut vga = unused_recalc_io();
        assert_eq!(set_mode(&mut cards, &mut st, &mut vga, 0xBEEF), -1);
    }
}
