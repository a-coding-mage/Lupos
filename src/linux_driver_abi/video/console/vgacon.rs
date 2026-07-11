//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/console/vgacon.c
//! test-origin: linux:vendor/linux/drivers/video/console/vgacon.c
pub mod buffer {
    /// VGA text-mode buffer driver.
    ///
    /// The standard VGA text buffer lives at physical address `0xB8000` and
    /// is 80 columns × 25 rows of 16-bit cells.  Each cell stores an ASCII
    /// byte and a color attribute byte.
    ///
    /// The `Writer` struct accepts a raw pointer to the buffer so that unit
    /// tests can supply a stack-allocated array instead of real hardware.
    ///
    /// Ref: https://wiki.osdev.org/Text_UI
    ///      https://wiki.osdev.org/Printing_To_Screen
    use core::fmt;

    use crate::linux_driver_abi::video::fbdev::core::writer::TextCell;

    /// Standard VGA text-mode dimensions.
    pub const BUFFER_WIDTH: usize = 80;
    pub const BUFFER_HEIGHT: usize = 25;

    /// VGA color palette — 4-bit color codes used in attribute bytes.
    ///
    /// Ref: https://wiki.osdev.org/Text_UI#Colours
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    #[allow(dead_code)]
    pub enum Color {
        Black = 0,
        Blue = 1,
        Green = 2,
        Cyan = 3,
        Red = 4,
        Magenta = 5,
        Brown = 6,
        LightGray = 7,
        DarkGray = 8,
        LightBlue = 9,
        LightGreen = 10,
        LightCyan = 11,
        LightRed = 12,
        Pink = 13,
        Yellow = 14,
        White = 15,
    }

    /// A single character cell in the VGA text buffer.
    ///
    /// Layout: `[ascii_byte, attribute_byte]` where the attribute encodes
    /// foreground (bits 0–3) and background (bits 4–6) colors, plus a
    /// blink bit (bit 7, usually disabled in modern VGA emulations).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(C)]
    pub struct VgaChar {
        pub ascii: u8,
        pub color: u8,
    }

    /// Combine foreground and background into an attribute byte.
    pub const fn color_code(fg: Color, bg: Color) -> u8 {
        (bg as u8) << 4 | (fg as u8)
    }

    /// A writer that renders text into a VGA-compatible character buffer.
    ///
    /// The buffer pointer is injectable: on real hardware it points to
    /// `0xB8000`; in tests it points to a stack-allocated array.
    pub struct Writer {
        pub col: usize,
        pub row: usize,
        pub color: u8,
        cursor_pos: usize,
        cursor_visible: bool,
        /// Pointer to a `[[VgaChar; 80]; 25]` buffer.
        buffer: *mut VgaChar,
    }

    // Safety: Writer is only accessed behind a Mutex (spin lock), and we
    // are single-threaded in the kernel at this stage.
    unsafe impl Send for Writer {}
    unsafe impl Sync for Writer {}

    impl Writer {
        /// Create a new writer targeting `buffer` with the given colors.
        ///
        /// # Safety
        /// `buffer` must point to a valid, writable region of at least
        /// `BUFFER_WIDTH * BUFFER_HEIGHT * 2` bytes (4000 bytes for 80×25).
        pub unsafe fn new(buffer: *mut VgaChar, fg: Color, bg: Color) -> Self {
            Self {
                col: 0,
                row: 0,
                color: color_code(fg, bg),
                cursor_pos: 0,
                cursor_visible: true,
                buffer,
            }
        }

        /// Change the foreground and background colors for subsequent writes.
        pub fn set_color(&mut self, fg: Color, bg: Color) {
            self.color = color_code(fg, bg);
        }

        /// Clear the entire screen with spaces in the current color.
        pub fn clear(&mut self) {
            let blank = VgaChar {
                ascii: b' ',
                color: self.color,
            };
            for i in 0..(BUFFER_WIDTH * BUFFER_HEIGHT) {
                self.write_cell(i, blank);
            }
            self.col = 0;
            self.row = 0;
            self.set_cursor_visible(true);
            self.sync_cursor();
        }

