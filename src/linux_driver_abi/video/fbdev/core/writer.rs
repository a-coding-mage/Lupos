//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/fbdev/core/fbcon.c
//! test-origin: linux:vendor/linux/drivers/video/fbdev/core/fbcon.c
extern crate alloc;

use super::font;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

pub const DEFAULT_FG: u32 = 0x00ff_ffff;
pub const DEFAULT_BG: u32 = 0x0000_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextCell {
    pub ch: u8,
    pub fg: u32,
    pub bg: u32,
}

impl Default for TextCell {
    fn default() -> Self {
        Self {
            ch: b' ',
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EscapeState {
    Ground,
    Escape,
    Csi,
    Osc,
    OscEscape,
    Dcs,
    DcsEscape,
}

/// Framebuffer text writer.
///
/// This is a compact VT-style console for QEMU's linear framebuffer.  It keeps
/// a text-cell backing buffer so cursor redraws and ANSI line edits do not
/// corrupt the visible shell.
pub struct FramebufferWriter {
    fb_addr: *mut u8,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u8,
    col: usize,
    row: usize,
    fg_color: u32,
    bg_color: u32,
    bold: bool,
    cells: Vec<TextCell>,
    cursor_drawn: bool,
    cursor_enabled: bool,
    cursor_blink_on: bool,
    esc_state: EscapeState,
    csi_params: [usize; 4],
    csi_count: usize,
    csi_value: usize,
    csi_have_digit: bool,
    csi_private: bool,
    csi_intermediate: u8,
}

// Safety: FramebufferWriter is only accessed behind a Mutex.
unsafe impl Send for FramebufferWriter {}
unsafe impl Sync for FramebufferWriter {}

impl FramebufferWriter {
    /// # Safety
    /// `fb_addr` must point to a valid, writable framebuffer region of at
    /// least `pitch * height` bytes. The framebuffer must be identity-mapped.
    pub unsafe fn new(fb_addr: *mut u8, pitch: u32, width: u32, height: u32, bpp: u8) -> Self {
        let cols = (width as usize / font::GLYPH_WIDTH).max(1);
        let rows = (height as usize / font::GLYPH_HEIGHT).max(1);
        Self {
            fb_addr,
            pitch,
            width,
            height,
            bpp,
            col: 0,
            row: 0,
            fg_color: DEFAULT_FG,
            bg_color: DEFAULT_BG,
            bold: false,
            cells: vec![TextCell::default(); cols * rows],
            cursor_drawn: false,
            cursor_enabled: true,
            cursor_blink_on: true,
            esc_state: EscapeState::Ground,
            csi_params: [0; 4],
            csi_count: 0,
            csi_value: 0,
            csi_have_digit: false,
            csi_private: false,
            csi_intermediate: 0,
        }
    }

    pub fn set_colors(&mut self, fg: u32, bg: u32) {
        self.erase_cursor();
        self.fg_color = fg;
        self.bg_color = bg;
        self.draw_cursor();
    }

    pub fn cols(&self) -> usize {
        (self.width as usize / font::GLYPH_WIDTH).max(1)
    }

    pub fn rows(&self) -> usize {
        (self.height as usize / font::GLYPH_HEIGHT).max(1)
    }

    pub fn pixel_width(&self) -> u16 {
        self.width.min(u16::MAX as u32) as u16
    }

    pub fn pixel_height(&self) -> u16 {
        self.height.min(u16::MAX as u32) as u16
    }

    pub fn clear(&mut self) {
        self.erase_cursor();
        let blank = TextCell {
            ch: b' ',
            fg: self.fg_color,
            bg: self.bg_color,
        };
        for cell in &mut self.cells {
            *cell = blank;
        }
        self.fill_pixels(self.bg_color);
        self.col = 0;
        self.row = 0;
        self.esc_state = EscapeState::Ground;
        self.draw_cursor();
    }

    pub fn clear_visible_cells(&mut self, blank: TextCell, cursor: Option<(usize, usize)>) {
        for cell in &mut self.cells {
            *cell = blank;
        }
        self.fill_pixels(blank.bg);
        if let Some((col, row)) = cursor {
            let col = col.min(self.cols() - 1);
            let row = row.min(self.rows() - 1);
            self.render_text_cell(col, row, blank, true);
        }
    }

    pub fn render_batch_cell(&mut self, col: usize, row: usize, cell: TextCell, cursor: bool) {
        let idx = self.cell_index(col, row);
        self.cells[idx] = cell;
        self.render_text_cell(col, row, cell, cursor);
    }

    pub fn write_char(&mut self, byte: u8) {
        self.erase_cursor();
        self.write_byte_inner(byte);
        self.draw_cursor();
    }

    pub fn set_cursor_blink_on(&mut self, on: bool) {
        if self.cursor_blink_on == on {
            return;
        }
        if on {
            self.cursor_blink_on = true;
            self.draw_cursor();
        } else {
            self.erase_cursor();
            self.cursor_blink_on = false;
        }
    }

    #[cfg(test)]
    fn cell_char(&self, row: usize, col: usize) -> u8 {
        self.cells[self.cell_index(col, row)].ch
    }

    fn write_byte_inner(&mut self, byte: u8) {
        match self.esc_state {
            EscapeState::Ground => self.write_ground(byte),
            EscapeState::Escape => self.write_escape(byte),
            EscapeState::Csi => self.write_csi(byte),
            EscapeState::Osc => self.write_osc(byte),
            EscapeState::OscEscape => self.write_osc_escape(byte),
            EscapeState::Dcs => self.write_dcs(byte),
            EscapeState::DcsEscape => self.write_dcs_escape(byte),
        }
    }

    fn write_ground(&mut self, byte: u8) {
        match byte {
            0x1b => self.esc_state = EscapeState::Escape,
            b'\n' => self.new_line(),
            b'\r' => self.col = 0,
            b'\t' => self.tab(),
            0x08 | 0x7f => self.backspace(),
            0x20..=0x7e => self.put_printable(byte),
            _ => {}
        }
    }

    fn write_escape(&mut self, byte: u8) {
        match byte {
            b'[' => {
                self.reset_csi();
                self.esc_state = EscapeState::Csi;
            }
            b']' => self.esc_state = EscapeState::Osc,
            b'P' => self.esc_state = EscapeState::Dcs,
            b'c' => {
                self.clear();
                self.esc_state = EscapeState::Ground;
            }
            _ => self.esc_state = EscapeState::Ground,
        }
    }

    fn write_csi(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' if self.csi_intermediate == 0 => {
                self.csi_have_digit = true;
                self.csi_value = self
                    .csi_value
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as usize)
                    .min(999);
            }
            b';' if self.csi_intermediate == 0 => self.push_csi_param(),
            b'?' if self.csi_intermediate == 0 => self.csi_private = true,
            0x20..=0x2f => self.csi_intermediate = byte,
            0x40..=0x7e => {
                self.push_csi_param();
                self.apply_csi(byte);
                self.esc_state = EscapeState::Ground;
            }
            _ => self.esc_state = EscapeState::Ground,
        }
    }

