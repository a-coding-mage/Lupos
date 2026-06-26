//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/video-vga.c
//! test-origin: linux:vendor/linux/arch/x86/boot/video-vga.c
//! Common all-VGA text-mode driver.
//!
//! Ports / mirrors (1:1, no simplification):
//! - vendor/linux/arch/x86/boot/video-vga.c
//!
//! The VGA card supplies the canonical text modes (80x25/50/43/28/30/34/60),
//! the BIOS-based adapter detection (`vga_probe`), and the register pokes that
//! reprogram the CRTC for the tall/short text modes. INT 10h calls thread the
//! [`BiosCaller`] seam; raw CRTC/Misc-Output port writes thread [`PortIoOps`].
//! `force_x`/`force_y`, `do_restore`, `adapter` and the active mode list live
//! in [`VideoState`], exactly as in the C file-scope globals.

use super::biosregs::{BiosCaller, BiosRegs};
use super::io::PortIoOps;
use super::regs::initregs;
use super::video::{
    ADAPTER_CGA, ADAPTER_EGA, ADAPTER_VGA, ModeInfo, VIDEO_8POINT, VIDEO_80X25, VIDEO_80X28,
    VIDEO_80X30, VIDEO_80X34, VIDEO_80X43, VIDEO_80X60, VideoState, out_idx,
};

// =====================================================================
// video-vga.c:17-34 — the three mode tables
// =====================================================================

/// `vga_modes[]` (video-vga.c:17-25) — full VGA text modes.
pub const VGA_MODES: [ModeInfo; 7] = [
    ModeInfo {
        mode: VIDEO_80X25,
        x: 80,
        y: 25,
        depth: 0,
    },
    ModeInfo {
        mode: VIDEO_8POINT,
        x: 80,
        y: 50,
        depth: 0,
    },
    ModeInfo {
        mode: VIDEO_80X43,
        x: 80,
        y: 43,
        depth: 0,
    },
    ModeInfo {
        mode: VIDEO_80X28,
        x: 80,
        y: 28,
        depth: 0,
    },
    ModeInfo {
        mode: VIDEO_80X30,
        x: 80,
        y: 30,
        depth: 0,
    },
    ModeInfo {
        mode: VIDEO_80X34,
        x: 80,
        y: 34,
        depth: 0,
    },
    ModeInfo {
        mode: VIDEO_80X60,
        x: 80,
        y: 60,
        depth: 0,
    },
];

/// `ega_modes[]` (video-vga.c:27-30) — EGA text modes.
pub const EGA_MODES: [ModeInfo; 2] = [
    ModeInfo {
        mode: VIDEO_80X25,
        x: 80,
        y: 25,
        depth: 0,
    },
    ModeInfo {
        mode: VIDEO_8POINT,
        x: 80,
        y: 43,
        depth: 0,
    },
];

/// `cga_modes[]` (video-vga.c:32-34) — the single CGA/MDA/HGC mode.
pub const CGA_MODES: [ModeInfo; 1] = [ModeInfo {
    mode: VIDEO_80X25,
    x: 80,
    y: 25,
    depth: 0,
}];

/// The per-adapter card names (video-vga.c:233-235).
pub const VGA_CARD_NAMES: [&str; 3] = ["CGA/MDA/HGC", "EGA", "VGA"];

// =====================================================================
// video-vga.c:39-59 — vga_set_basic_mode
// =====================================================================

/// `vga_set_basic_mode()` (video-vga.c:39-59) — query the current mode, fall
/// back to mode 3 unless it's already 3 or 7, set it, and request a restore.
/// Returns the mode that was set.
pub fn vga_set_basic_mode<B: BiosCaller>(bios: &B, st: &mut VideoState) -> u8 {
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();

    initregs(&mut ireg);

    // Query current mode.
    ireg.set_ax(0x0f00);
    bios.intcall(0x10, &ireg, Some(&mut oreg));
    let mut mode = oreg.al();

    if mode != 3 && mode != 7 {
        mode = 3;
    }

    // Set the mode. AH=0: set mode.
    ireg.set_ax(mode as u16);
    bios.intcall(0x10, &ireg, None);
    st.do_restore = 1;
    mode
}