        /// Write a single ASCII byte to the buffer.
        pub fn write_byte(&mut self, byte: u8) {
            match byte {
                b'\n' => self.new_line(),
                // Printable ASCII range (space through tilde).
                0x20..=0x7E => {
                    if self.col >= BUFFER_WIDTH {
                        self.new_line();
                    }
                    let idx = self.row * BUFFER_WIDTH + self.col;
                    self.write_cell(
                        idx,
                        VgaChar {
                            ascii: byte,
                            color: self.color,
                        },
                    );
                    self.col += 1;
                }
                // Non-printable bytes (including multi-byte UTF-8 continuations) are
                // rendered as 0xFE (■) directly — do NOT recurse, since 0xFE > 0x7E
                // and would cause infinite recursion → stack overflow → triple fault.
                _ => {
                    if self.col >= BUFFER_WIDTH {
                        self.new_line();
                    }
                    let idx = self.row * BUFFER_WIDTH + self.col;
                    self.write_cell(
                        idx,
                        VgaChar {
                            ascii: 0xFE,
                            color: self.color,
                        },
                    );
                    self.col += 1;
                }
            }
            self.sync_cursor();
        }

        /// Write a string, byte by byte.
        pub fn write_string(&mut self, s: &str) {
            for byte in s.bytes() {
                self.write_byte(byte);
            }
        }

        /// Advance to the next line, scrolling if at the bottom.
        fn new_line(&mut self) {
            self.col = 0;
            if self.row < BUFFER_HEIGHT - 1 {
                self.row += 1;
            } else {
                self.scroll_up();
            }
        }

