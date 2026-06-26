//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/video.c
//! linux-source: vendor/linux/arch/x86/boot/video.h
//! test-origin: linux:vendor/linux/arch/x86/boot/video.c
//! Real-mode video-mode probing and selection.
//!
//! Ports / mirrors (1:1, no simplification):
//! - vendor/linux/arch/x86/boot/video.h  (mode-id constants, `struct mode_info`,
//!   `struct card_info`, `ADAPTER_*`, the indexed-register helpers
//!   `in_idx`/`out_idx`/`tst_idx`)
//! - vendor/linux/arch/x86/boot/video.c  (`set_video` and all of its helpers)
//!
//! Lupos' current bzImage handoff enters protected mode directly, so this
//! real-mode setup video path is kept as a behaviourally faithful Rust twin
//! of the Linux setup logic, threading the existing seams:
//!   * BIOS INT 10h calls go through [`BiosCaller`] (biosregs.rs).
//!   * Port I/O goes through [`PortIoOps`] (io.rs).
//!   * The real-mode `set_fs`/`rdfs*`/`copy_*_fs`/heap accesses go through the
//!     [`VideoMem`] seam.
//!
//! The `screen_info` subset and the C file-scope globals (`adapter`, `force_x`,
//! `force_y`, `do_restore`, `graphic_mode`, `video_segment`, the saved-screen
//! state) are carried in [`VideoState`], which is the genuine Linux state — the
//! shared `BootParams` blob in this tree does not expose the `orig_video_*`
//! fields the setup stub writes, so they live here exactly as in Linux.

use super::biosregs::{BiosCaller, BiosRegs};
use super::io::PortIoOps;
use super::regs::initregs;

// =====================================================================
// video.h: extended video-mode numbers
// =====================================================================

/// `VIDEO_FIRST_MENU` — modes numbered by menu position (0x00..0xff).
pub const VIDEO_FIRST_MENU: u16 = 0x0000;

/// `VIDEO_FIRST_BIOS` — standard BIOS modes (BIOS number + 0x0100).
pub const VIDEO_FIRST_BIOS: u16 = 0x0100;

/// `VIDEO_FIRST_VESA` — VESA BIOS modes (VESA number + 0x0200).
pub const VIDEO_FIRST_VESA: u16 = 0x0200;

/// `VIDEO_FIRST_V7` — Video7 special modes (BIOS number + 0x0900).
pub const VIDEO_FIRST_V7: u16 = 0x0900;

/// `VIDEO_FIRST_SPECIAL` — base of the special-mode range.
pub const VIDEO_FIRST_SPECIAL: u16 = 0x0f00;
pub const VIDEO_80X25: u16 = 0x0f00;
pub const VIDEO_8POINT: u16 = 0x0f01;
pub const VIDEO_80X43: u16 = 0x0f02;
pub const VIDEO_80X28: u16 = 0x0f03;
pub const VIDEO_CURRENT_MODE: u16 = 0x0f04;
pub const VIDEO_80X30: u16 = 0x0f05;
pub const VIDEO_80X34: u16 = 0x0f06;
pub const VIDEO_80X60: u16 = 0x0f07;
pub const VIDEO_GFX_HACK: u16 = 0x0f08;
pub const VIDEO_LAST_SPECIAL: u16 = 0x0f09;

/// `VIDEO_FIRST_RESOLUTION` — modes given by resolution.
pub const VIDEO_FIRST_RESOLUTION: u16 = 0x1000;

/// `VIDEO_RECALC` — the "recalculate timings" flag.
pub const VIDEO_RECALC: u16 = 0x8000;

// uapi/asm/boot.h: internal svga startup constants.
/// `NORMAL_VGA` — request the 80x25 mode.
pub const NORMAL_VGA: u16 = 0xffff;
/// `EXTENDED_VGA` — request the 80x50 mode.
pub const EXTENDED_VGA: u16 = 0xfffe;
/// `ASK_VGA` — ask for the mode at boot via the menu.
pub const ASK_VGA: u16 = 0xfffd;

// video.h: basic video adapter type.
/// `ADAPTER_CGA` — CGA/MDA/HGC.
pub const ADAPTER_CGA: i32 = 0;
/// `ADAPTER_EGA`.
pub const ADAPTER_EGA: i32 = 1;
/// `ADAPTER_VGA`.
pub const ADAPTER_VGA: i32 = 2;

// uapi/linux/screen_info.h: video type / flags used by the setup stub.
/// `VIDEO_TYPE_VLFB` — VESA VGA in graphic mode.
pub const VIDEO_TYPE_VLFB: u8 = 0x23;
/// `VIDEO_FLAGS_NOCURSOR` — the video mode has no cursor set.
pub const VIDEO_FLAGS_NOCURSOR: u8 = 1 << 0;

// =====================================================================
// video.h: struct mode_info
// =====================================================================

/// `struct mode_info` — one entry in a card's mode table (video.h:64-68).
///
/// Field order and widths match the C struct exactly so setup/bzImage interop
/// tests can `transmute` between this and the C definition.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct ModeInfo {
    /// Mode number (vga= style).
    pub mode: u16,
    /// Width.
    pub x: u16,
    /// Height.
    pub y: u16,
    /// Bits per pixel, 0 for text mode.
    pub depth: u16,
}

// =====================================================================
// video.h: indexed VGA register helpers (in_idx / out_idx / tst_idx)
// =====================================================================

/// `in_idx(port, index)` — write the index then read the data port+1
/// (video.h:97-101).
#[inline]
pub fn in_idx(io: &PortIoOps, port: u16, index: u8) -> u8 {
    io.outb(index, port);
    io.inb(port + 1)
}

/// `out_idx(v, port, index)` — write `index + (v << 8)` as a word so the
/// index/data pair is programmed atomically (video.h:103-106).
#[inline]
pub fn out_idx(io: &PortIoOps, v: u8, port: u16, index: u8) {
    io.outw((index as u16) + ((v as u16) << 8), port);
}

/// `tst_idx(v, port, index)` — write a value to an indexed register and then
/// read it back (video.h:109-113).
///
/// Note: the C body passes its arguments through in the original (and buggy)
/// `out_idx(port, index, v)` / `in_idx(port, index)` order; this port mirrors
/// that exact call shape so the observable behaviour is identical.
#[inline]
pub fn tst_idx(io: &PortIoOps, v: u8, port: u16, index: u8) -> u8 {
    out_idx(io, port as u8, index as u16, v);
    in_idx(io, port, index)
}

// =====================================================================
// video.h: struct screen_info subset written by the setup stub
// =====================================================================