// =====================================================================
// video-vga.c:61-110 — font programming
// =====================================================================

/// `vga_set_8font()` (video-vga.c:61-87) — set the 8x8 font (80x43 on EGA,
/// 80x50 on VGA) and reposition the cursor scan lines.
pub fn vga_set_8font<B: BiosCaller>(bios: &B) {
    let mut ireg = BiosRegs::default();
    initregs(&mut ireg);

    // Set 8x8 font (BL=0 implied).
    ireg.set_ax(0x1112);
    bios.intcall(0x10, &ireg, None);

    // Use alternate print screen.
    ireg.set_ax(0x1200);
    set_bl(&mut ireg, 0x20);
    bios.intcall(0x10, &ireg, None);

    // Turn off cursor emulation.
    ireg.set_ax(0x1201);
    set_bl(&mut ireg, 0x34);
    bios.intcall(0x10, &ireg, None);

    // Cursor is scan lines 6-7.
    ireg.set_ax(0x0100);
    set_cx(&mut ireg, 0x0607);
    bios.intcall(0x10, &ireg, None);
}

/// `vga_set_14font()` (video-vga.c:89-110) — set the 9x14 font (80x28 on VGA)
/// and reposition the cursor scan lines.
pub fn vga_set_14font<B: BiosCaller>(bios: &B) {
    let mut ireg = BiosRegs::default();
    initregs(&mut ireg);

    // Set 9x14 font (BL=0 implied).
    ireg.set_ax(0x1111);
    bios.intcall(0x10, &ireg, None);

    // Turn off cursor emulation.
    ireg.set_ax(0x1201);
    set_bl(&mut ireg, 0x34);
    bios.intcall(0x10, &ireg, None);

    // Cursor is scan lines 11-12.
    ireg.set_ax(0x0100);
    set_cx(&mut ireg, 0x0b0c);
    bios.intcall(0x10, &ireg, None);
}

/// `vga_set_80x43()` (video-vga.c:112-129) — set 80x43 on VGA (not EGA):
/// force 350-scan-line mode, reset to mode 3, then load the 8x8 font.
pub fn vga_set_80x43<B: BiosCaller>(bios: &B) {
    let mut ireg = BiosRegs::default();
    initregs(&mut ireg);

    // Set 350 scans.
    ireg.set_ax(0x1201);
    set_bl(&mut ireg, 0x30);
    bios.intcall(0x10, &ireg, None);

    // Reset video mode.
    ireg.set_ax(0x0003);
    bios.intcall(0x10, &ireg, None);

    vga_set_8font(bios);
}

// =====================================================================
// video-vga.c:132-135 — vga_crtc
// =====================================================================

/// `vga_crtc()` (video-vga.c:132-135) — return the CRTC base port: 0x3d4 for
/// color, 0x3b4 for monochrome, selected by Misc-Output-Register bit 0.
pub fn vga_crtc(io: &PortIoOps) -> u16 {
    if io.inb(0x3cc) & 1 != 0 { 0x3d4 } else { 0x3b4 }
}

// =====================================================================
// video-vga.c:137-189 — 480-scanline reprogramming and the tall modes
// =====================================================================

/// `vga_set_480_scanlines()` (video-vga.c:137-155) — reprogram the CRTC for a
/// 480-scan-line frame and select the 60 Hz timing in the Misc Output reg.
pub fn vga_set_480_scanlines(io: &PortIoOps) {
    let crtc = vga_crtc(io); // CRTC base address.

    out_idx(io, 0x0c, crtc, 0x11); // Vertical sync end, unlock CR0-7.
    out_idx(io, 0x0b, crtc, 0x06); // Vertical total.
    out_idx(io, 0x3e, crtc, 0x07); // Vertical overflow.
    out_idx(io, 0xea, crtc, 0x10); // Vertical sync start.
    out_idx(io, 0xdf, crtc, 0x12); // Vertical display end.
    out_idx(io, 0xe7, crtc, 0x15); // Vertical blank start.
    out_idx(io, 0x04, crtc, 0x16); // Vertical blank end.

    let mut csel = io.inb(0x3cc); // CRTC miscellaneous output register.
    csel &= 0x0d;
    csel |= 0xe2;
    io.outb(csel, 0x3c2);
}