        /// Scroll all rows up by one, clearing the bottom row.
        fn scroll_up(&mut self) {
            // Move rows 1..25 to 0..24.
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let ch = self.read_cell(row * BUFFER_WIDTH + col);
                    self.write_cell((row - 1) * BUFFER_WIDTH + col, ch);
                }
            }
            // Clear the last row.
            let blank = VgaChar {
                ascii: b' ',
                color: self.color,
            };
            for col in 0..BUFFER_WIDTH {
                self.write_cell((BUFFER_HEIGHT - 1) * BUFFER_WIDTH + col, blank);
            }
        }

        pub fn render_console_batch(&mut self, batch: &crate::kernel::console::RenderBatch) {
            let max_rows = batch.rows.min(BUFFER_HEIGHT);
            let max_cols = batch.cols.min(BUFFER_WIDTH);
            if let Some(clear) = batch.clear {
                self.clear_cells_with(clear.blank);
            }
            for dirty in &batch.dirty_rows {
                if dirty.row >= max_rows {
                    continue;
                }
                for col in 0..max_cols.min(dirty.cells.len()) {
                    self.write_cell(
                        dirty.row * BUFFER_WIDTH + col,
                        vga_char_from_text_cell(dirty.cells[col]),
                    );
                }
            }
            match batch.cursor {
                Some((col, row)) => {
                    self.col = col.min(BUFFER_WIDTH - 1);
                    self.row = row.min(BUFFER_HEIGHT - 1);
                    self.set_cursor_visible(true);
                    self.sync_cursor();
                }
                None => self.set_cursor_visible(false),
            }
        }

        fn clear_cells_with(&mut self, blank: TextCell) {
            let cell = vga_char_from_text_cell(blank);
            for i in 0..(BUFFER_WIDTH * BUFFER_HEIGHT) {
                self.write_cell(i, cell);
            }
        }

        fn sync_cursor(&mut self) {
            let pos = self.next_cursor_pos();
            self.cursor_pos = pos;
            #[cfg(not(test))]
            unsafe {
                write_hardware_cursor(pos as u16);
            }
        }

        fn set_cursor_visible(&mut self, visible: bool) {
            if self.cursor_visible == visible {
                return;
            }
            self.cursor_visible = visible;
            #[cfg(not(test))]
            unsafe {
                write_hardware_cursor_visibility(visible);
            }
        }

        fn next_cursor_pos(&self) -> usize {
            let mut row = self.row.min(BUFFER_HEIGHT - 1);
            let mut col = self.col;
            if col >= BUFFER_WIDTH {
                if row < BUFFER_HEIGHT - 1 {
                    row += 1;
                    col = 0;
                } else {
                    col = BUFFER_WIDTH - 1;
                }
            }
            row * BUFFER_WIDTH + col.min(BUFFER_WIDTH - 1)
        }

        /// Write a `VgaChar` to the cell at flat index `idx`.
        ///
        /// Uses volatile writes so the compiler cannot optimize away
        /// stores to memory-mapped I/O.
        fn write_cell(&mut self, idx: usize, ch: VgaChar) {
            unsafe {
                core::ptr::write_volatile(self.buffer.add(idx), ch);
            }
        }

        /// Read a `VgaChar` from the cell at flat index `idx`.
        fn read_cell(&self, idx: usize) -> VgaChar {
            unsafe { core::ptr::read_volatile(self.buffer.add(idx)) }
        }
    }

    fn vga_char_from_text_cell(cell: TextCell) -> VgaChar {
        VgaChar {
            ascii: if cell.ch == 0 { b' ' } else { cell.ch },
            color: color_code(vga_color(cell.fg), vga_color(cell.bg)),
        }
    }

    fn vga_color(rgb: u32) -> Color {
        match rgb {
            0x0000_0000 => Color::Black,
            0x0000_00aa => Color::Blue,
            0x0000_aa00 => Color::Green,
            0x0000_aaaa => Color::Cyan,
            0x00aa_0000 => Color::Red,
            0x00aa_00aa => Color::Magenta,
            0x00aa_5500 => Color::Brown,
            0x00aa_aaaa => Color::LightGray,
            0x0055_5555 => Color::DarkGray,
            0x0055_55ff => Color::LightBlue,
            0x0055_ff55 => Color::LightGreen,
            0x0055_ffff => Color::LightCyan,
            0x00ff_5555 => Color::LightRed,
            0x00ff_55ff => Color::Pink,
            0x00ff_ff55 => Color::Yellow,
            0x00ff_ffff => Color::White,
            _ => Color::White,
        }
    }

    #[cfg(not(test))]
    unsafe fn write_hardware_cursor(pos: u16) {
        const VGA_CRTC_INDEX: u16 = 0x3d4;
        const VGA_CRTC_DATA: u16 = 0x3d5;
        const VGA_CRTC_CURSOR_HI: u8 = 0x0e;
        const VGA_CRTC_CURSOR_LO: u8 = 0x0f;

        unsafe {
            crate::arch::x86::include::asm::io::outb(VGA_CRTC_INDEX, VGA_CRTC_CURSOR_HI);
            crate::arch::x86::include::asm::io::outb(VGA_CRTC_DATA, (pos >> 8) as u8);
            crate::arch::x86::include::asm::io::outb(VGA_CRTC_INDEX, VGA_CRTC_CURSOR_LO);
            crate::arch::x86::include::asm::io::outb(VGA_CRTC_DATA, pos as u8);
        }
    }

    #[cfg(not(test))]
    unsafe fn write_hardware_cursor_visibility(visible: bool) {
        const VGA_CRTC_INDEX: u16 = 0x3d4;
        const VGA_CRTC_DATA: u16 = 0x3d5;
        const VGA_CRTC_CURSOR_START: u8 = 0x0a;
        const VGA_CRTC_CURSOR_DISABLE: u8 = 1 << 5;

        unsafe {
            crate::arch::x86::include::asm::io::outb(VGA_CRTC_INDEX, VGA_CRTC_CURSOR_START);
            let start = crate::arch::x86::include::asm::io::inb(VGA_CRTC_DATA);
            crate::arch::x86::include::asm::io::outb(VGA_CRTC_INDEX, VGA_CRTC_CURSOR_START);
            crate::arch::x86::include::asm::io::outb(
                VGA_CRTC_DATA,
                if visible {
                    start & !VGA_CRTC_CURSOR_DISABLE
                } else {
                    start | VGA_CRTC_CURSOR_DISABLE
                },
            );
        }
    }

    impl fmt::Write for Writer {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.write_string(s);
            Ok(())
        }
    }

    // ==========================================================================
    // Unit tests — run on the host with `cargo test -p lupos --lib`
    // ==========================================================================
    #[cfg(test)]
    mod tests {
        use super::*;

        /// Helper: create a Writer backed by a stack-allocated buffer.
        fn test_writer(buf: &mut [VgaChar; BUFFER_WIDTH * BUFFER_HEIGHT]) -> Writer {
            unsafe { Writer::new(buf.as_mut_ptr(), Color::White, Color::Black) }
        }

        fn blank_buffer() -> [VgaChar; BUFFER_WIDTH * BUFFER_HEIGHT] {
            [VgaChar { ascii: 0, color: 0 }; BUFFER_WIDTH * BUFFER_HEIGHT]
        }

        #[test]
        fn write_single_byte() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            w.write_byte(b'A');
            assert_eq!(buf[0].ascii, b'A');
            assert_eq!(buf[0].color, color_code(Color::White, Color::Black));
            assert_eq!(w.col, 1);
            assert_eq!(w.row, 0);
        }

        #[test]
        fn write_string_hello() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            w.write_string("hello world");
            assert_eq!(buf[0].ascii, b'h');
            assert_eq!(buf[1].ascii, b'e');
            assert_eq!(buf[10].ascii, b'd');
            assert_eq!(w.col, 11);
        }

        #[test]
        fn newline_advances_row() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            w.write_string("abc\ndef");
            assert_eq!(w.row, 1);
            assert_eq!(w.col, 3);
            // 'd' is at row 1, col 0
            assert_eq!(buf[BUFFER_WIDTH].ascii, b'd');
        }

        #[test]
        fn line_wrap_at_80_columns() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            // Write exactly 80 characters, then one more.
            for _ in 0..BUFFER_WIDTH {
                w.write_byte(b'X');
            }
            assert_eq!(w.col, BUFFER_WIDTH);
            w.write_byte(b'Y');
            // Should have wrapped to row 1, col 1.
            assert_eq!(w.row, 1);
            assert_eq!(w.col, 1);
            assert_eq!(buf[BUFFER_WIDTH].ascii, b'Y');
            assert_eq!(w.cursor_pos, BUFFER_WIDTH + 1);
        }

        #[test]
        fn scroll_moves_rows_up() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            // Fill all 25 rows, then write one more line.
            for i in 0..BUFFER_HEIGHT {
                w.write_byte(b'0' + (i as u8 % 10));
                if i < BUFFER_HEIGHT - 1 {
                    w.write_byte(b'\n');
                }
            }
            assert_eq!(w.row, BUFFER_HEIGHT - 1);
            // Trigger scroll.
            w.write_byte(b'\n');
            w.write_byte(b'Z');
            // Row 0 should now contain what was row 1 ('1').
            assert_eq!(buf[0].ascii, b'1');
            // Last row should have 'Z'.
            assert_eq!(buf[(BUFFER_HEIGHT - 1) * BUFFER_WIDTH].ascii, b'Z');
            assert_eq!(w.cursor_pos, (BUFFER_HEIGHT - 1) * BUFFER_WIDTH + 1);
        }

        #[test]
        fn clear_resets_buffer() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            w.write_string("dirty");
            w.clear();
            assert_eq!(w.col, 0);
            assert_eq!(w.row, 0);
            assert_eq!(w.cursor_pos, 0);
            // Every cell should be a space with the writer's color.
            for cell in buf.iter() {
                assert_eq!(cell.ascii, b' ');
                assert_eq!(cell.color, color_code(Color::White, Color::Black));
            }
        }

        #[test]
        fn hardware_cursor_position_tracks_written_bytes() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);

            w.write_string("abc\ndef");

            assert_eq!(w.row, 1);
            assert_eq!(w.col, 3);
            assert_eq!(w.cursor_pos, BUFFER_WIDTH + 3);
        }

        #[test]
        fn render_batch_writes_tty_cells_to_vga_text_buffer() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            let cells = b"login:"
                .iter()
                .map(|ch| TextCell {
                    ch: *ch,
                    fg: 0x00ff_ffff,
                    bg: 0x0000_0000,
                })
                .collect();
            let batch = crate::kernel::console::RenderBatch {
                cols: BUFFER_WIDTH,
                rows: BUFFER_HEIGHT,
                clear: None,
                dirty_rows: alloc::vec![crate::kernel::console::DirtyRow { row: 2, cells }],
                cursor: Some((6, 2)),
            };

            w.render_console_batch(&batch);

            for (col, ch) in b"login:".iter().enumerate() {
                assert_eq!(buf[2 * BUFFER_WIDTH + col].ascii, *ch);
                assert_eq!(
                    buf[2 * BUFFER_WIDTH + col].color,
                    color_code(Color::White, Color::Black)
                );
            }
            assert_eq!(w.cursor_pos, 2 * BUFFER_WIDTH + 6);
        }

        #[test]
        fn render_batch_clear_replaces_stale_boot_log_cells() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            w.write_string("Run /sbin/init");
            let batch = crate::kernel::console::RenderBatch {
                cols: BUFFER_WIDTH,
                rows: BUFFER_HEIGHT,
                clear: Some(crate::kernel::console::ClearOp {
                    blank: TextCell::default(),
                    flush_scrollback: false,
                }),
                dirty_rows: alloc::vec![],
                cursor: Some((0, 0)),
            };

            w.render_console_batch(&batch);

            assert!(buf.iter().all(|cell| cell.ascii == b' '));
            assert_eq!(w.cursor_pos, 0);
        }

        #[test]
        fn render_batch_hides_and_restores_hardware_cursor_state() {
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            let mut batch = crate::kernel::console::RenderBatch {
                cols: BUFFER_WIDTH,
                rows: BUFFER_HEIGHT,
                clear: None,
                dirty_rows: alloc::vec![],
                cursor: None,
            };

            w.render_console_batch(&batch);
            assert!(!w.cursor_visible);

            batch.cursor = Some((7, 3));
            w.render_console_batch(&batch);
            assert!(w.cursor_visible);
            assert_eq!(w.cursor_pos, 3 * BUFFER_WIDTH + 7);
        }

        #[test]
        fn fmt_write_trait() {
            use core::fmt::Write;
            let mut buf = blank_buffer();
            let mut w = test_writer(&mut buf);
            write!(w, "num={}", 42).unwrap();
            // "num=42" — 6 characters
            assert_eq!(buf[0].ascii, b'n');
            assert_eq!(buf[4].ascii, b'4');
            assert_eq!(buf[5].ascii, b'2');
        }
    }
}