/// The subset of `struct screen_info` (uapi/linux/screen_info.h) that the
/// real-mode video code reads and writes. Carried here because the shared
/// `BootParams` blob in this tree does not expose these `orig_video_*` /
/// VESA fields; the values and their meanings match Linux exactly.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct ScreenInfo {
    pub orig_x: u8,
    pub orig_y: u8,
    pub orig_video_page: u16,
    pub orig_video_mode: u8,
    pub orig_video_cols: u8,
    pub flags: u8,
    pub orig_video_ega_bx: u16,
    pub orig_video_lines: u8,
    pub orig_video_isvga: u8,
    pub orig_video_points: u16,

    // VESA graphic-mode (linear frame buffer) fields.
    pub lfb_width: u16,
    pub lfb_height: u16,
    pub lfb_depth: u16,
    pub lfb_base: u32,
    pub lfb_size: u32,
    pub lfb_linelength: u16,
    pub red_size: u8,
    pub red_pos: u8,
    pub green_size: u8,
    pub green_pos: u8,
    pub blue_size: u8,
    pub blue_pos: u8,
    pub rsvd_size: u8,
    pub rsvd_pos: u8,
    pub vesapm_seg: u16,
    pub vesapm_off: u16,
    pub pages: u16,
    pub vesa_attributes: u16,
}

// =====================================================================
// video-mode.c: common file-scope globals + screen_info + heap
// =====================================================================

/// One saved screen image (video.c:233-237 `struct saved_screen`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SavedScreen {
    pub x: i32,
    pub y: i32,
    pub curx: i32,
    pub cury: i32,
    /// `u16 *data` — the saved character/attribute cells. `None` means the
    /// heap could not hold the screen image (Linux leaves `data` NULL).
    pub data: Option<alloc::vec::Vec<u16>>,
}

/// Aggregated mutable state for the setup video code: the `screen_info`
/// subset, the video-mode.c globals, `video_segment`, and the saved screen.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VideoState {
    pub screen_info: ScreenInfo,
    /// `int adapter` — 0=CGA/MDA/HGC, 1=EGA, 2=VGA+ (video-mode.c:25).
    pub adapter: i32,
    /// `int force_x` — override BIOS column count (video-mode.c:26).
    pub force_x: i32,
    /// `int force_y` — override BIOS row count (video-mode.c:26).
    pub force_y: i32,
    /// `int do_restore` — screen contents changed during the mode flip.
    pub do_restore: i32,
    /// `int graphic_mode` — graphic mode with a linear frame buffer.
    pub graphic_mode: i32,
    /// `static u16 video_segment` (video.c:20).
    pub video_segment: u16,
    /// `static struct saved_screen saved` (video.c:237).
    pub saved: SavedScreen,
    /// `boot_params.hdr.vid_mode` — the canonical mode number stored by
    /// `set_mode` (video-mode.c:168). The shared `BootParams` blob in this
    /// tree exposes `set_video_mode`/`video_mode`; this mirror lets the
    /// dispatcher run against either store.
    pub video_mode: u16,
}

// =====================================================================
// Real-mode memory seam (set_fs / rdfs* / copy_*_fs)
// =====================================================================

/// Seam for the real-mode far-segment memory helpers the setup stub uses
/// (`set_fs`, `rdfs8`/`rdfs16`, `copy_from_fs`, `copy_to_fs`). Lupos does not
/// run in real mode, so production wiring substitutes a no-op/identity-mapped
/// implementation and the tests use deterministic stubs. The argument and
/// return widths match the C primitives exactly.
pub trait VideoMem {
    /// `set_fs(seg)` — point FS at `seg` (boot.h:54).
    fn set_fs(&mut self, seg: u16);
    /// `rdfs8(addr)` — read a byte from `FS:addr` (boot.h:78).
    fn rdfs8(&self, addr: u32) -> u8;
    /// `rdfs16(addr)` — read a word from `FS:addr` (boot.h:85).
    fn rdfs16(&self, addr: u32) -> u16;
    /// `copy_from_fs(dst, src, len)` — copy `len` bytes from `FS:src`.
    fn copy_from_fs(&self, dst: &mut [u16], src: u32, len_bytes: usize);
    /// `copy_to_fs(dst, src, len)` — copy `len` bytes to `FS:dst`.
    fn copy_to_fs(&mut self, dst: u32, src: &[u16], len_bytes: usize);
    /// Fill `[FS:dst .. FS:dst+count]` cells with the blank cell `0x0720`,
    /// matching the inline `rep stos` blanking in `restore_screen`.
    fn blank_fs(&mut self, dst: u32, count: usize);
}

// =====================================================================
// video.h: struct card_info — the per-driver vtable
// =====================================================================

/// `struct card_info` (video.h:70-79) — a video "card" (driver). Each driver
/// supplies `set_mode`/`probe` plus the bookkeeping fields used by the
/// dispatcher in video-mode.c.
pub trait CardInfo {
    /// `const char *card_name`.
    fn card_name(&self) -> &str;
    /// `int (*set_mode)(struct mode_info *mode)` — returns 0 on success.
    fn set_mode(&mut self, mode: &ModeInfo) -> i32;
    /// `int (*probe)(void)` — populate the mode list, returns count. A driver
    /// without a probe returns 0 (handled by `probe_cards`).
    fn probe(&mut self) -> i32 {
        0
    }
    /// Number of probed modes so far (`int nmodes`).
    fn nmodes(&self) -> i32;
    /// Set `nmodes` after a probe.
    fn set_nmodes(&mut self, n: i32);
    /// Read the i-th entry of `struct mode_info *modes`.
    fn mode(&self, index: usize) -> Option<ModeInfo>;
    /// `int unsafe` — probing is unsafe; only do it after "scan".
    fn is_unsafe(&self) -> bool {
        false
    }
    /// `u16 xmode_first` — first unprobed mode to try anyway.
    fn xmode_first(&self) -> u16 {
        0
    }
    /// `u16 xmode_n` — size of the unprobed mode range.
    fn xmode_n(&self) -> u16 {
        0
    }
}

// =====================================================================
// video.c: store_cursor_position / store_video_mode / store_mode_params
// =====================================================================

/// `store_cursor_position()` (video.c:22-38) — INT 10h AH=03h Get Cursor
/// Position, then record `orig_x`/`orig_y` and the no-cursor flags.
pub fn store_cursor_position<B: BiosCaller>(bios: &B, st: &mut VideoState) {
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();
    initregs(&mut ireg);
    set_ah(&mut ireg, 0x03);
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    st.screen_info.orig_x = dl(&oreg);
    st.screen_info.orig_y = dh(&oreg);

    if ch(&oreg) & 0x20 != 0 {
        st.screen_info.flags |= VIDEO_FLAGS_NOCURSOR;
    }

    if (ch(&oreg) & 0x1f) > (cl(&oreg) & 0x1f) {
        st.screen_info.flags |= VIDEO_FLAGS_NOCURSOR;
    }
}