/// `vga_set_vertical_end(lines)` (video-vga.c:157-169) — program the vertical
/// display-end register (and its overflow bits) for `lines` scan lines.
pub fn vga_set_vertical_end(io: &PortIoOps, lines: i32) {
    let end = lines - 1;

    let crtc = vga_crtc(io); // CRTC base address.

    // CRTC overflow register: keep the fixed 0x3c base plus overflow bits 8/9
    // of `end` re-distributed into bit 1 (0x02) and bit 6 (0x40).
    let ovfw: u8 = (0x3c | ((end >> (8 - 1)) & 0x02) | ((end >> (9 - 6)) & 0x40)) as u8;

    out_idx(io, ovfw, crtc, 0x07); // Vertical overflow.
    out_idx(io, end as u8, crtc, 0x12); // Vertical display end.
}

/// `vga_set_80x30()` (video-vga.c:171-175).
pub fn vga_set_80x30(io: &PortIoOps) {
    vga_set_480_scanlines(io);
    vga_set_vertical_end(io, 30 * 16);
}

/// `vga_set_80x34()` (video-vga.c:177-182).
pub fn vga_set_80x34<B: BiosCaller>(bios: &B, io: &PortIoOps) {
    vga_set_480_scanlines(io);
    vga_set_14font(bios);
    vga_set_vertical_end(io, 34 * 14);
}

/// `vga_set_80x60()` (video-vga.c:184-189).
pub fn vga_set_80x60<B: BiosCaller>(bios: &B, io: &PortIoOps) {
    vga_set_480_scanlines(io);
    vga_set_8font(bios);
    vga_set_vertical_end(io, 60 * 8);
}

// =====================================================================
// video-vga.c:191-224 — vga_set_mode
// =====================================================================

/// `vga_set_mode(mode)` (video-vga.c:191-224) — set the basic mode, override
/// any broken BIOS rows/cols via `force_x`/`force_y`, then dispatch to the
/// per-mode register/font programming. Returns 0 (Linux always does).
pub fn vga_set_mode<B: BiosCaller>(
    bios: &B,
    io: &PortIoOps,
    st: &mut VideoState,
    mode: &ModeInfo,
) -> i32 {
    // Set the basic mode.
    vga_set_basic_mode(bios, st);

    // Override a possibly broken BIOS.
    st.force_x = mode.x as i32;
    st.force_y = mode.y as i32;

    match mode.mode {
        VIDEO_80X25 => {}
        VIDEO_8POINT => vga_set_8font(bios),
        VIDEO_80X43 => vga_set_80x43(bios),
        VIDEO_80X28 => vga_set_14font(bios),
        VIDEO_80X30 => vga_set_80x30(io),
        VIDEO_80X34 => vga_set_80x34(bios, io),
        VIDEO_80X60 => vga_set_80x60(bios, io),
        _ => {}
    }

    0
}

// =====================================================================
// video-vga.c:226-280 — vga_probe
// =====================================================================

/// Which mode table `vga_probe` selected, and how many entries it holds. The
/// C code stores the table pointer/name into the static `video_vga`; the Rust
/// port returns the resolved table so the caller can install it on its card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VgaModeTable {
    Cga,
    Ega,
    Vga,
}

impl VgaModeTable {
    /// The mode list (`video_vga.modes = mode_lists[adapter]`).
    pub fn modes(self) -> &'static [ModeInfo] {
        match self {
            VgaModeTable::Cga => &CGA_MODES,
            VgaModeTable::Ega => &EGA_MODES,
            VgaModeTable::Vga => &VGA_MODES,
        }
    }
    /// The card name (`video_vga.card_name = card_name[adapter]`).
    pub fn card_name(self) -> &'static str {
        match self {
            VgaModeTable::Cga => VGA_CARD_NAMES[0],
            VgaModeTable::Ega => VGA_CARD_NAMES[1],
            VgaModeTable::Vga => VGA_CARD_NAMES[2],
        }
    }
}