    fn write_osc(&mut self, byte: u8) {
        match byte {
            0x07 => self.esc_state = EscapeState::Ground,
            0x1b => self.esc_state = EscapeState::OscEscape,
            _ => {}
        }
    }

    fn write_osc_escape(&mut self, byte: u8) {
        self.esc_state = if byte == b'\\' {
            EscapeState::Ground
        } else {
            EscapeState::Osc
        };
    }

    fn write_dcs(&mut self, byte: u8) {
        if byte == 0x1b {
            self.esc_state = EscapeState::DcsEscape;
        }
    }

    fn write_dcs_escape(&mut self, byte: u8) {
        self.esc_state = if byte == b'\\' {
            EscapeState::Ground
        } else {
            EscapeState::Dcs
        };
    }

    fn reset_csi(&mut self) {
        self.csi_params = [0; 4];
        self.csi_count = 0;
        self.csi_value = 0;
        self.csi_have_digit = false;
        self.csi_private = false;
        self.csi_intermediate = 0;
    }

    fn push_csi_param(&mut self) {
        if self.csi_count < self.csi_params.len() {
            self.csi_params[self.csi_count] = if self.csi_have_digit {
                self.csi_value
            } else {
                0
            };
            self.csi_count += 1;
        }
        self.csi_value = 0;
        self.csi_have_digit = false;
    }