/// `store_video_mode()` (video.c:40-53) — INT 10h AH=0Fh Get Current Video
/// Mode; record the mode (masking the top bit) and the active page.
pub fn store_video_mode<B: BiosCaller>(bios: &B, st: &mut VideoState) {
    let mut ireg = BiosRegs::default();
    let mut oreg = BiosRegs::default();
    initregs(&mut ireg);
    set_ah(&mut ireg, 0x0f);
    bios.intcall(0x10, &ireg, Some(&mut oreg));

    // Not all BIOSes are clean with respect to the top bit.
    st.screen_info.orig_video_mode = oreg.al() & 0x7f;
    st.screen_info.orig_video_page = bh(&oreg) as u16;
}

/// `store_mode_params()` (video.c:61-96) — ask the BIOS for the cursor,
/// mode, font size and rows/cols, applying `force_x`/`force_y` overrides.
/// Returns early for graphics mode, exactly like the C code.
pub fn store_mode_params<B: BiosCaller, M: VideoMem>(bios: &B, mem: &mut M, st: &mut VideoState) {
    // For graphics mode, it is up to the mode-setting driver
    // (currently only video-vesa.c) to store the parameters.
    if st.graphic_mode != 0 {
        return;
    }

    store_cursor_position(bios, st);
    store_video_mode(bios, st);

    if st.screen_info.orig_video_mode == 0x07 {
        // MDA, HGC, or VGA in monochrome mode.
        st.video_segment = 0xb000;
    } else {
        // CGA, EGA, VGA and so forth.
        st.video_segment = 0xb800;
    }

    mem.set_fs(0);
    let font_size = mem.rdfs16(0x485); // Font size, BIOS area.
    st.screen_info.orig_video_points = font_size;

    let mut x = mem.rdfs16(0x44a) as i32;
    let mut y = if st.adapter == ADAPTER_CGA {
        25
    } else {
        mem.rdfs8(0x484) as i32 + 1
    };

    if st.force_x != 0 {
        x = st.force_x;
    }
    if st.force_y != 0 {
        y = st.force_y;
    }

    st.screen_info.orig_video_cols = x as u8;
    st.screen_info.orig_video_lines = y as u8;
}

// =====================================================================
// video.c: menu input (get_entry / display_menu / mode_menu)
// =====================================================================

/// Seam for the setup TTY (`getchar`/`getchar_timeout`/`puts`/`putchar`/
/// `kbd_flush`) used by the interactive video menu. Production builds wire it
/// to tty.c; tests drive it with scripted key input and capture the output.
pub trait VideoTty {
    fn getchar(&mut self) -> i32;
    fn getchar_timeout(&mut self) -> i32;
    fn putchar(&mut self, c: i32);
    fn puts(&mut self, s: &str);
    fn kbd_flush(&mut self);
}

/// `get_entry()` (video.c:98-135) — read a hex mode number from the keyboard,
/// honouring backspace and ignoring non-alphanumerics; an empty line means
/// `VIDEO_CURRENT_MODE`.
pub fn get_entry<T: VideoTty>(tty: &mut T) -> u32 {
    // char entry_buf[4]
    let mut entry_buf = [0u8; 4];
    let mut len: usize = 0;
    let mut key: i32;

    loop {
        key = tty.getchar();

        if key == '\u{8}' as i32 {
            // '\b'
            if len > 0 {
                tty.puts("\u{8} \u{8}");
                len -= 1;
            }
        } else if (key >= '0' as i32 && key <= '9' as i32)
            || (key >= 'A' as i32 && key <= 'Z' as i32)
            || (key >= 'a' as i32 && key <= 'z' as i32)
        {
            if len < entry_buf.len() {
                entry_buf[len] = key as u8;
                len += 1;
                tty.putchar(key);
            }
        }

        if key == '\r' as i32 {
            break;
        }
    }
    tty.putchar('\n' as i32);

    if len == 0 {
        return VIDEO_CURRENT_MODE as u32; // Default.
    }

    let mut v: u32 = 0;
    for &b in entry_buf.iter().take(len) {
        v <<= 4;
        let key = (b | 0x20) as i32; // lower-case
        v += if key > '9' as i32 {
            (key - 'a' as i32 + 10) as u32
        } else {
            (key - '0' as i32) as u32
        };
    }

    v
}

/// `H(x)` helper (video.c:197).
const fn h(x: u8) -> u32 {
    (x - b'a' + 10) as u32
}

/// `SCAN` token (video.c:198) — the value `get_entry` returns for "scan".
pub const SCAN: u32 = (h(b's') << 12) + (h(b'c') << 8) + (h(b'a') << 4) + h(b'n');

/// `display_menu()` (video.c:137-195) — print the table of available modes
/// across the supplied cards.
pub fn display_menu<C: CardInfo, T: VideoTty>(cards: &mut [C], tty: &mut T) {
    let mut nmodes = 0i32;
    for card in cards.iter() {
        nmodes += card.nmodes();
    }

    let mut modes_per_line = 1;
    if nmodes >= 20 {
        modes_per_line = 3;
    }

    for _ in 0..modes_per_line {
        tty.puts("Mode: Resolution:  Type: ");
    }
    tty.putchar('\n' as i32);

    let mut col = 0;
    let mut ch: u8 = b'0';
    for card in cards.iter() {
        for i in 0..card.nmodes() as usize {
            let Some(mi) = card.mode(i) else {
                continue;
            };
            let visible = mi.x != 0 && mi.y != 0;
            let mode_id = if mi.mode != 0 {
                mi.mode
            } else {
                (mi.y << 8).wrapping_add(mi.x)
            };

            if !visible {
                continue; // Hidden mode.
            }

            // resbuf := "%dx%d" (y, depth) when depth!=0 else "%d" (y).
            let mut resbuf = [0u8; 32];
            let res = if mi.depth != 0 {
                fmt_pair(&mut resbuf, mi.y, mi.depth)
            } else {
                fmt_one(&mut resbuf, mi.y)
            };

            // printf("%c %03X %4dx%-7s %-6s", ch, mode_id, x, resbuf, name)
            print_menu_row(tty, ch, mode_id, mi.x, res, card.card_name());
            col += 1;
            if col >= modes_per_line {
                tty.putchar('\n' as i32);
                col = 0;
            }

            if ch == b'9' {
                ch = b'a';
            } else if ch == b'z' || ch == b' ' {
                ch = b' '; // Out of keys...
            } else {
                ch += 1;
            }
        }
    }
    if col != 0 {
        tty.putchar('\n' as i32);
    }
}