/// `vga_probe()` (video-vga.c:231-280) — BIOS-based adapter detection.
///
/// INT 10h AX=1200 BL=10 (EGA/VGA info): if BL comes back unchanged at 0x10 we
/// have MDA/CGA/HGC; otherwise AX=1A00 (display-combination code) distinguishes
/// VGA (AL==0x1a) from EGA. Records `orig_video_ega_bx`/`orig_video_isVGA`,
/// sets `st.adapter`, and returns the table + mode count.
pub fn vga_probe<B: BiosCaller>(bios: &B, st: &mut VideoState) -> (VgaModeTable, i32) {
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();

    initregs(&mut ireg);

    ireg.set_ax(0x1200);
    set_bl(&mut ireg, 0x10); // Check EGA/VGA.
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    st.screen_info.orig_video_ega_bx = oreg.bx();

    // If we have MDA/CGA/HGC then BL will be unchanged at 0x10.
    if bl(&oreg) != 0x10 {
        // EGA/VGA.
        ireg.set_ax(0x1a00);
        bios.intcall(0x10, &ireg, Some(&mut oreg));

        if oreg.al() == 0x1a {
            st.adapter = ADAPTER_VGA;
            st.screen_info.orig_video_isvga = 1;
        } else {
            st.adapter = ADAPTER_EGA;
        }
    } else {
        st.adapter = ADAPTER_CGA;
    }

    let table = match st.adapter {
        ADAPTER_VGA => VgaModeTable::Vga,
        ADAPTER_EGA => VgaModeTable::Ega,
        _ => VgaModeTable::Cga,
    };
    (table, table.modes().len() as i32)
}

// --- BiosRegs byte/word accessors not provided by biosregs.rs ---------