    fn csi_param(&self, idx: usize, default: usize) -> usize {
        if idx >= self.csi_count || self.csi_params[idx] == 0 {
            default
        } else {
            self.csi_params[idx]
        }
    }

    fn apply_csi(&mut self, final_byte: u8) {
        if self.csi_intermediate != 0 {
            if self.csi_intermediate == b'!' && final_byte == b'p' {
                self.soft_reset();
            }
            return;
        }
        match final_byte {
            b'A' => self.move_up(self.csi_param(0, 1)),
            b'B' => self.move_down(self.csi_param(0, 1)),
            b'C' => self.move_right(self.csi_param(0, 1)),
            b'D' => self.move_left(self.csi_param(0, 1)),
            b'G' => self.set_col_1based(self.csi_param(0, 1)),
            b'H' | b'f' => {
                self.set_cursor_1based(self.csi_param(0, 1), self.csi_param(1, 1));
            }
            b'J' => self.clear_display(self.csi_param(0, 0)),
            b'K' => self.clear_line(self.csi_param(0, 0)),
            b'P' => self.delete_chars(self.csi_param(0, 1)),
            b'h' if self.csi_private && self.csi_param(0, 0) == 25 => {
                self.cursor_enabled = true;
            }
            b'l' if self.csi_private && self.csi_param(0, 0) == 25 => {
                self.cursor_enabled = false;
            }
            b'm' => self.apply_sgr(),
            _ => {}
        }
    }

    fn soft_reset(&mut self) {
        self.fg_color = DEFAULT_FG;
        self.bg_color = DEFAULT_BG;
        self.bold = false;
        self.cursor_enabled = true;
        self.col = 0;
        self.row = 0;
    }

    fn apply_sgr(&mut self) {
        if self.csi_count == 0 {
            self.fg_color = DEFAULT_FG;
            self.bg_color = DEFAULT_BG;
            self.bold = false;
            return;
        }
        for idx in 0..self.csi_count {
            let code = self.csi_params[idx];
            match code {
                0 => {
                    self.fg_color = DEFAULT_FG;
                    self.bg_color = DEFAULT_BG;
                    self.bold = false;
                }
                1 => self.bold = true,
                22 => self.bold = false,
                30..=37 => {
                    self.fg_color = ansi_color(code - 30, self.bold);
                }
                40..=47 => {
                    self.bg_color = ansi_color(code - 40, false);
                }
                90..=97 => {
                    self.fg_color = ansi_color(code - 90, true);
                }
                100..=107 => {
                    self.bg_color = ansi_color(code - 100, true);
                }
                39 => self.fg_color = DEFAULT_FG,
                49 => self.bg_color = DEFAULT_BG,
                _ => {}
            }
        }
    }

    fn put_printable(&mut self, ch: u8) {
        if self.col >= self.cols() {
            self.new_line();
        }
        let idx = self.cell_index(self.col, self.row);
        self.cells[idx] = TextCell {
            ch,
            fg: self.fg_color,
            bg: self.bg_color,
        };
        self.render_cell(self.col, self.row, false);
        self.col += 1;
        if self.col >= self.cols() {
            self.new_line();
        }
    }

    fn tab(&mut self) {
        let target = ((self.col / 8) + 1) * 8;
        while self.col < target.min(self.cols()) {
            self.put_printable(b' ');
        }
        if self.col >= self.cols() {
            self.new_line();
        }
    }

    fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        }
    }

    fn new_line(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row >= self.rows() {
            self.scroll_up();
            self.row = self.rows() - 1;
        }
    }

    fn scroll_up(&mut self) {
        let cols = self.cols();
        let rows = self.rows();
        if rows <= 1 {
            self.clear_line(2);
            return;
        }

        for row in 1..rows {
            for col in 0..cols {
                let src = self.cell_index(col, row);
                let dst = self.cell_index(col, row - 1);
                self.cells[dst] = self.cells[src];
            }
        }
        let blank = TextCell {
            ch: b' ',
            fg: self.fg_color,
            bg: self.bg_color,
        };
        for col in 0..cols {
            let idx = self.cell_index(col, rows - 1);
            self.cells[idx] = blank;
        }

        let glyph_h = font::GLYPH_HEIGHT;
        let pitch = self.pitch as usize;
        for y in glyph_h..self.height as usize {
            let src_offset = y * pitch;
            let dst_offset = (y - glyph_h) * pitch;
            unsafe {
                core::ptr::copy(
                    self.fb_addr.add(src_offset),
                    self.fb_addr.add(dst_offset),
                    pitch,
                );
            }
        }
        for col in 0..cols {
            self.render_cell(col, rows - 1, false);
        }
    }

    fn clear_display(&mut self, mode: usize) {
        match mode {
            0 if self.row == 0 && self.col == 0 => self.clear_visible_display(),
            0 => {
                for row in self.row..self.rows() {
                    let start = if row == self.row { self.col } else { 0 };
                    self.clear_row_range(row, start, self.cols());
                }
            }
            1 => {
                for row in 0..=self.row {
                    let end = if row == self.row {
                        self.col.saturating_add(1)
                    } else {
                        self.cols()
                    };
                    self.clear_row_range(row, 0, end);
                }
            }
            2 => self.clear_visible_display(),
            3 => self.clear_visible_display(),
            _ => {}
        }
    }

    fn clear_visible_display(&mut self) {
        let blank = TextCell {
            ch: b' ',
            fg: self.fg_color,
            bg: self.bg_color,
        };
        for cell in &mut self.cells {
            *cell = blank;
        }
        self.fill_pixels(blank.bg);
    }

    fn clear_line(&mut self, mode: usize) {
        match mode {
            0 => self.clear_row_range(self.row, self.col, self.cols()),
            1 => self.clear_row_range(self.row, 0, self.col.saturating_add(1)),
            2 => self.clear_row_range(self.row, 0, self.cols()),
            _ => {}
        }
    }

    fn clear_row_range(&mut self, row: usize, start: usize, end: usize) {
        let end = end.min(self.cols());
        let blank = TextCell {
            ch: b' ',
            fg: self.fg_color,
            bg: self.bg_color,
        };
        for col in start.min(end)..end {
            let idx = self.cell_index(col, row);
            self.cells[idx] = blank;
            self.render_cell(col, row, false);
        }
    }

    fn delete_chars(&mut self, count: usize) {
        let cols = self.cols();
        let count = count.min(cols.saturating_sub(self.col));
        if count == 0 {
            return;
        }
        for col in self.col..cols - count {
            let src = self.cell_index(col + count, self.row);
            let dst = self.cell_index(col, self.row);
            self.cells[dst] = self.cells[src];
        }
        let blank = TextCell {
            ch: b' ',
            fg: self.fg_color,
            bg: self.bg_color,
        };
        for col in cols - count..cols {
            let idx = self.cell_index(col, self.row);
            self.cells[idx] = blank;
        }
        for col in self.col..cols {
            self.render_cell(col, self.row, false);
        }
    }

    fn move_up(&mut self, count: usize) {
        self.row = self.row.saturating_sub(count);
    }

    fn move_down(&mut self, count: usize) {
        self.row = self.row.saturating_add(count).min(self.rows() - 1);
    }

    fn move_left(&mut self, count: usize) {
        self.col = self.col.saturating_sub(count);
    }

    fn move_right(&mut self, count: usize) {
        self.col = self.col.saturating_add(count).min(self.cols() - 1);
    }

    fn set_col_1based(&mut self, col: usize) {
        self.col = col.saturating_sub(1).min(self.cols() - 1);
    }

    fn set_cursor_1based(&mut self, row: usize, col: usize) {
        self.row = row.saturating_sub(1).min(self.rows() - 1);
        self.col = col.saturating_sub(1).min(self.cols() - 1);
    }

    fn draw_cursor(&mut self) {
        if self.cursor_enabled && self.cursor_blink_on && !self.cursor_drawn {
            self.render_cell(self.col, self.row, true);
            self.cursor_drawn = true;
        }
    }

    fn erase_cursor(&mut self) {
        if self.cursor_drawn {
            self.render_cell(self.col, self.row, false);
            self.cursor_drawn = false;
        }
    }

    fn cell_index(&self, col: usize, row: usize) -> usize {
        row.min(self.rows() - 1) * self.cols() + col.min(self.cols() - 1)
    }

    fn render_cell(&self, col: usize, row: usize, cursor: bool) {
        let cell = self.cells[self.cell_index(col, row)];
        self.render_text_cell(col, row, cell, cursor);
    }

    pub fn render_text_cell(&self, col: usize, row: usize, cell: TextCell, cursor: bool) {
        let (fg, bg) = if cursor {
            (cell.bg, cell.fg)
        } else {
            (cell.fg, cell.bg)
        };
        self.render_glyph(col, row, cell.ch, fg, bg);
    }

    fn render_glyph(&self, col: usize, row: usize, ch: u8, fg: u32, bg: u32) {
        let Some(bytes_per_pixel) = self.bytes_per_pixel() else {
            return;
        };
        let glyph_data = font::glyph(ch);
        let px_x = col * font::GLYPH_WIDTH;
        let px_y = row * font::GLYPH_HEIGHT;

        if self.bpp == 32 {
            for (scanline, &glyph_byte) in glyph_data.iter().enumerate() {
                let row_offset = (px_y + scanline) * self.pitch as usize + px_x * 4;
                unsafe {
                    let row = self.fb_addr.add(row_offset) as *mut u32;
                    for bit in 0..font::GLYPH_WIDTH {
                        let is_fg = (glyph_byte >> (7 - bit)) & 1 == 1;
                        row.add(bit).write(if is_fg { fg } else { bg });
                    }
                }
            }
            return;
        }

        for (scanline, &glyph_byte) in glyph_data.iter().enumerate() {
            for bit in 0..font::GLYPH_WIDTH {
                let is_fg = (glyph_byte >> (7 - bit)) & 1 == 1;
                let color = if is_fg { fg } else { bg };
                self.put_pixel(px_x + bit, px_y + scanline, color, bytes_per_pixel);
            }
        }
    }

    fn fill_pixels(&self, color: u32) {
        let Some(bytes_per_pixel) = self.bytes_per_pixel() else {
            return;
        };
        if self.bpp == 32 {
            for y in 0..self.height as usize {
                let row_offset = y * self.pitch as usize;
                unsafe {
                    let row = self.fb_addr.add(row_offset) as *mut u32;
                    for x in 0..self.width as usize {
                        row.add(x).write(color);
                    }
                }
            }
            return;
        }
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                self.put_pixel(x, y, color, bytes_per_pixel);
            }
        }
    }

    fn put_pixel(&self, x: usize, y: usize, color: u32, bytes_per_pixel: usize) {
        if !(3..=4).contains(&bytes_per_pixel) {
            return;
        }
        let Some(row_offset) = y.checked_mul(self.pitch as usize) else {
            return;
        };
        let Some(pixel_offset) = x.checked_mul(bytes_per_pixel) else {
            return;
        };
        let Some(offset) = row_offset.checked_add(pixel_offset) else {
            return;
        };
        unsafe {
            let pixel_ptr = self.fb_addr.add(offset);
            core::ptr::write_volatile(pixel_ptr, color as u8);
            core::ptr::write_volatile(pixel_ptr.add(1), (color >> 8) as u8);
            core::ptr::write_volatile(pixel_ptr.add(2), (color >> 16) as u8);
            if bytes_per_pixel == 4 {
                core::ptr::write_volatile(pixel_ptr.add(3), 0);
            }
        }
    }

    fn bytes_per_pixel(&self) -> Option<usize> {
        match self.bpp {
            24 => Some(3),
            32 => Some(4),
            _ => None,
        }
    }
}