/// `mode_menu()` (video.c:200-230) — prompt, optionally display the menu, and
/// loop on "scan" requests. The `probe_cards(1)` rescans go through the
/// supplied closure so the dispatcher state stays with the caller.
pub fn mode_menu<C: CardInfo, T: VideoTty, F: FnMut(&mut [C])>(
    cards: &mut [C],
    tty: &mut T,
    mut probe_scan: F,
) -> u32 {
    tty.puts(
        "Press <ENTER> to see video modes available, \
         <SPACE> to continue, or wait 30 sec\n",
    );

    tty.kbd_flush();
    loop {
        let key = tty.getchar_timeout();
        if key == ' ' as i32 || key == 0 {
            return VIDEO_CURRENT_MODE as u32; // Default.
        }
        if key == '\r' as i32 {
            break;
        }
        tty.putchar('\u{7}' as i32); // Beep!
    }

    loop {
        display_menu(cards, tty);

        tty.puts("Enter a video mode or \"scan\" to scan for additional modes: ");
        let sel = get_entry(tty);
        if sel != SCAN {
            return sel;
        }

        probe_scan(cards);
    }
}

// =====================================================================
// video.c: save_screen / restore_screen
// =====================================================================

/// `save_screen()` (video.c:239-254) — snapshot the current text screen onto
/// the heap. `heap_free` is approximated by the `Vec` allocator: if it fails
/// the saved data stays `None`, exactly like Linux's "not enough heap" path.
pub fn save_screen<M: VideoMem>(mem: &mut M, st: &mut VideoState) {
    // Should be called after store_mode_params().
    st.saved.x = st.screen_info.orig_video_cols as i32;
    st.saved.y = st.screen_info.orig_video_lines as i32;
    st.saved.curx = st.screen_info.orig_x as i32;
    st.saved.cury = st.screen_info.orig_y as i32;

    let cells = (st.saved.x * st.saved.y) as usize;
    let mut data = alloc::vec::Vec::new();
    if data.try_reserve_exact(cells).is_err() {
        st.saved.data = None; // Not enough heap to save the screen.
        return;
    }
    data.resize(cells, 0u16);

    mem.set_fs(st.video_segment);
    mem.copy_from_fs(&mut data, 0, cells * core::mem::size_of::<u16>());
    st.saved.data = Some(data);
}

/// `restore_screen()` (video.c:256-315) — paint the saved image back, padding
/// short rows with blanks, then restore the cursor position.
pub fn restore_screen<B: BiosCaller, M: VideoMem>(bios: &B, mem: &mut M, st: &mut VideoState) {
    // Should be called after store_mode_params().
    let xs = st.screen_info.orig_video_cols as i32;
    let ys = st.screen_info.orig_video_lines as i32;

    if st.graphic_mode != 0 {
        return; // Can't restore onto a graphic mode.
    }

    let Some(saved_data) = st.saved.data.clone() else {
        return; // No saved screen contents.
    };

    // Restore screen contents.
    mem.set_fs(st.video_segment);
    let mut dst: u32 = 0;
    let mut src_idx: usize = 0; // index into saved_data, advances by saved.x
    for y in 0..ys {
        let npad;

        if y < st.saved.y {
            let copy = if xs < st.saved.x { xs } else { st.saved.x };
            let copy = copy as usize;
            mem.copy_to_fs(
                dst,
                &saved_data[src_idx..src_idx + copy],
                copy * core::mem::size_of::<u16>(),
            );
            dst += (copy * core::mem::size_of::<u16>()) as u32;
            src_idx += st.saved.x as usize;
            npad = if xs < st.saved.x { 0 } else { xs - st.saved.x };
        } else {
            npad = xs;
        }

        // Write "npad" blank characters to video_segment:dst, advance dst.
        if npad > 0 {
            mem.blank_fs(dst, npad as usize);
            dst += (npad as u32) * core::mem::size_of::<u16>() as u32;
        }
    }

    // Restore cursor position.
    if st.saved.curx >= xs {
        st.saved.curx = xs - 1;
    }
    if st.saved.cury >= ys {
        st.saved.cury = ys - 1;
    }

    let mut ireg = BiosRegs::default();
    initregs(&mut ireg);
    set_ah(&mut ireg, 0x02); // Set cursor position.
    set_dh(&mut ireg, st.saved.cury as u8);
    set_dl(&mut ireg, st.saved.curx as u8);
    bios.intcall(0x10, &ireg, None);

    store_cursor_position(bios, st);
}

// =====================================================================
// video.c: set_video — the top-level orchestrator
// =====================================================================

/// `set_video()` (video.c:317-343) — the boot-time entry point. Drives mode
/// parameter capture, the optional menu, mode selection, EDID storage and
/// optional screen restore.
///
/// The Linux dispatcher functions (`probe_cards`, `set_mode`) and the menu
/// rescans are threaded through the closures `probe`, `set_mode` and
/// `vesa_store_edid` so this file stays free of the video-mode.c internals it
/// would otherwise duplicate.
#[allow(clippy::too_many_arguments)]
pub fn set_video<C, B, M, T, P, S, E>(
    cards: &mut [C],
    bios: &B,
    mem: &mut M,
    tty: &mut T,
    st: &mut VideoState,
    mut requested_mode: u16,
    mut probe: P,
    mut set_mode: S,
    mut vesa_store_edid: E,
    mut probe_scan: impl FnMut(&mut [C]),
) -> u16
where
    C: CardInfo,
    B: BiosCaller,
    M: VideoMem,
    T: VideoTty,
    P: FnMut(&mut [C], bool),
    S: FnMut(&mut [C], u16) -> i32,
    E: FnMut(),
{
    // RESET_HEAP() — the Vec-backed mode lists own their storage, so the
    // heap reset is a no-op here.

    store_mode_params(bios, mem, st);
    save_screen(mem, st);
    probe(cards, false);

    let mut mode = requested_mode;
    loop {
        if mode == ASK_VGA {
            mode = mode_menu(cards, tty, &mut probe_scan) as u16;
        }

        if set_mode(cards, mode) == 0 {
            break;
        }

        // printf("Undefined video mode number: %x\n", mode)
        tty.puts("Undefined video mode number: ");
        put_hex(tty, mode as u32);
        tty.putchar('\n' as i32);
        mode = ASK_VGA;
    }
    requested_mode = mode;

    vesa_store_edid();
    store_mode_params(bios, mem, st);

    if st.do_restore != 0 {
        restore_screen(bios, mem, st);
    }

    requested_mode
}

// =====================================================================
// Small formatting/register helpers (faithful to the C uses above)
// =====================================================================