#[inline]
fn set_bl(r: &mut BiosRegs, v: u8) {
    r.ebx = (r.ebx & 0xffff_ff00) | v as u32;
}
#[inline]
fn set_cx(r: &mut BiosRegs, v: u16) {
    r.ecx = (r.ecx & 0xffff_0000) | v as u32;
}
#[inline]
fn bl(r: &BiosRegs) -> u8 {
    r.ebx as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;
    use core::cell::RefCell;
    use core::sync::atomic::{AtomicU32, Ordering};

    // ---- mode tables (video-vga.c:17-34) -----------------------------

    #[test]
    fn vga_modes_table_matches_video_vga_c() {
        assert_eq!(VGA_MODES.len(), 7);
        assert_eq!(
            VGA_MODES[0],
            ModeInfo {
                mode: VIDEO_80X25,
                x: 80,
                y: 25,
                depth: 0
            }
        );
        assert_eq!(
            VGA_MODES[1],
            ModeInfo {
                mode: VIDEO_8POINT,
                x: 80,
                y: 50,
                depth: 0
            }
        );
        assert_eq!(
            VGA_MODES[2],
            ModeInfo {
                mode: VIDEO_80X43,
                x: 80,
                y: 43,
                depth: 0
            }
        );
        assert_eq!(
            VGA_MODES[3],
            ModeInfo {
                mode: VIDEO_80X28,
                x: 80,
                y: 28,
                depth: 0
            }
        );
        assert_eq!(
            VGA_MODES[4],
            ModeInfo {
                mode: VIDEO_80X30,
                x: 80,
                y: 30,
                depth: 0
            }
        );
        assert_eq!(
            VGA_MODES[5],
            ModeInfo {
                mode: VIDEO_80X34,
                x: 80,
                y: 34,
                depth: 0
            }
        );
        assert_eq!(
            VGA_MODES[6],
            ModeInfo {
                mode: VIDEO_80X60,
                x: 80,
                y: 60,
                depth: 0
            }
        );
    }

    #[test]
    fn ega_modes_table_matches_video_vga_c() {
        // EGA: 80x25 and an 80x43 entry tagged with VIDEO_8POINT.
        assert_eq!(EGA_MODES.len(), 2);
        assert_eq!(
            EGA_MODES[0],
            ModeInfo {
                mode: VIDEO_80X25,
                x: 80,
                y: 25,
                depth: 0
            }
        );
        assert_eq!(
            EGA_MODES[1],
            ModeInfo {
                mode: VIDEO_8POINT,
                x: 80,
                y: 43,
                depth: 0
            }
        );
    }

    #[test]
    fn cga_modes_table_has_single_80x25_entry() {
        assert_eq!(CGA_MODES.len(), 1);
        assert_eq!(
            CGA_MODES[0],
            ModeInfo {
                mode: VIDEO_80X25,
                x: 80,
                y: 25,
                depth: 0
            }
        );
    }

    // ---- recording BIOS ----------------------------------------------

    struct RecBios {
        // (int_no, ax, bx, cx) snapshot of each call.
        calls: RefCell<Vec<(u8, u16, u16, u16)>>,
        // Scripted replies popped on each oreg-returning call.
        replies: RefCell<alloc::collections::VecDeque<BiosRegs>>,
    }
    impl RecBios {
        fn new() -> Self {
            RecBios {
                calls: RefCell::new(Vec::new()),
                replies: RefCell::new(alloc::collections::VecDeque::new()),
            }
        }
        fn reply_ax(&self, ax: u16) {
            let mut r = BiosRegs::default();
            r.set_ax(ax);
            self.replies.borrow_mut().push_back(r);
        }
        fn reply(&self, r: BiosRegs) {
            self.replies.borrow_mut().push_back(r);
        }
    }
    impl BiosCaller for RecBios {
        fn intcall(&self, int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            self.calls
                .borrow_mut()
                .push((int_no, ireg.ax(), ireg.bx(), ireg.cx()));
            if let Some(o) = oreg {
                *o = self.replies.borrow_mut().pop_front().unwrap_or_default();
            }
        }
    }

    #[test]
    fn vga_set_basic_mode_keeps_mode_7_and_requests_restore() {
        // Query reply AL=7 => keep mode 7.
        let bios = RecBios::new();
        bios.reply_ax(0x0007);
        let mut st = VideoState::default();
        let mode = vga_set_basic_mode(&bios, &mut st);
        assert_eq!(mode, 7);
        assert_eq!(st.do_restore, 1);
        // Two calls: query (AX=0F00), then set (AX=0007).
        let calls = bios.calls.borrow();
        assert_eq!(calls[0].1, 0x0f00);
        assert_eq!(calls[1].1, 0x0007);
    }

    #[test]
    fn vga_set_basic_mode_falls_back_to_mode_3() {
        // Query reply AL=0x13 (graphics) => fall back to mode 3.
        let bios = RecBios::new();
        bios.reply_ax(0x0013);
        let mut st = VideoState::default();
        let mode = vga_set_basic_mode(&bios, &mut st);
        assert_eq!(mode, 3);
        assert_eq!(bios.calls.borrow()[1].1, 0x0003);
    }

    #[test]
    fn vga_set_8font_issues_exact_int10_sequence() {
        // video-vga.c:61-87 — AX/BL/CX of each of the four INT 10h calls.
        let bios = RecBios::new();
        vga_set_8font(&bios);
        let calls = bios.calls.borrow();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].1, 0x1112); // set 8x8 font
        assert_eq!(calls[1].1, 0x1200); // alt print screen
        assert_eq!(calls[1].2 & 0xff, 0x20); // BL=0x20
        assert_eq!(calls[2].1, 0x1201); // cursor emulation off
        assert_eq!(calls[2].2 & 0xff, 0x34); // BL=0x34
        assert_eq!(calls[3].1, 0x0100); // set cursor shape
        assert_eq!(calls[3].3, 0x0607); // CX=0x0607
    }

    #[test]
    fn vga_set_14font_issues_exact_int10_sequence() {
        let bios = RecBios::new();
        vga_set_14font(&bios);
        let calls = bios.calls.borrow();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1, 0x1111); // set 9x14 font
        assert_eq!(calls[1].1, 0x1201);
        assert_eq!(calls[1].2 & 0xff, 0x34);
        assert_eq!(calls[2].1, 0x0100);
        assert_eq!(calls[2].3, 0x0b0c); // cursor scan lines 11-12
    }

    #[test]
    fn vga_set_80x43_sets_350_scans_resets_then_8font() {
        let bios = RecBios::new();
        vga_set_80x43(&bios);
        let calls = bios.calls.borrow();
        // 0x1201 (BL=0x30), 0x0003, then 4 font calls.
        assert_eq!(calls[0].1, 0x1201);
        assert_eq!(calls[0].2 & 0xff, 0x30);
        assert_eq!(calls[1].1, 0x0003);
        assert_eq!(calls.len(), 6);
    }

    // ---- port-I/O backed register pokes ------------------------------

    static INB_3CC: AtomicU32 = AtomicU32::new(0);
    static OUTW_LOG_IDX: AtomicU32 = AtomicU32::new(0);

    // Record up to 16 outw calls as (port:16 | word:16) -> but words can be
    // 32-bit packed, so store separately keyed by an incrementing index.
    static OUTW_WORDS: [AtomicU32; 16] = [
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
        AtomicU32::new(0),
    ];
    static LAST_OUTB: AtomicU32 = AtomicU32::new(0); // (val<<16)|port

    fn p_inb(port: u16) -> u8 {
        if port == 0x3cc {
            INB_3CC.load(Ordering::Relaxed) as u8
        } else {
            0
        }
    }
    fn p_outb(v: u8, port: u16) {
        LAST_OUTB.store(((v as u32) << 16) | port as u32, Ordering::Relaxed);
    }
    fn p_outw(v: u16, port: u16) {
        let i = OUTW_LOG_IDX.fetch_add(1, Ordering::Relaxed) as usize;
        if i < OUTW_WORDS.len() {
            OUTW_WORDS[i].store(((v as u32) << 16) | port as u32, Ordering::Relaxed);
        }
    }
    fn reset_io_log() {
        OUTW_LOG_IDX.store(0, Ordering::Relaxed);
        for w in &OUTW_WORDS {
            w.store(0, Ordering::Relaxed);
        }
    }
    fn io_ops() -> PortIoOps {
        PortIoOps {
            f_inb: p_inb,
            f_outb: p_outb,
            f_outw: p_outw,
        }
    }

    #[test]
    fn vga_crtc_selects_color_or_mono_from_misc_output() {
        INB_3CC.store(0x01, Ordering::Relaxed);
        assert_eq!(vga_crtc(&io_ops()), 0x3d4);
        INB_3CC.store(0x00, Ordering::Relaxed);
        assert_eq!(vga_crtc(&io_ops()), 0x3b4);
    }

    #[test]
    fn vga_set_480_scanlines_writes_documented_crtc_values() {
        // Color CRTC.
        INB_3CC.store(0x01, Ordering::Relaxed);
        reset_io_log();
        let io = io_ops();
        vga_set_480_scanlines(&io);
        // First out_idx: out_idx(0x0c, 0x3d4, 0x11) => word = (0x0c<<8)|0x11.
        let w0 = OUTW_WORDS[0].load(Ordering::Relaxed);
        assert_eq!(w0 & 0xffff, 0x3d4);
        assert_eq!((w0 >> 16) as u16, (0x0c << 8) | 0x11);
        // Last out_idx (index 0x16, value 0x04).
        let w6 = OUTW_WORDS[6].load(Ordering::Relaxed);
        assert_eq!((w6 >> 16) as u16, (0x04 << 8) | 0x16);
        // Misc output write to 0x3c2: csel = (0x01 & 0x0d) | 0xe2 = 0xe3.
        let outb = LAST_OUTB.load(Ordering::Relaxed);
        assert_eq!(outb & 0xffff, 0x3c2);
        assert_eq!((outb >> 16) as u8, 0xe3);
    }

    #[test]
    fn vga_set_vertical_end_computes_overflow_bits() {
        // For 80x30: 30*16 = 480 scan lines => end = 479 = 0x1DF.
        // ovfw = 0x3c | ((479>>7)&0x02) | ((479>>3)&0x40)
        //      = 0x3c | (3 & 0x02) | (59 & 0x40) = 0x3c | 0x02 = 0x3e.
        INB_3CC.store(0x01, Ordering::Relaxed);
        reset_io_log();
        let io = io_ops();
        vga_set_vertical_end(&io, 30 * 16);
        // out_idx(ovfw, crtc, 0x07) then out_idx(end as u8, crtc, 0x12).
        let w0 = OUTW_WORDS[0].load(Ordering::Relaxed);
        assert_eq!((w0 >> 16) as u16, (0x3e << 8) | 0x07);
        let w1 = OUTW_WORDS[1].load(Ordering::Relaxed);
        // end & 0xff = 0xDF.
        assert_eq!((w1 >> 16) as u16, (0xdf << 8) | 0x12);
    }

    // ---- vga_set_mode dispatch ---------------------------------------

    #[test]
    fn vga_set_mode_overrides_force_xy_and_sets_8font_for_8point() {
        let bios = RecBios::new();
        bios.reply_ax(0x0003); // basic-mode query
        let io = io_ops();
        let mut st = VideoState::default();
        let mi = ModeInfo {
            mode: VIDEO_8POINT,
            x: 80,
            y: 50,
            depth: 0,
        };
        assert_eq!(vga_set_mode(&bios, &io, &mut st, &mi), 0);
        assert_eq!(st.force_x, 80);
        assert_eq!(st.force_y, 50);
        // 8font path issues the 0x1112 set-8x8-font call somewhere.
        assert!(bios.calls.borrow().iter().any(|c| c.1 == 0x1112));
    }

    #[test]
    fn vga_set_mode_80x25_does_no_extra_font_work() {
        let bios = RecBios::new();
        bios.reply_ax(0x0003);
        let io = io_ops();
        let mut st = VideoState::default();
        let mi = ModeInfo {
            mode: VIDEO_80X25,
            x: 80,
            y: 25,
            depth: 0,
        };
        vga_set_mode(&bios, &io, &mut st, &mi);
        // Only the two vga_set_basic_mode calls (query + set), no font calls.
        assert_eq!(bios.calls.borrow().len(), 2);
    }

    // ---- vga_probe adapter detection ---------------------------------

    #[test]
    fn vga_probe_detects_vga_via_display_combination_code() {
        let bios = RecBios::new();
        // First call AX=1200 BL=10: BL must change away from 0x10 to be EGA/VGA.
        let mut r1 = BiosRegs::default();
        r1.ebx = 0x0003; // bl != 0x10
        bios.reply(r1);
        // Second call AX=1A00: AL=0x1a => VGA.
        let mut r2 = BiosRegs::default();
        r2.eax = 0x001a;
        bios.reply(r2);

        let mut st = VideoState::default();
        let (table, n) = vga_probe(&bios, &mut st);
        assert_eq!(st.adapter, ADAPTER_VGA);
        assert_eq!(st.screen_info.orig_video_isvga, 1);
        assert_eq!(st.screen_info.orig_video_ega_bx, 0x0003);
        assert_eq!(table, VgaModeTable::Vga);
        assert_eq!(n, VGA_MODES.len() as i32);
    }

    #[test]
    fn vga_probe_detects_ega_when_dcc_not_1a() {
        let bios = RecBios::new();
        let mut r1 = BiosRegs::default();
        r1.ebx = 0x0007;
        bios.reply(r1);
        let mut r2 = BiosRegs::default();
        r2.eax = 0x0000; // not 0x1a => EGA
        bios.reply(r2);

        let mut st = VideoState::default();
        let (table, n) = vga_probe(&bios, &mut st);
        assert_eq!(st.adapter, ADAPTER_EGA);
        assert_eq!(st.screen_info.orig_video_isvga, 0);
        assert_eq!(table, VgaModeTable::Ega);
        assert_eq!(n, EGA_MODES.len() as i32);
    }

    #[test]
    fn vga_probe_detects_cga_when_bl_unchanged() {
        let bios = RecBios::new();
        // BL stays 0x10 => MDA/CGA/HGC. Only ONE reply consumed.
        let mut r1 = BiosRegs::default();
        r1.ebx = 0x0010;
        bios.reply(r1);

        let mut st = VideoState::default();
        let (table, n) = vga_probe(&bios, &mut st);
        assert_eq!(st.adapter, ADAPTER_CGA);
        assert_eq!(table, VgaModeTable::Cga);
        assert_eq!(n, 1);
        // Only the single AX=1200 probe was issued.
        assert_eq!(bios.calls.borrow().len(), 1);
    }

    #[test]
    fn vga_mode_table_exposes_correct_names_and_lists() {
        assert_eq!(VgaModeTable::Cga.card_name(), "CGA/MDA/HGC");
        assert_eq!(VgaModeTable::Ega.card_name(), "EGA");
        assert_eq!(VgaModeTable::Vga.card_name(), "VGA");
        assert_eq!(VgaModeTable::Vga.modes().len(), 7);
    }
}