/// VGA text-mode output — the kernel's primary display during early boot.
///
/// Provides `print!` and `println!` macros that write to the 80×25 VGA
/// text buffer at `0xB8000`.  The global `WRITER` is protected by a
/// spin-lock so it is safe to call from interrupt handlers (once we have them).
///
/// Ref: https://wiki.osdev.org/Text_UI
use self::buffer::{Color, VgaChar, Writer};
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

/// Physical address of the VGA text buffer (standard on all VGA-compatible PCs).
const VGA_BUFFER_ADDR: usize = 0xB8000;
static VGACON_ENABLED: AtomicBool = AtomicBool::new(true);

lazy_static! {
    /// Global VGA writer instance.
    ///
    /// Protected by a spin-lock `Mutex` from the `spin` crate (no_std compatible).
    /// The underlying buffer pointer targets `0xB8000` — the memory-mapped VGA
    /// text buffer provided by the hardware (or QEMU's emulated VGA).
    pub static ref WRITER: Mutex<Writer> = {
        let writer = unsafe {
            Writer::new(
                VGA_BUFFER_ADDR as *mut VgaChar,
                Color::LightGreen,
                Color::Black,
            )
        };
        Mutex::new(writer)
    };
}

/// Initialize the VGA display by clearing the screen.
pub fn init() {
    let mut writer = WRITER.lock();
    VGACON_ENABLED.store(true, Ordering::Release);
    writer.clear();
}