/// Format `"%d"` of `v` into `buf`, returning the written slice.
fn fmt_one(buf: &mut [u8; 32], v: u16) -> &str {
    let n = write_dec(buf, 0, v as u32);
    core::str::from_utf8(&buf[..n]).unwrap_or("")
}

/// Format `"%dx%d"` of `(a, b)` into `buf`, returning the written slice.
fn fmt_pair(buf: &mut [u8; 32], a: u16, b: u16) -> &str {
    let mut n = write_dec(buf, 0, a as u32);
    buf[n] = b'x';
    n += 1;
    n = write_dec(buf, n, b as u32);
    core::str::from_utf8(&buf[..n]).unwrap_or("")
}

/// Write the decimal representation of `v` into `buf` at `off`; return the new
/// length. Mirrors the `%d` conversion used by `sprintf` in display_menu.
fn write_dec(buf: &mut [u8; 32], off: usize, v: u32) -> usize {
    if v == 0 {
        buf[off] = b'0';
        return off + 1;
    }
    let mut tmp = [0u8; 10];
    let mut i = 0;
    let mut n = v;
    while n > 0 {
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    let mut o = off;
    while i > 0 {
        i -= 1;
        buf[o] = tmp[i];
        o += 1;
    }
    o
}

/// `printf("%c %03X %4dx%-7s %-6s", ch, mode_id, x, resbuf, name)`.
fn print_menu_row<T: VideoTty>(tty: &mut T, ch: u8, mode_id: u16, x: u16, res: &str, name: &str) {
    tty.putchar(ch as i32);
    tty.putchar(' ' as i32);
    put_hex_width(tty, mode_id as u32, 3);
    tty.putchar(' ' as i32);
    put_dec_width(tty, x as u32, 4);
    tty.putchar('x' as i32);
    puts_left(tty, res, 7);
    tty.putchar(' ' as i32);
    puts_left(tty, name, 6);
}

/// Emit `s` left-justified in a field of `width` (`%-Ns`).
fn puts_left<T: VideoTty>(tty: &mut T, s: &str, width: usize) {
    tty.puts(s);
    let len = s.chars().count();
    for _ in len..width {
        tty.putchar(' ' as i32);
    }
}

/// Emit `v` in decimal right-justified in `width` (`%Nd`).
fn put_dec_width<T: VideoTty>(tty: &mut T, v: u32, width: usize) {
    let mut buf = [0u8; 32];
    let n = write_dec(&mut buf, 0, v);
    for _ in n..width {
        tty.putchar(' ' as i32);
    }
    for &b in &buf[..n] {
        tty.putchar(b as i32);
    }
}

/// Emit `v` in uppercase hex right-justified in `width`, zero-padded (`%0NX`).
fn put_hex_width<T: VideoTty>(tty: &mut T, v: u32, width: usize) {
    let mut digits = [0u8; 8];
    let mut n = 0;
    let mut x = v;
    if x == 0 {
        digits[0] = b'0';
        n = 1;
    } else {
        while x > 0 {
            let d = (x & 0xf) as u8;
            digits[n] = if d < 10 { b'0' + d } else { b'A' + d - 10 };
            x >>= 4;
            n += 1;
        }
    }
    for _ in n..width {
        tty.putchar('0' as i32);
    }
    for i in (0..n).rev() {
        tty.putchar(digits[i] as i32);
    }
}

/// Emit `v` in lowercase hex with no padding (`%x`).
fn put_hex<T: VideoTty>(tty: &mut T, v: u32) {
    let mut digits = [0u8; 8];
    let mut n = 0;
    let mut x = v;
    if x == 0 {
        tty.putchar('0' as i32);
        return;
    }
    while x > 0 {
        let d = (x & 0xf) as u8;
        digits[n] = if d < 10 { b'0' + d } else { b'a' + d - 10 };
        x >>= 4;
        n += 1;
    }
    for i in (0..n).rev() {
        tty.putchar(digits[i] as i32);
    }
}

// --- BiosRegs byte/word accessors not provided by biosregs.rs ---------
// These operate directly on the public `eax`/`ebx`/`ecx`/`edx` fields and
// mirror the union "uppercase letter = low byte" convention in boot.h.

#[inline]
fn set_ah(r: &mut BiosRegs, v: u8) {
    r.eax = (r.eax & 0xffff_00ff) | ((v as u32) << 8);
}
#[inline]
fn set_dh(r: &mut BiosRegs, v: u8) {
    r.edx = (r.edx & 0xffff_00ff) | ((v as u32) << 8);
}
#[inline]
fn set_dl(r: &mut BiosRegs, v: u8) {
    r.edx = (r.edx & 0xffff_ff00) | v as u32;
}
#[inline]
fn dh(r: &BiosRegs) -> u8 {
    (r.edx >> 8) as u8
}
#[inline]
fn dl(r: &BiosRegs) -> u8 {
    r.edx as u8
}
#[inline]
fn ch(r: &BiosRegs) -> u8 {
    (r.ecx >> 8) as u8
}
#[inline]
fn cl(r: &BiosRegs) -> u8 {
    r.ecx as u8
}
#[inline]
fn bh(r: &BiosRegs) -> u8 {
    (r.ebx >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;
    use core::cell::RefCell;

    // ---- video.h constant fidelity (video.h:27-89) -------------------

    #[test]
    fn extended_video_mode_constants_match_video_h() {
        assert_eq!(VIDEO_FIRST_MENU, 0x0000);
        assert_eq!(VIDEO_FIRST_BIOS, 0x0100);
        assert_eq!(VIDEO_FIRST_VESA, 0x0200);
        assert_eq!(VIDEO_FIRST_V7, 0x0900);
        assert_eq!(VIDEO_FIRST_SPECIAL, 0x0f00);
        assert_eq!(VIDEO_80X25, 0x0f00);
        assert_eq!(VIDEO_8POINT, 0x0f01);
        assert_eq!(VIDEO_80X43, 0x0f02);
        assert_eq!(VIDEO_80X28, 0x0f03);
        assert_eq!(VIDEO_CURRENT_MODE, 0x0f04);
        assert_eq!(VIDEO_80X30, 0x0f05);
        assert_eq!(VIDEO_80X34, 0x0f06);
        assert_eq!(VIDEO_80X60, 0x0f07);
        assert_eq!(VIDEO_GFX_HACK, 0x0f08);
        assert_eq!(VIDEO_LAST_SPECIAL, 0x0f09);
        assert_eq!(VIDEO_FIRST_RESOLUTION, 0x1000);
        assert_eq!(VIDEO_RECALC, 0x8000);
    }

    #[test]
    fn svga_startup_constants_match_uapi_asm_boot_h() {
        // uapi/asm/boot.h
        assert_eq!(NORMAL_VGA, 0xffff);
        assert_eq!(EXTENDED_VGA, 0xfffe);
        assert_eq!(ASK_VGA, 0xfffd);
    }

    #[test]
    fn adapter_constants_match_video_h() {
        // video.h:87-89
        assert_eq!(ADAPTER_CGA, 0);
        assert_eq!(ADAPTER_EGA, 1);
        assert_eq!(ADAPTER_VGA, 2);
    }

    #[test]
    fn mode_info_field_order_matches_struct_mode_info() {
        // video.h:64-68 — { u16 mode; u16 x, y; u16 depth; } => 8 bytes,
        // all u16, no padding.
        assert_eq!(core::mem::size_of::<ModeInfo>(), 8);
        let mi = ModeInfo {
            mode: 0x0117,
            x: 1024,
            y: 768,
            depth: 32,
        };
        assert_eq!((mi.mode, mi.x, mi.y, mi.depth), (0x0117, 1024, 768, 32));
    }

    #[test]
    fn scan_token_matches_video_c_macro() {
        // video.c:197-198: SCAN = (H('s')<<12)+(H('c')<<8)+(H('a')<<4)+H('n')
        // H('s')=28, H('c')=12, H('a')=10, H('n')=23.
        assert_eq!(h(b's'), 28);
        assert_eq!(h(b'c'), 12);
        assert_eq!(h(b'a'), 10);
        assert_eq!(h(b'n'), 23);
        assert_eq!(SCAN, (28 << 12) + (12 << 8) + (10 << 4) + 23);
    }

    // ---- BIOS stubs --------------------------------------------------

    struct CursorBios {
        // Get-cursor (AH=03) reply.
        cur_dx: u32,
        cur_cx: u32,
        // Get-mode (AH=0F) reply.
        mode_ax: u16,
        mode_bx: u16,
        calls: RefCell<Vec<(u8, u8)>>, // (int_no, ah at call time)
    }
    impl BiosCaller for CursorBios {
        fn intcall(&self, int_no: u8, ireg: &BiosRegs, oreg: Option<&mut BiosRegs>) {
            let ah = ireg.ah();
            self.calls.borrow_mut().push((int_no, ah));
            if let Some(o) = oreg {
                *o = BiosRegs::default();
                if ah == 0x03 {
                    o.edx = self.cur_dx;
                    o.ecx = self.cur_cx;
                } else if ah == 0x0f {
                    o.eax = self.mode_ax as u32;
                    o.ebx = self.mode_bx as u32;
                }
            }
        }
    }

    struct FakeMem {
        font_size: u16,
        cols: u16,
        rows: u8,
        // (addr -> value) recordings are unnecessary; we serve fixed reads.
        copies_from: RefCell<Vec<(u32, usize)>>,
        copies_to: RefCell<Vec<(u32, usize)>>,
        blanks: RefCell<Vec<(u32, usize)>>,
        fs: RefCell<u16>,
    }
    impl Default for FakeMem {
        fn default() -> Self {
            FakeMem {
                font_size: 16,
                cols: 80,
                rows: 24,
                copies_from: RefCell::new(Vec::new()),
                copies_to: RefCell::new(Vec::new()),
                blanks: RefCell::new(Vec::new()),
                fs: RefCell::new(0),
            }
        }
    }
    impl VideoMem for FakeMem {
        fn set_fs(&mut self, seg: u16) {
            *self.fs.borrow_mut() = seg;
        }
        fn rdfs8(&self, addr: u32) -> u8 {
            match addr {
                0x484 => self.rows,
                0x485 => self.font_size as u8,
                _ => 0,
            }
        }
        fn rdfs16(&self, addr: u32) -> u16 {
            match addr {
                0x485 => self.font_size,
                0x44a => self.cols,
                _ => 0,
            }
        }
        fn copy_from_fs(&self, _dst: &mut [u16], src: u32, len: usize) {
            self.copies_from.borrow_mut().push((src, len));
        }
        fn copy_to_fs(&mut self, dst: u32, _src: &[u16], len: usize) {
            self.copies_to.borrow_mut().push((dst, len));
        }
        fn blank_fs(&mut self, dst: u32, count: usize) {
            self.blanks.borrow_mut().push((dst, count));
        }
    }

    #[test]
    fn store_cursor_position_records_xy_and_nocursor_flag() {
        // AH=03: DL=col=10, DH=row=5; CH=0x20 sets the NOCURSOR flag.
        let bios = CursorBios {
            cur_dx: 0x0500 | 0x0a, // dh=5, dl=10
            cur_cx: 0x2000,        // ch=0x20
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut st = VideoState::default();
        store_cursor_position(&bios, &mut st);
        assert_eq!(st.screen_info.orig_x, 10);
        assert_eq!(st.screen_info.orig_y, 5);
        assert_eq!(
            st.screen_info.flags & VIDEO_FLAGS_NOCURSOR,
            VIDEO_FLAGS_NOCURSOR
        );
    }

    #[test]
    fn store_cursor_position_sets_nocursor_when_start_below_end() {
        // (CH & 0x1f) > (CL & 0x1f) => no cursor (video.c:36-37).
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0x0a05, // ch=0x0a (=10), cl=0x05 (=5): 10 > 5
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut st = VideoState::default();
        store_cursor_position(&bios, &mut st);
        assert_eq!(
            st.screen_info.flags & VIDEO_FLAGS_NOCURSOR,
            VIDEO_FLAGS_NOCURSOR
        );
    }

    #[test]
    fn store_video_mode_masks_top_bit_and_records_page() {
        // AH=0F reply: AL=0x83 (mode 3 with stray top bit), BH=page 2.
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x0083,
            mode_bx: 0x0200, // bh=2
            calls: RefCell::new(Vec::new()),
        };
        let mut st = VideoState::default();
        store_video_mode(&bios, &mut st);
        assert_eq!(st.screen_info.orig_video_mode, 0x03);
        assert_eq!(st.screen_info.orig_video_page, 2);
    }

    #[test]
    fn store_mode_params_picks_b000_segment_for_mono_mode_07() {
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x07, // monochrome
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut mem = FakeMem::default();
        let mut st = VideoState {
            adapter: ADAPTER_VGA,
            ..Default::default()
        };
        store_mode_params(&bios, &mut mem, &mut st);
        assert_eq!(st.video_segment, 0xb000);
        assert_eq!(st.screen_info.orig_video_points, 16);
        assert_eq!(st.screen_info.orig_video_cols, 80);
        // rows = rdfs8(0x484)+1 = 24+1 = 25 (non-CGA path).
        assert_eq!(st.screen_info.orig_video_lines, 25);
    }

    #[test]
    fn store_mode_params_forces_cga_to_25_rows() {
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut mem = FakeMem::default();
        let mut st = VideoState {
            adapter: ADAPTER_CGA,
            ..Default::default()
        };
        store_mode_params(&bios, &mut mem, &mut st);
        assert_eq!(st.video_segment, 0xb800);
        // CGA path forces y = 25 regardless of rdfs8(0x484).
        assert_eq!(st.screen_info.orig_video_lines, 25);
    }

    #[test]
    fn store_mode_params_applies_force_x_and_force_y() {
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut mem = FakeMem::default();
        let mut st = VideoState {
            adapter: ADAPTER_VGA,
            force_x: 132,
            force_y: 60,
            ..Default::default()
        };
        store_mode_params(&bios, &mut mem, &mut st);
        assert_eq!(st.screen_info.orig_video_cols, 132);
        assert_eq!(st.screen_info.orig_video_lines, 60);
    }

    #[test]
    fn store_mode_params_returns_early_for_graphics() {
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut mem = FakeMem::default();
        let mut st = VideoState {
            graphic_mode: 1,
            ..Default::default()
        };
        store_mode_params(&bios, &mut mem, &mut st);
        // No BIOS calls should have been issued.
        assert!(bios.calls.borrow().is_empty());
    }

    // ---- get_entry / menu --------------------------------------------

    struct ScriptTty {
        keys: alloc::collections::VecDeque<i32>,
        timeout_keys: alloc::collections::VecDeque<i32>,
        out: RefCell<alloc::string::String>,
    }
    impl ScriptTty {
        fn new(keys: &[i32]) -> Self {
            ScriptTty {
                keys: keys.iter().copied().collect(),
                timeout_keys: alloc::collections::VecDeque::new(),
                out: RefCell::new(alloc::string::String::new()),
            }
        }
    }
    impl VideoTty for ScriptTty {
        fn getchar(&mut self) -> i32 {
            self.keys.pop_front().unwrap_or('\r' as i32)
        }
        fn getchar_timeout(&mut self) -> i32 {
            self.timeout_keys.pop_front().unwrap_or(0)
        }
        fn putchar(&mut self, c: i32) {
            if let Some(ch) = char::from_u32(c as u32) {
                self.out.borrow_mut().push(ch);
            }
        }
        fn puts(&mut self, s: &str) {
            self.out.borrow_mut().push_str(s);
        }
        fn kbd_flush(&mut self) {}
    }

    #[test]
    fn get_entry_parses_hex_and_defaults_to_current_mode() {
        // "31f\r" => 0x31f.
        let mut tty = ScriptTty::new(&['3' as i32, '1' as i32, 'f' as i32, '\r' as i32]);
        assert_eq!(get_entry(&mut tty), 0x31f);

        // Empty line => VIDEO_CURRENT_MODE.
        let mut tty2 = ScriptTty::new(&['\r' as i32]);
        assert_eq!(get_entry(&mut tty2), VIDEO_CURRENT_MODE as u32);
    }

    #[test]
    fn get_entry_handles_backspace() {
        // "12\b3\r" => "13" => 0x13.
        let mut tty = ScriptTty::new(&[
            '1' as i32,
            '2' as i32,
            '\u{8}' as i32,
            '3' as i32,
            '\r' as i32,
        ]);
        assert_eq!(get_entry(&mut tty), 0x13);
    }

    #[test]
    fn get_entry_returns_scan_token_for_scan() {
        let mut tty =
            ScriptTty::new(&['s' as i32, 'c' as i32, 'a' as i32, 'n' as i32, '\r' as i32]);
        assert_eq!(get_entry(&mut tty), SCAN);
    }

    // A tiny card so the menu/set_video tests have something to drive.
    struct VgaLikeCard {
        modes: Vec<ModeInfo>,
        nmodes: i32,
        name: alloc::string::String,
        set_calls: RefCell<Vec<u16>>,
    }
    impl CardInfo for VgaLikeCard {
        fn card_name(&self) -> &str {
            &self.name
        }
        fn set_mode(&mut self, mode: &ModeInfo) -> i32 {
            self.set_calls.borrow_mut().push(mode.mode);
            0
        }
        fn probe(&mut self) -> i32 {
            self.nmodes
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
    }

    fn vga_like() -> VgaLikeCard {
        VgaLikeCard {
            modes: alloc::vec![
                ModeInfo {
                    mode: VIDEO_80X25,
                    x: 80,
                    y: 25,
                    depth: 0
                },
                ModeInfo {
                    mode: VIDEO_8POINT,
                    x: 80,
                    y: 50,
                    depth: 0
                },
            ],
            nmodes: 2,
            name: alloc::string::String::from("VGA"),
            set_calls: RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn display_menu_prints_one_header_and_each_visible_mode() {
        let mut cards = [vga_like()];
        let mut tty = ScriptTty::new(&[]);
        display_menu(&mut cards, &mut tty);
        let out = tty.out.borrow().clone();
        // Single header line (nmodes < 20 => modes_per_line == 1).
        assert_eq!(out.matches("Mode: Resolution:  Type: ").count(), 1);
        // The two VGA mode IDs printed in 3-wide uppercase hex.
        assert!(out.contains("F00")); // VIDEO_80X25 = 0x0f00
        assert!(out.contains("F01")); // VIDEO_8POINT = 0x0f01
        assert!(out.contains("VGA"));
    }

    #[test]
    fn mode_menu_space_returns_current_mode() {
        let mut cards = [vga_like()];
        let mut tty = ScriptTty::new(&[]);
        tty.timeout_keys.push_back(' ' as i32);
        let sel = mode_menu(&mut cards, &mut tty, |_| {});
        assert_eq!(sel, VIDEO_CURRENT_MODE as u32);
    }

    #[test]
    fn mode_menu_enter_then_pick_returns_selection() {
        let mut cards = [vga_like()];
        let mut tty = ScriptTty::new(&['f' as i32, '0' as i32, '0' as i32, '\r' as i32]);
        tty.timeout_keys.push_back('\r' as i32); // press ENTER => show menu
        let sel = mode_menu(&mut cards, &mut tty, |_| {});
        assert_eq!(sel, 0xf00);
    }

    #[test]
    fn mode_menu_scan_triggers_rescan_then_returns_next_entry() {
        let mut cards = [vga_like()];
        let mut tty = ScriptTty::new(&[
            // First entry: "scan"
            's' as i32,
            'c' as i32,
            'a' as i32,
            'n' as i32,
            '\r' as i32,
            // Second entry: "1"
            '1' as i32,
            '\r' as i32,
        ]);
        tty.timeout_keys.push_back('\r' as i32);
        let mut scans = 0;
        let sel = mode_menu(&mut cards, &mut tty, |_| scans += 1);
        assert_eq!(scans, 1);
        assert_eq!(sel, 0x1);
    }

    // ---- save_screen / restore_screen --------------------------------

    #[test]
    fn save_screen_snapshots_cols_times_rows_cells() {
        let mut mem = FakeMem::default();
        let mut st = VideoState::default();
        st.screen_info.orig_video_cols = 80;
        st.screen_info.orig_video_lines = 25;
        st.video_segment = 0xb800;
        save_screen(&mut mem, &mut st);
        assert_eq!(st.saved.x, 80);
        assert_eq!(st.saved.y, 25);
        let copies = mem.copies_from.borrow();
        assert_eq!(copies.len(), 1);
        // len in bytes = 80*25*2.
        assert_eq!(copies[0].1, 80 * 25 * 2);
        assert!(st.saved.data.is_some());
    }

    #[test]
    fn restore_screen_pads_and_restores_cursor() {
        let mut mem = FakeMem::default();
        let mut st = VideoState::default();
        st.screen_info.orig_video_cols = 80;
        st.screen_info.orig_video_lines = 25;
        st.video_segment = 0xb800;
        // Saved smaller screen: 40x10 so each row pads 40 blanks, and rows
        // 10..25 are entirely blank.
        st.saved.x = 40;
        st.saved.y = 10;
        st.saved.curx = 5;
        st.saved.cury = 3;
        st.saved.data = Some(alloc::vec![0u16; 40 * 10]);

        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        restore_screen(&bios, &mut mem, &mut st);

        // 10 copy_to_fs of 40 cells each.
        assert_eq!(mem.copies_to.borrow().len(), 10);
        // Cursor stays in range (5 < 80, 3 < 25).
        assert_eq!(st.saved.curx, 5);
        assert_eq!(st.saved.cury, 3);
        // A set-cursor (AH=02) then a re-read (AH=03) were issued.
        let ahs: Vec<u8> = bios.calls.borrow().iter().map(|c| c.1).collect();
        assert!(ahs.contains(&0x02));
        assert!(ahs.contains(&0x03));
    }

    #[test]
    fn restore_screen_noops_in_graphic_mode() {
        let mut mem = FakeMem::default();
        let mut st = VideoState {
            graphic_mode: 1,
            ..Default::default()
        };
        st.saved.data = Some(alloc::vec![0u16; 4]);
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        restore_screen(&bios, &mut mem, &mut st);
        assert!(mem.copies_to.borrow().is_empty());
    }

    // ---- set_video orchestration -------------------------------------

    #[test]
    fn set_video_sets_requested_mode_and_returns_it() {
        let mut cards = [vga_like()];
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut mem = FakeMem::default();
        let mut tty = ScriptTty::new(&[]);
        let mut st = VideoState {
            adapter: ADAPTER_VGA,
            ..Default::default()
        };

        let mut probe_calls = 0;
        let mut edid_calls = 0;
        let final_mode = set_video(
            &mut cards,
            &bios,
            &mut mem,
            &mut tty,
            &mut st,
            VIDEO_80X25,
            |_, _| probe_calls += 1,
            |c, m| {
                c[0].set_mode(&ModeInfo {
                    mode: m,
                    ..Default::default()
                })
            },
            || edid_calls += 1,
            |_| {},
        );

        assert_eq!(final_mode, VIDEO_80X25);
        assert_eq!(probe_calls, 1);
        assert_eq!(edid_calls, 1);
    }

    #[test]
    fn set_video_retries_via_menu_when_mode_undefined() {
        let mut cards = [vga_like()];
        let bios = CursorBios {
            cur_dx: 0,
            cur_cx: 0,
            mode_ax: 0x03,
            mode_bx: 0,
            calls: RefCell::new(Vec::new()),
        };
        let mut mem = FakeMem::default();
        // The menu will be entered: ENTER to show, then "f00\r" picks 0xf00.
        let mut tty = ScriptTty::new(&['f' as i32, '0' as i32, '0' as i32, '\r' as i32]);
        tty.timeout_keys.push_back('\r' as i32);
        let mut st = VideoState::default();

        // First set_mode (for a deliberately bad mode) fails; the picked
        // 0xf00 succeeds.
        let final_mode = set_video(
            &mut cards,
            &bios,
            &mut mem,
            &mut tty,
            &mut st,
            0xBAD,
            |_, _| {},
            |_, m| if m == 0xf00 { 0 } else { -1 },
            || {},
            |_| {},
        );
        assert_eq!(final_mode, 0xf00);
    }

    // ---- indexed register helpers ------------------------------------
    use core::sync::atomic::{AtomicU32, Ordering};
    static IDX_LAST_OUTB: AtomicU32 = AtomicU32::new(0);
    static IDX_LAST_OUTW: AtomicU32 = AtomicU32::new(0);
    static IDX_INB_RET: AtomicU32 = AtomicU32::new(0);

    fn idx_inb(_port: u16) -> u8 {
        IDX_INB_RET.load(Ordering::Relaxed) as u8
    }
    fn idx_outb(v: u8, port: u16) {
        IDX_LAST_OUTB.store(((v as u32) << 16) | port as u32, Ordering::Relaxed);
    }
    fn idx_outw(v: u16, port: u16) {
        IDX_LAST_OUTW.store(((v as u32) << 16) | port as u32, Ordering::Relaxed);
    }

    #[test]
    fn out_idx_packs_index_and_value_into_one_word() {
        // video.h:103-106: outw(index + (v << 8), port).
        let io = PortIoOps {
            f_inb: idx_inb,
            f_outb: idx_outb,
            f_outw: idx_outw,
        };
        out_idx(&io, 0x0c, 0x3d4, 0x11);
        let rec = IDX_LAST_OUTW.load(Ordering::Relaxed);
        assert_eq!(rec & 0xffff, 0x3d4); // port
        assert_eq!((rec >> 16) as u16, 0x0c11); // (v<<8)+index
    }

    #[test]
    fn in_idx_writes_index_then_reads_data_port() {
        IDX_INB_RET.store(0x5a, Ordering::Relaxed);
        let io = PortIoOps {
            f_inb: idx_inb,
            f_outb: idx_outb,
            f_outw: idx_outw,
        };
        let v = in_idx(&io, 0x3d4, 0x0f);
        // index byte was written to the port.
        let rec = IDX_LAST_OUTB.load(Ordering::Relaxed);
        assert_eq!(rec & 0xffff, 0x3d4);
        assert_eq!((rec >> 16) as u8, 0x0f);
        assert_eq!(v, 0x5a);
    }
}