impl fmt::Write for FramebufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_char(byte);
        }
        Ok(())
    }
}

fn ansi_color(index: usize, bright: bool) -> u32 {
    const NORMAL: [u32; 8] = [
        0x0000_0000,
        0x00aa_0000,
        0x0000_aa00,
        0x00aa_5500,
        0x0000_00aa,
        0x00aa_00aa,
        0x0000_aaaa,
        0x00aa_aaaa,
    ];
    const BRIGHT: [u32; 8] = [
        0x0055_5555,
        0x00ff_5555,
        0x0055_ff55,
        0x00ff_ff55,
        0x0055_55ff,
        0x00ff_55ff,
        0x0055_ffff,
        0x00ff_ffff,
    ];
    let idx = index.min(7);
    if bright { BRIGHT[idx] } else { NORMAL[idx] }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec;
    use std::vec::Vec;

    fn make_test_writer() -> (Vec<u8>, FramebufferWriter) {
        make_test_writer_with_cells(10, 3)
    }

    fn make_test_writer_with_cells(cols: usize, rows: usize) -> (Vec<u8>, FramebufferWriter) {
        let width = (cols * font::GLYPH_WIDTH) as u32;
        let height = (rows * font::GLYPH_HEIGHT) as u32;
        let bpp = 32u8;
        let pitch = width * (bpp as u32 / 8);
        let buf_size = (pitch * height) as usize;
        let mut buf = vec![0u8; buf_size];
        let writer = unsafe { FramebufferWriter::new(buf.as_mut_ptr(), pitch, width, height, bpp) };
        (buf, writer)
    }

    #[test]
    fn dimensions_are_correct() {
        let (_buf, writer) = make_test_writer();
        assert_eq!(writer.cols(), 10);
        assert_eq!(writer.rows(), 3);
    }

    #[test]
    fn write_char_advances_cursor() {
        let (_buf, mut writer) = make_test_writer();
        writer.write_char(b'A');
        assert_eq!(writer.col, 1);
        assert_eq!(writer.row, 0);
    }

    #[test]
    fn newline_advances_row() {
        let (_buf, mut writer) = make_test_writer();
        writer.write_char(b'A');
        writer.write_char(b'\n');
        assert_eq!(writer.col, 0);
        assert_eq!(writer.row, 1);
    }

    #[test]
    fn line_wrap_at_max_cols() {
        let (_buf, mut writer) = make_test_writer();
        for _ in 0..10 {
            writer.write_char(b'X');
        }
        writer.write_char(b'Y');
        assert_eq!(writer.row, 1);
        assert_eq!(writer.col, 1);
    }

    #[test]
    fn scroll_at_bottom() {
        let (_buf, mut writer) = make_test_writer();
        for _ in 0..3 {
            writer.write_char(b'\n');
        }
        assert_eq!(writer.row, 2);
    }

    #[test]
    fn fmt_write_trait_works() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "hi").unwrap();
        assert_eq!(writer.col, 2);
    }

    #[test]
    fn unsupported_bpp_does_not_write_pixels() {
        let mut buf = vec![0u8; 4];
        let writer = unsafe { FramebufferWriter::new(buf.as_mut_ptr(), 4, 1, 1, 8) };

        writer.fill_pixels(0x00ff_ffff);

        assert_eq!(buf, vec![0u8; 4]);
    }

    #[test]
    fn glyph_renders_nonzero_pixels_for_a() {
        let (buf, mut writer) = make_test_writer();
        writer.set_colors(0x00ff_ffff, 0x0000_0000);
        writer.write_char(b'A');

        let bpp = 4;
        let mut has_fg = false;
        for y in 0..font::GLYPH_HEIGHT {
            for x in 0..font::GLYPH_WIDTH {
                let offset = y * writer.pitch as usize + x * bpp;
                let r = buf[offset + 2];
                if r > 0 {
                    has_fg = true;
                }
            }
        }
        assert!(has_fg, "glyph 'A' should have foreground pixels");
    }

    #[test]
    fn cursor_redraw_preserves_underlying_cell() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "A\x1b[1;1H").unwrap();
        assert_eq!(writer.cell_char(0, 0), b'A');
        writer.write_char(b'B');
        assert_eq!(writer.cell_char(0, 0), b'B');
        assert_eq!(writer.col, 1);
    }

    #[test]
    fn cursor_blink_phase_erases_and_restores_without_moving() {
        let (_buf, mut writer) = make_test_writer();
        writer.write_char(b'A');
        assert!(writer.cursor_drawn);
        assert_eq!((writer.row, writer.col), (0, 1));

        writer.set_cursor_blink_on(false);
        assert!(!writer.cursor_drawn);
        assert_eq!(writer.cell_char(0, 0), b'A');
        assert_eq!((writer.row, writer.col), (0, 1));

        writer.set_cursor_blink_on(true);
        assert!(writer.cursor_drawn);
        assert_eq!(writer.cell_char(0, 0), b'A');
        assert_eq!((writer.row, writer.col), (0, 1));
    }

    #[test]
    fn ansi_clear_and_cursor_home_work() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "abc\nxyz\x1b[H\x1b[2J").unwrap();
        for row in 0..writer.rows() {
            for col in 0..writer.cols() {
                assert_eq!(writer.cell_char(row, col), b' ');
            }
        }
        assert_eq!((writer.row, writer.col), (0, 0));
    }

    #[test]
    fn ansi_clear_scrollback_clears_visible_cells_like_linux_vt() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "abc\x1b[3J").unwrap();
        for row in 0..writer.rows() {
            for col in 0..writer.cols() {
                assert_eq!(writer.cell_char(row, col), b' ');
            }
        }
        assert_eq!((writer.row, writer.col), (0, 3));
    }

    #[test]
    fn render_batch_clear_fills_cells_and_pixels_without_glyph_redraw() {
        let (buf, mut writer) = make_test_writer();
        let blank = TextCell {
            ch: b' ',
            fg: 0x00ff_ffff,
            bg: 0x0000_00aa,
        };

        writer.clear_visible_cells(blank, Some((0, 0)));

        for row in 0..writer.rows() {
            for col in 0..writer.cols() {
                assert_eq!(writer.cells[writer.cell_index(col, row)], blank);
            }
        }
        let second_cell = font::GLYPH_WIDTH * 4;
        assert_eq!(&buf[0..4], &[0xff, 0xff, 0xff, 0]);
        assert_eq!(&buf[second_cell..second_cell + 4], &[0xaa, 0x00, 0x00, 0]);
    }

    #[test]
    fn ncurses_clear_sequence_clears_visible_once() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "abc\nxyz\x1b[H\x1b[J\x1b[3J").unwrap();
        for row in 0..writer.rows() {
            for col in 0..writer.cols() {
                assert_eq!(writer.cell_char(row, col), b' ');
            }
        }
        assert_eq!((writer.row, writer.col), (0, 0));
    }

    #[test]
    fn readline_backspace_pattern_overwrites_character() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "ab\x08 \x08c").unwrap();
        assert_eq!(writer.cell_char(0, 0), b'a');
        assert_eq!(writer.cell_char(0, 1), b'c');
        assert_eq!(writer.col, 2);
    }

    #[test]
    fn ansi_delete_character_shifts_line_left() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "abcd\x1b[1;2H\x1b[P").unwrap();
        assert_eq!(writer.cell_char(0, 0), b'a');
        assert_eq!(writer.cell_char(0, 1), b'c');
        assert_eq!(writer.cell_char(0, 2), b'd');
        assert_eq!(writer.cell_char(0, 3), b' ');
    }

    #[test]
    fn sgr_green_and_reset_update_cell_colors() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "\x1b[32mO\x1b[0mK").unwrap();
        let green = writer.cells[writer.cell_index(0, 0)].fg;
        let reset = writer.cells[writer.cell_index(1, 0)].fg;
        assert_eq!(green, 0x0000_aa00);
        assert_eq!(reset, DEFAULT_FG);
        assert_eq!(writer.cell_char(0, 0), b'O');
        assert_eq!(writer.cell_char(0, 1), b'K');
    }

    #[test]
    fn sgr_bold_uses_bright_ansi_colors_for_prompt_text() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "\x1b[1;32mL\x1b[22;32mx\x1b[0m").unwrap();
        let bold_green = writer.cells[writer.cell_index(0, 0)].fg;
        let normal_green = writer.cells[writer.cell_index(1, 0)].fg;
        assert_eq!(bold_green, 0x0055_ff55);
        assert_eq!(normal_green, 0x0000_aa00);
        assert_eq!(writer.cell_char(0, 0), b'L');
        assert_eq!(writer.cell_char(0, 1), b'x');
    }

    #[test]
    fn bash_prompt_sgr_sequences_color_cells_without_artifacts() {
        let (_buf, mut writer) = make_test_writer_with_cells(20, 3);
        use core::fmt::Write;
        write!(
            writer,
            "\x1b[01;32m[root@lupos\x1b[00m \x1b[01;34m/\x1b[00m]# "
        )
        .unwrap();

        for (col, byte) in b"[root@lupos /]# ".iter().enumerate() {
            assert_eq!(writer.cell_char(0, col), *byte);
        }
        assert_eq!(writer.cells[writer.cell_index(0, 0)].fg, 0x0055_ff55);
        assert_eq!(writer.cells[writer.cell_index(10, 0)].fg, 0x0055_ff55);
        assert_eq!(writer.cells[writer.cell_index(11, 0)].fg, DEFAULT_FG);
        assert_eq!(writer.cells[writer.cell_index(12, 0)].fg, 0x0055_55ff);
        assert_eq!(writer.cells[writer.cell_index(13, 0)].fg, DEFAULT_FG);
    }

    #[test]
    fn systemd_terminal_reset_controls_do_not_render_artifacts() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(writer, "\x1b[!p\x1b]104\x07\x1b[?7hOK").unwrap();

        assert_eq!(writer.cell_char(0, 0), b'O');
        assert_eq!(writer.cell_char(0, 1), b'K');
        assert_eq!(writer.cell_char(0, 2), b' ');
        assert_eq!(writer.col, 2);
    }

    #[test]
    fn terminal_cursor_probe_reset_rehomes_before_banner() {
        let (_buf, mut writer) = make_test_writer();
        use core::fmt::Write;
        write!(
            writer,
            "old\nlogs\n\x1b[18t\x1b[6n\x1b[32766;32766H\x1b[6n\x1b[!p\
             \x1b]104\x07\x1b[0m\x1b[?7h\x1b[1G\x1b[0J\x1bP+q6E616D65\x1b\\OK"
        )
        .unwrap();

        assert_eq!(writer.cell_char(0, 0), b'O');
        assert_eq!(writer.cell_char(0, 1), b'K');
        assert_eq!(writer.cell_char(0, 2), b' ');
        assert_eq!(writer.col, 2);
        assert_eq!(writer.row, 0);
    }
}