/// Unregister the legacy VGA console when a native graphics driver takes over.
/// This is the output-side effect of Linux `vga_remove_vgacon()`.
pub fn detach() {
    // Linux holds console_lock() across the dummy-console takeover.  WRITER is
    // the serialization point for Lupos's legacy console: changing the state
    // while holding it, paired with the post-lock checks below, guarantees no
    // printk/render that observed the old state can touch 0xb8000 after this
    // function returns to a native DRM driver.
    let _writer = WRITER.lock();
    VGACON_ENABLED.store(false, Ordering::Release);
}

pub fn is_enabled() -> bool {
    VGACON_ENABLED.load(Ordering::Acquire)
}

/// Internal helper used by the `print!` / `println!` macros.
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments<'_>) {
    if !is_enabled() {
        return;
    }
    use core::fmt::Write;
    let mut writer = WRITER.lock();
    if !is_enabled() {
        return;
    }
    writer.write_fmt(args).unwrap();
}

/// Render parsed VT output into the legacy VGA text console.
pub fn render_batch(batch: &crate::kernel::console::RenderBatch) {
    if !is_enabled() {
        return;
    }
    let mut writer = WRITER.lock();
    if !is_enabled() {
        return;
    }
    writer.render_console_batch(batch);
}

/// Print to the VGA text buffer (no trailing newline).
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::linux_driver_abi::video::console::vgacon::_print(format_args!($($arg)*))
    };
}

/// Print to the VGA text buffer with a trailing newline.
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($fmt:expr) => { $crate::print!(concat!($fmt, "\n")) };
    ($fmt:expr, $($arg:tt)*) => { $crate::print!(concat!($fmt, "\n"), $($arg)*) };
}
