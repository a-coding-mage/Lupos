//! linux-parity: partial
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Linux-like console router and VT text buffer.
//!
//! The hot `/dev/console` write path feeds bytes into an in-memory VT grid and
//! the serial transmit queue. Pixel drawing is deferred to maintenance/flush
//! points, which mirrors Linux's tty/vt/fbcon split closely enough for the
//! current single-console boot path.

extern crate alloc;

use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::linux_driver_abi::video::fbdev::core::font;
use crate::linux_driver_abi::video::fbdev::core::writer::{DEFAULT_BG, DEFAULT_FG, TextCell};

const DEFAULT_COLS: usize = 80;
const DEFAULT_ROWS: usize = 25;
const SERIAL_BUDGET: usize = 4096;
const FBCON_ROW_BUDGET: usize = 8;

lazy_static! {
    static ref CONSOLE: Mutex<Option<VirtualConsole>> = Mutex::new(None);
}

static FBCON_ENABLED: AtomicBool = AtomicBool::new(true);
/// Serializes host tests that mutate global console/fbcon state, including
/// framebuffer detach tests in sibling modules.
#[cfg(test)]
pub(crate) static TEST_CONSOLE_LOCK: spin::Mutex<()> = spin::Mutex::new(());
static CURSOR_BLINK_ON: AtomicBool = AtomicBool::new(true);
static CURSOR_LAST_TSC: AtomicU64 = AtomicU64::new(0);
const CURSOR_BLINK_CYCLES: u64 = 250_000_000;

/// TSC deadline at which the in-progress console bell is silenced, or `0`
/// when the speaker is idle. Driven from the console write path and cleared
/// by [`bell_tick`] on the maintenance pump.
static BELL_DEADLINE_TSC: AtomicU64 = AtomicU64::new(0);
/// Linux `DEFAULT_BELL_PITCH` (drivers/tty/vt/vt.c) — 750 Hz.
const BELL_PITCH_HZ: u32 = 750;
/// Bell ring length in TSC cycles. Linux uses `DEFAULT_BELL_DURATION = HZ/8`
/// (~125 ms); scaled here to a quarter of the cursor-blink interval to match
/// the same TSC assumption used for [`CURSOR_BLINK_CYCLES`].
const BELL_DURATION_CYCLES: u64 = CURSOR_BLINK_CYCLES / 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirtyRow {
    pub row: usize,
    pub cells: Vec<TextCell>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClearOp {
    pub blank: TextCell,
    pub flush_scrollback: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderBatch {
    pub cols: usize,
    pub rows: usize,
    pub clear: Option<ClearOp>,
    pub dirty_rows: Vec<DirtyRow>,
    pub cursor: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EscapeState {
    Ground,
    Escape,
    CharsetDesignation,
    Csi,
    Osc,
    OscEscape,
    Dcs,
    DcsEscape,
}

#[derive(Clone, Copy, Debug)]
struct SavedCursor {
    col: usize,
    row: usize,
    fg_color: u32,
    bg_color: u32,
    bold: bool,
    reverse: bool,
}

#[derive(Clone, Debug)]
struct VirtualConsole {
    cols: usize,
    rows: usize,
    row_head: usize,
    col: usize,
    row: usize,
    fg_color: u32,
    bg_color: u32,
    bold: bool,
    reverse: bool,
    cells: Vec<TextCell>,
    dirty_rows: Vec<bool>,
    pending_clear: Option<ClearOp>,
    cursor_enabled: bool,
    cursor_blink_on: bool,
    insert_mode: bool,
    need_wrap: bool,
    scroll_top: usize,
    scroll_bottom: usize,
    saved_cursor: SavedCursor,
    esc_state: EscapeState,
    csi_params: [usize; 4],
    csi_count: usize,
    csi_value: usize,
    csi_have_digit: bool,
    csi_private: bool,
    csi_intermediate: u8,
}

impl VirtualConsole {
    fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            cols,
            rows,
            row_head: 0,
            col: 0,
            row: 0,
            fg_color: DEFAULT_FG,
            bg_color: DEFAULT_BG,
            bold: false,
            reverse: false,
            cells: vec![TextCell::default(); cols * rows],
            dirty_rows: vec![true; rows],
            pending_clear: None,
            cursor_enabled: true,
            cursor_blink_on: true,
            insert_mode: false,
            need_wrap: false,
            scroll_top: 0,
            scroll_bottom: rows,
            saved_cursor: SavedCursor {
                col: 0,
                row: 0,
                fg_color: DEFAULT_FG,
                bg_color: DEFAULT_BG,
                bold: false,
                reverse: false,
            },
            esc_state: EscapeState::Ground,
            csi_params: [0; 4],
            csi_count: 0,
            csi_value: 0,
            csi_have_digit: false,
            csi_private: false,
            csi_intermediate: 0,
        }
    }

    fn resize_if_needed(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if self.cols == cols && self.rows == rows {
            return;
        }
        *self = Self::new(cols, rows);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_byte_inner(byte);
        }
    }

    fn write_byte_inner(&mut self, byte: u8) {
        match self.esc_state {
            EscapeState::Ground => self.write_ground(byte),
            EscapeState::Escape => self.write_escape(byte),
            EscapeState::CharsetDesignation => {
                self.esc_state = EscapeState::Ground;
            }
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
            0x07 => ring_bell(),
            b'\n' => self.new_line(),
            b'\r' => self.set_col(0),
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
            b'(' | b')' => self.esc_state = EscapeState::CharsetDesignation,
            b'D' => {
                self.line_feed();
                self.esc_state = EscapeState::Ground;
            }
            b'E' => {
                self.new_line();
                self.esc_state = EscapeState::Ground;
            }
            b'M' => {
                self.reverse_index();
                self.esc_state = EscapeState::Ground;
            }
            b'7' => {
                self.save_cursor();
                self.esc_state = EscapeState::Ground;
            }
            b'8' => {
                self.restore_cursor();
                self.esc_state = EscapeState::Ground;
            }
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
            b'd' => self.set_row_1based(self.csi_param(0, 1)),
            b'H' | b'f' => self.set_cursor_1based(self.csi_param(0, 1), self.csi_param(1, 1)),
            b'J' => self.clear_display(self.csi_param(0, 0)),
            b'K' => self.clear_line(self.csi_param(0, 0)),
            b'L' => self.insert_lines(self.csi_param(0, 1)),
            b'M' => self.delete_lines(self.csi_param(0, 1)),
            b'@' => self.insert_chars(self.csi_param(0, 1)),
            b'P' => self.delete_chars(self.csi_param(0, 1)),
            b'X' => self.erase_chars(self.csi_param(0, 1)),
            b'n' if !self.csi_private && self.csi_param(0, 0) == 6 => self.report_cursor_position(),
            b't' if !self.csi_private && self.csi_param(0, 0) == 18 => self.report_text_area_size(),
            b'r' if !self.csi_private => {
                self.set_scroll_region(self.csi_param(0, 1), self.csi_param(1, self.rows));
            }
            b's' if !self.csi_private => self.save_cursor(),
            b'u' if !self.csi_private => self.restore_cursor(),
            b'h' if !self.csi_private && self.csi_param(0, 0) == 4 => {
                self.insert_mode = true;
            }
            b'l' if !self.csi_private && self.csi_param(0, 0) == 4 => {
                self.insert_mode = false;
            }
            b'h' if self.csi_private && self.csi_param(0, 0) == 25 => {
                self.cursor_enabled = true;
                self.mark_cursor_dirty();
            }
            b'l' if self.csi_private && self.csi_param(0, 0) == 25 => {
                self.cursor_enabled = false;
                self.mark_cursor_dirty();
            }
            b'm' => self.apply_sgr(),
            _ => {}
        }
    }

    fn report_cursor_position(&self) {
        let row = self.row.min(self.rows - 1) + 1;
        let col = self.col.min(self.cols - 1) + 1;
        let response = format!("\x1b[{row};{col}R");
        crate::init::rootfs::queue_console_input_response(response.as_bytes());
    }

    fn report_text_area_size(&self) {
        let response = format!("\x1b[8;{};{}t", self.rows, self.cols);
        crate::init::rootfs::queue_console_input_response(response.as_bytes());
    }

    fn soft_reset(&mut self) {
        self.mark_cursor_dirty();
        self.fg_color = DEFAULT_FG;
        self.bg_color = DEFAULT_BG;
        self.bold = false;
        self.reverse = false;
        self.cursor_enabled = true;
        self.insert_mode = false;
        self.need_wrap = false;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows;
        self.col = 0;
        self.row = 0;
        self.saved_cursor = SavedCursor {
            col: 0,
            row: 0,
            fg_color: DEFAULT_FG,
            bg_color: DEFAULT_BG,
            bold: false,
            reverse: false,
        };
        self.mark_cursor_dirty();
    }

    fn apply_sgr(&mut self) {
        if self.csi_count == 0 {
            self.fg_color = DEFAULT_FG;
            self.bg_color = DEFAULT_BG;
            self.bold = false;
            self.reverse = false;
            return;
        }
        for idx in 0..self.csi_count {
            let code = self.csi_params[idx];
            match code {
                0 => {
                    self.fg_color = DEFAULT_FG;
                    self.bg_color = DEFAULT_BG;
                    self.bold = false;
                    self.reverse = false;
                }
                1 => self.bold = true,
                7 => self.reverse = true,
                22 => self.bold = false,
                27 => self.reverse = false,
                30..=37 => self.fg_color = ansi_color(code - 30, self.bold),
                40..=47 => self.bg_color = ansi_color(code - 40, false),
                90..=97 => self.fg_color = ansi_color(code - 90, true),
                100..=107 => self.bg_color = ansi_color(code - 100, true),
                39 => self.fg_color = DEFAULT_FG,
                49 => self.bg_color = DEFAULT_BG,
                _ => {}
            }
        }
    }

    fn current_cell(&self, ch: u8) -> TextCell {
        let (fg, bg) = if self.reverse {
            (self.bg_color, self.fg_color)
        } else {
            (self.fg_color, self.bg_color)
        };
        TextCell { ch, fg, bg }
    }

    fn blank_cell(&self) -> TextCell {
        self.current_cell(b' ')
    }

    fn put_printable(&mut self, ch: u8) {
        if self.need_wrap {
            self.new_line();
        }
        if self.insert_mode {
            self.insert_chars(1);
        }
        self.mark_cursor_dirty();
        let idx = self.cell_index(self.col, self.row);
        let cell = self.current_cell(ch);
        self.cells[idx] = cell;
        self.mark_dirty(self.row);
        if self.col == self.cols - 1 {
            self.need_wrap = true;
        } else {
            self.col += 1;
        }
        self.mark_cursor_dirty();
    }

    fn insert_chars(&mut self, count: usize) {
        self.need_wrap = false;
        let count = count.min(self.cols.saturating_sub(self.col));
        if count == 0 {
            return;
        }
        self.mark_cursor_dirty();
        for col in (self.col + count..self.cols).rev() {
            let src = self.cell_index(col - count, self.row);
            let dst = self.cell_index(col, self.row);
            self.cells[dst] = self.cells[src];
        }
        let blank = self.blank_cell();
        for col in self.col..self.col + count {
            let idx = self.cell_index(col, self.row);
            self.cells[idx] = blank;
        }
        self.mark_dirty(self.row);
        self.mark_cursor_dirty();
    }

    fn tab(&mut self) {
        self.mark_cursor_dirty();
        let target = ((self.col / 8) + 1) * 8;
        self.col = target.min(self.cols - 1);
        self.need_wrap = false;
        self.mark_cursor_dirty();
    }

    fn backspace(&mut self) {
        self.need_wrap = false;
        if self.col > 0 {
            self.mark_cursor_dirty();
            self.col -= 1;
            self.mark_cursor_dirty();
        }
    }

    fn new_line(&mut self) {
        self.mark_cursor_dirty();
        self.col = 0;
        self.need_wrap = false;
        self.advance_line();
        self.mark_cursor_dirty();
    }

    fn line_feed(&mut self) {
        self.mark_cursor_dirty();
        self.need_wrap = false;
        self.advance_line();
        self.mark_cursor_dirty();
    }

    fn advance_line(&mut self) {
        if self.row + 1 == self.scroll_bottom {
            self.scroll_region_up(self.scroll_top, self.scroll_bottom, 1);
            self.row = self.scroll_bottom - 1;
        } else if self.row < self.rows - 1 {
            self.row += 1;
        }
    }

    fn reverse_index(&mut self) {
        self.mark_cursor_dirty();
        self.need_wrap = false;
        if self.row == self.scroll_top {
            self.scroll_region_down(self.scroll_top, self.scroll_bottom, 1);
        } else if self.row > 0 {
            self.row -= 1;
        }
        self.mark_cursor_dirty();
    }

    fn scroll_region_up(&mut self, top: usize, bottom: usize, count: usize) {
        if top >= bottom || bottom > self.rows {
            return;
        }
        let count = count.min(bottom - top);
        if count == 0 {
            return;
        }

        if top == 0 && bottom == self.rows {
            self.row_head = (self.row_head + count) % self.rows;
        } else {
            for row in top..bottom - count {
                for col in 0..self.cols {
                    let src = self.cell_index(col, row + count);
                    let dst = self.cell_index(col, row);
                    let cell = self.cells[src];
                    self.cells[dst] = cell;
                }
            }
        }

        let blank = self.blank_cell();
        for row in bottom - count..bottom {
            for col in 0..self.cols {
                let idx = self.cell_index(col, row);
                self.cells[idx] = blank;
            }
        }
        for row in top..bottom {
            self.mark_dirty(row);
        }
    }

    fn scroll_region_down(&mut self, top: usize, bottom: usize, count: usize) {
        if top >= bottom || bottom > self.rows {
            return;
        }
        let count = count.min(bottom - top);
        if count == 0 {
            return;
        }

        if top == 0 && bottom == self.rows {
            self.row_head = (self.row_head + self.rows - count) % self.rows;
        } else {
            for row in (top + count..bottom).rev() {
                for col in 0..self.cols {
                    let src = self.cell_index(col, row - count);
                    let dst = self.cell_index(col, row);
                    let cell = self.cells[src];
                    self.cells[dst] = cell;
                }
            }
        }

        let blank = self.blank_cell();
        for row in top..top + count {
            for col in 0..self.cols {
                let idx = self.cell_index(col, row);
                self.cells[idx] = blank;
            }
        }
        for row in top..bottom {
            self.mark_dirty(row);
        }
    }

    fn clear(&mut self) {
        self.fg_color = DEFAULT_FG;
        self.bg_color = DEFAULT_BG;
        self.bold = false;
        self.reverse = false;
        self.cursor_enabled = true;
        self.insert_mode = false;
        self.need_wrap = false;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows;
        let blank = self.blank_cell();
        for cell in &mut self.cells {
            *cell = blank;
        }
        self.col = 0;
        self.row = 0;
        self.row_head = 0;
        self.saved_cursor = SavedCursor {
            col: 0,
            row: 0,
            fg_color: DEFAULT_FG,
            bg_color: DEFAULT_BG,
            bold: false,
            reverse: false,
        };
        self.esc_state = EscapeState::Ground;
        self.queue_full_clear(blank, false);
    }

    fn clear_display(&mut self, mode: usize) {
        self.need_wrap = false;
        match mode {
            0 if self.row == 0 && self.col == 0 => self.clear_visible_display(),
            0 => {
                for row in self.row..self.rows {
                    let start = if row == self.row { self.col } else { 0 };
                    self.clear_row_range(row, start, self.cols);
                }
            }
            1 => {
                for row in 0..=self.row {
                    let end = if row == self.row {
                        self.col.saturating_add(1)
                    } else {
                        self.cols
                    };
                    self.clear_row_range(row, 0, end);
                }
            }
            2 => self.clear_visible_display(),
            3 => self.clear_visible_display_and_scrollback(),
            _ => {}
        }
    }

    fn clear_visible_display(&mut self) {
        self.clear_visible_display_with_scrollback(false);
    }

    fn clear_visible_display_and_scrollback(&mut self) {
        self.clear_visible_display_with_scrollback(true);
    }

    fn clear_visible_display_with_scrollback(&mut self, flush_scrollback: bool) {
        let blank = self.blank_cell();
        for cell in &mut self.cells {
            *cell = blank;
        }
        self.queue_full_clear(blank, flush_scrollback);
    }

    fn clear_line(&mut self, mode: usize) {
        self.need_wrap = false;
        match mode {
            0 => self.clear_row_range(self.row, self.col, self.cols),
            1 => self.clear_row_range(self.row, 0, self.col.saturating_add(1)),
            2 => self.clear_row_range(self.row, 0, self.cols),
            _ => {}
        }
    }

    fn clear_row_range(&mut self, row: usize, start: usize, end: usize) {
        let end = end.min(self.cols);
        let blank = self.blank_cell();
        for col in start.min(end)..end {
            let idx = self.cell_index(col, row);
            self.cells[idx] = blank;
        }
        self.mark_dirty(row);
    }

    fn delete_chars(&mut self, count: usize) {
        self.need_wrap = false;
        let count = count.min(self.cols.saturating_sub(self.col));
        if count == 0 {
            return;
        }
        self.mark_cursor_dirty();
        for col in self.col..self.cols - count {
            let src = self.cell_index(col + count, self.row);
            let dst = self.cell_index(col, self.row);
            self.cells[dst] = self.cells[src];
        }
        let blank = self.blank_cell();
        for col in self.cols - count..self.cols {
            let idx = self.cell_index(col, self.row);
            self.cells[idx] = blank;
        }
        self.mark_dirty(self.row);
        self.mark_cursor_dirty();
    }

    fn erase_chars(&mut self, count: usize) {
        self.need_wrap = false;
        let count = count.min(self.cols.saturating_sub(self.col));
        if count == 0 {
            return;
        }
        let blank = self.blank_cell();
        for col in self.col..self.col + count {
            let idx = self.cell_index(col, self.row);
            self.cells[idx] = blank;
        }
        self.mark_dirty(self.row);
    }

    fn insert_lines(&mut self, count: usize) {
        self.need_wrap = false;
        if !(self.scroll_top..self.scroll_bottom).contains(&self.row) {
            return;
        }
        self.mark_cursor_dirty();
        self.scroll_region_down(self.row, self.scroll_bottom, count);
        self.mark_cursor_dirty();
    }

    fn delete_lines(&mut self, count: usize) {
        self.need_wrap = false;
        if !(self.scroll_top..self.scroll_bottom).contains(&self.row) {
            return;
        }
        self.mark_cursor_dirty();
        self.scroll_region_up(self.row, self.scroll_bottom, count);
        self.mark_cursor_dirty();
    }

    fn set_scroll_region(&mut self, top_1based: usize, bottom_1based: usize) {
        if top_1based >= bottom_1based || bottom_1based > self.rows {
            return;
        }
        let top = top_1based.saturating_sub(1);
        let bottom = bottom_1based;
        self.mark_cursor_dirty();
        self.scroll_top = top;
        self.scroll_bottom = bottom;
        self.col = 0;
        self.row = 0;
        self.need_wrap = false;
        self.mark_cursor_dirty();
    }

    fn save_cursor(&mut self) {
        self.saved_cursor = SavedCursor {
            col: self.col,
            row: self.row,
            fg_color: self.fg_color,
            bg_color: self.bg_color,
            bold: self.bold,
            reverse: self.reverse,
        };
    }

    fn restore_cursor(&mut self) {
        self.mark_cursor_dirty();
        self.col = self.saved_cursor.col.min(self.cols - 1);
        self.row = self.saved_cursor.row.min(self.rows - 1);
        self.fg_color = self.saved_cursor.fg_color;
        self.bg_color = self.saved_cursor.bg_color;
        self.bold = self.saved_cursor.bold;
        self.reverse = self.saved_cursor.reverse;
        self.need_wrap = false;
        self.mark_cursor_dirty();
    }

    fn move_up(&mut self, count: usize) {
        self.set_row(self.row.saturating_sub(count));
    }

    fn move_down(&mut self, count: usize) {
        self.set_row(self.row.saturating_add(count).min(self.rows - 1));
    }

    fn move_left(&mut self, count: usize) {
        self.set_col(self.col.saturating_sub(count));
    }

    fn move_right(&mut self, count: usize) {
        self.set_col(self.col.saturating_add(count).min(self.cols - 1));
    }

    fn set_col_1based(&mut self, col: usize) {
        self.set_col(col.saturating_sub(1).min(self.cols - 1));
    }

    fn set_row_1based(&mut self, row: usize) {
        self.set_row(row.saturating_sub(1).min(self.rows - 1));
    }

    fn set_cursor_1based(&mut self, row: usize, col: usize) {
        self.mark_cursor_dirty();
        self.row = row.saturating_sub(1).min(self.rows - 1);
        self.col = col.saturating_sub(1).min(self.cols - 1);
        self.need_wrap = false;
        self.mark_cursor_dirty();
    }

    fn set_col(&mut self, col: usize) {
        self.mark_cursor_dirty();
        self.col = col.min(self.cols - 1);
        self.need_wrap = false;
        self.mark_cursor_dirty();
    }

    fn set_row(&mut self, row: usize) {
        self.mark_cursor_dirty();
        self.row = row.min(self.rows - 1);
        self.need_wrap = false;
        self.mark_cursor_dirty();
    }

    fn set_cursor_blink_on(&mut self, on: bool) {
        if self.cursor_blink_on != on {
            self.cursor_blink_on = on;
            self.mark_cursor_dirty();
        }
    }

    fn cursor(&self) -> Option<(usize, usize)> {
        if self.cursor_enabled && self.cursor_blink_on {
            Some((self.col.min(self.cols - 1), self.row.min(self.rows - 1)))
        } else {
            None
        }
    }

    fn take_dirty_rows(&mut self, budget: usize) -> Option<RenderBatch> {
        let clear = self.pending_clear.take();
        let mut dirty_rows = Vec::new();
        for row in 0..self.rows {
            if dirty_rows.len() >= budget {
                break;
            }
            if !self.dirty_rows[row] {
                continue;
            }
            let mut cells = Vec::with_capacity(self.cols);
            for col in 0..self.cols {
                cells.push(self.cells[self.cell_index(col, row)]);
            }
            self.dirty_rows[row] = false;
            dirty_rows.push(DirtyRow { row, cells });
        }
        if clear.is_none() && dirty_rows.is_empty() {
            None
        } else {
            Some(RenderBatch {
                cols: self.cols,
                rows: self.rows,
                clear,
                dirty_rows,
                cursor: self.cursor(),
            })
        }
    }

    fn mark_dirty(&mut self, row: usize) {
        if row < self.dirty_rows.len() {
            self.dirty_rows[row] = true;
        }
    }

    fn mark_cursor_dirty(&mut self) {
        self.mark_dirty(self.row);
    }

    fn mark_all_dirty(&mut self) {
        for dirty in &mut self.dirty_rows {
            *dirty = true;
        }
    }

    fn queue_full_clear(&mut self, blank: TextCell, flush_scrollback: bool) {
        let flush_scrollback = flush_scrollback
            || self
                .pending_clear
                .map(|clear| clear.flush_scrollback)
                .unwrap_or(false);
        self.pending_clear = Some(ClearOp {
            blank,
            flush_scrollback,
        });
        for dirty in &mut self.dirty_rows {
            *dirty = false;
        }
    }

    fn cell_index(&self, col: usize, row: usize) -> usize {
        let physical_row = (self.row_head + row.min(self.rows - 1)) % self.rows;
        physical_row * self.cols + col.min(self.cols - 1)
    }

    #[cfg(test)]
    fn cell_char(&self, row: usize, col: usize) -> u8 {
        self.cells[self.cell_index(col, row)].ch
    }
}

pub fn init(cols: usize, rows: usize) {
    let mut guard = CONSOLE.lock();
    match guard.as_mut() {
        Some(console) => console.resize_if_needed(cols, rows),
        None => *guard = Some(VirtualConsole::new(cols, rows)),
    }
}

pub fn init_from_pixels(width: u32, height: u32) {
    init(
        (width as usize / font::GLYPH_WIDTH).max(1),
        (height as usize / font::GLYPH_HEIGHT).max(1),
    );
}

fn with_console_mut<R>(f: impl FnOnce(&mut VirtualConsole) -> R) -> R {
    let mut guard = CONSOLE.lock();
    if guard.is_none() {
        *guard = Some(VirtualConsole::new(DEFAULT_COLS, DEFAULT_ROWS));
    }
    f(guard.as_mut().unwrap())
}

pub fn write_bytes(bytes: &[u8]) {
    crate::linux_driver_abi::tty::serial::enqueue_bytes_blocking(bytes);
    write_visible_bytes(bytes);
    render_dirty_to_display(usize::MAX);
    flush_serial_budgeted();
}

pub fn write_bytes_deferred(bytes: &[u8]) {
    crate::linux_driver_abi::tty::serial::enqueue_bytes(bytes);
    write_visible_bytes(bytes);
}

pub fn write_visible_bytes(bytes: &[u8]) {
    with_console_mut(|console| console.write_bytes(bytes));
}

pub fn maintenance_budgeted() {
    flush_serial_budgeted();
    bell_tick();
    render_dirty_to_display(FBCON_ROW_BUDGET);
}

fn render_dirty_to_display(budget: usize) {
    if !FBCON_ENABLED.load(Ordering::Acquire) {
        return;
    }
    if let Some(batch) = take_dirty_batch(budget) {
        crate::linux_driver_abi::video::fbdev::core::render_batch(&batch);
    }
}

pub fn flush_serial_budgeted() {
    let _ = crate::linux_driver_abi::tty::serial::poll_input_budget(64);
    let _ = crate::linux_driver_abi::tty::serial::flush_budget(SERIAL_BUDGET);
}

pub fn flush_all_blocking() {
    crate::linux_driver_abi::tty::serial::flush_all_blocking();
    if !FBCON_ENABLED.load(Ordering::Acquire) {
        return;
    }
    loop {
        let Some(batch) = take_dirty_batch(usize::MAX) else {
            break;
        };
        crate::linux_driver_abi::video::fbdev::core::render_batch(&batch);
    }
}

pub fn flush_all_nonblocking() {
    flush_serial_budgeted();
    if !FBCON_ENABLED.load(Ordering::Acquire) {
        return;
    }
    loop {
        let Some(batch) = take_dirty_batch(usize::MAX) else {
            break;
        };
        crate::linux_driver_abi::video::fbdev::core::render_batch(&batch);
    }
}

pub fn set_fbcon_enabled(enabled: bool) {
    let was_enabled = FBCON_ENABLED.swap(enabled, Ordering::AcqRel);
    if enabled && !was_enabled {
        with_console_mut(|console| console.mark_all_dirty());
    }
}

pub fn fbcon_enabled() -> bool {
    FBCON_ENABLED.load(Ordering::Acquire)
}

pub fn refresh_cursor_blink() {
    let now = read_tsc();
    let last = CURSOR_LAST_TSC.load(Ordering::Acquire);
    if last == 0 {
        CURSOR_LAST_TSC.store(now, Ordering::Release);
        return;
    }
    if now.saturating_sub(last) < CURSOR_BLINK_CYCLES {
        return;
    }
    if CURSOR_LAST_TSC
        .compare_exchange(last, now, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let on = !CURSOR_BLINK_ON.load(Ordering::Acquire);
    CURSOR_BLINK_ON.store(on, Ordering::Release);
    with_console_mut(|console| console.set_cursor_blink_on(on));
}

/// Handle a ground-state BEL (`0x07`) like Linux's `bell()` in the VT layer:
/// start the PC speaker at the default bell pitch and arm the silence
/// deadline. The maintenance pump ([`bell_tick`]) turns the speaker back off
/// once [`BELL_DURATION_CYCLES`] have elapsed, mirroring `kd_mksound`'s
/// duration timer.
fn ring_bell() {
    #[cfg(not(test))]
    {
        crate::linux_driver_abi::input::misc::pcspkr::kd_mksound(BELL_PITCH_HZ);
        let deadline = read_tsc().saturating_add(BELL_DURATION_CYCLES).max(1);
        BELL_DEADLINE_TSC.store(deadline, Ordering::Release);
    }
    #[cfg(test)]
    BELL_DEADLINE_TSC.store(1, Ordering::Release);
}

/// Silence the console bell once its duration has elapsed. Called from the
/// maintenance pump, which also drives the cursor blink, so the speaker is
/// gated off shortly after the ring even on an otherwise idle shell.
pub fn bell_tick() {
    let deadline = BELL_DEADLINE_TSC.load(Ordering::Acquire);
    if deadline == 0 {
        return;
    }
    if read_tsc() < deadline {
        return;
    }
    if BELL_DEADLINE_TSC
        .compare_exchange(deadline, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    #[cfg(not(test))]
    crate::linux_driver_abi::input::misc::pcspkr::kd_mksound(0);
}

fn take_dirty_batch(budget: usize) -> Option<RenderBatch> {
    let mut guard = CONSOLE.lock();
    guard.as_mut()?.take_dirty_rows(budget)
}

#[inline]
fn read_tsc() -> u64 {
    #[cfg(not(test))]
    unsafe {
        let lo: u32;
        let hi: u32;
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack, preserves_flags),
        );
        ((hi as u64) << 32) | lo as u64
    }

    #[cfg(test)]
    {
        CURSOR_LAST_TSC
            .load(Ordering::Relaxed)
            .saturating_add(CURSOR_BLINK_CYCLES)
            .saturating_add(1)
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
pub fn reset_for_tests(cols: usize, rows: usize) {
    *CONSOLE.lock() = Some(VirtualConsole::new(cols, rows));
    FBCON_ENABLED.store(true, Ordering::Release);
    CURSOR_BLINK_ON.store(true, Ordering::Release);
    CURSOR_LAST_TSC.store(0, Ordering::Release);
    BELL_DEADLINE_TSC.store(0, Ordering::Release);
    crate::init::rootfs::clear_console_input_for_tests();
}

#[cfg(test)]
pub fn dirty_batch_for_tests(budget: usize) -> Option<RenderBatch> {
    take_dirty_batch(budget)
}

#[cfg(test)]
pub fn cell_char_for_tests(row: usize, col: usize) -> u8 {
    with_console_mut(|console| console.cell_char(row, col))
}

#[cfg(test)]
pub fn cursor_for_tests() -> Option<(usize, usize)> {
    with_console_mut(|console| console.cursor())
}

#[cfg(test)]
pub fn bell_armed_for_tests() -> bool {
    BELL_DEADLINE_TSC.load(Ordering::Acquire) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_updates_cells_and_dirty_rows_without_rendering() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"hi");

        assert_eq!(cell_char_for_tests(0, 0), b'h');
        assert_eq!(cell_char_for_tests(0, 1), b'i');
        let batch = dirty_batch_for_tests(1).expect("dirty row");
        assert_eq!(batch.dirty_rows.len(), 1);
        assert_eq!(batch.dirty_rows[0].row, 0);
        assert_eq!(batch.dirty_rows[0].cells[0].ch, b'h');
        assert_eq!(batch.dirty_rows[0].cells[1].ch, b'i');
    }

    #[test]
    fn row_ring_scroll_preserves_visible_order() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(5, 2);
        write_visible_bytes(b"aa\nbb\ncc");

        assert_eq!(cell_char_for_tests(0, 0), b'b');
        assert_eq!(cell_char_for_tests(1, 0), b'c');
        let batch = dirty_batch_for_tests(8).expect("scroll dirty");
        assert_eq!(batch.dirty_rows.len(), 2);
    }

    #[test]
    fn ansi_clear_and_cursor_home_work() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"abc\nxyz\x1b[H\x1b[2J");
        for row in 0..3 {
            for col in 0..10 {
                assert_eq!(cell_char_for_tests(row, col), b' ');
            }
        }
        assert_eq!(cursor_for_tests(), Some((0, 0)));
    }

    #[test]
    fn busybox_clear_sequence_emits_single_clear_op() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"abc\nxyz\x1b[H\x1b[J");

        for row in 0..3 {
            for col in 0..10 {
                assert_eq!(cell_char_for_tests(row, col), b' ');
            }
        }
        assert_eq!(cursor_for_tests(), Some((0, 0)));

        let batch = dirty_batch_for_tests(usize::MAX).expect("clear batch");
        assert_eq!(
            batch.clear,
            Some(ClearOp {
                blank: TextCell::default(),
                flush_scrollback: false,
            })
        );
        assert!(batch.dirty_rows.is_empty());
    }

    #[test]
    fn bash_readline_clear_display_flushes_scrollback_and_coalesces() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"abc\nxyz\x1b[H\x1b[J\x1b[3J");

        let batch = dirty_batch_for_tests(usize::MAX).expect("clear-display batch");
        assert_eq!(
            batch.clear,
            Some(ClearOp {
                blank: TextCell::default(),
                flush_scrollback: true,
            })
        );
        assert!(batch.dirty_rows.is_empty());
        assert_eq!(cursor_for_tests(), Some((0, 0)));
    }

    #[test]
    fn linux_csi_3j_clears_visible_display_without_moving_cursor() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"abc\x1b[3J");

        for row in 0..3 {
            for col in 0..10 {
                assert_eq!(cell_char_for_tests(row, col), b' ');
            }
        }
        assert_eq!(cursor_for_tests(), Some((3, 0)));

        let batch = dirty_batch_for_tests(usize::MAX).expect("3J batch");
        assert_eq!(
            batch.clear,
            Some(ClearOp {
                blank: TextCell::default(),
                flush_scrollback: true,
            })
        );
        assert!(batch.dirty_rows.is_empty());
    }

    #[test]
    fn clear_then_text_keeps_one_clear_op_plus_later_dirty_rows() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"abc\x1b[H\x1b[J\x1b[3Jok");

        let batch = dirty_batch_for_tests(usize::MAX).expect("coalesced clear batch");
        assert_eq!(
            batch.clear,
            Some(ClearOp {
                blank: TextCell::default(),
                flush_scrollback: true,
            })
        );
        assert_eq!(batch.dirty_rows.len(), 1);
        assert_eq!(batch.dirty_rows[0].row, 0);
        assert_eq!(batch.dirty_rows[0].cells[0].ch, b'o');
        assert_eq!(batch.dirty_rows[0].cells[1].ch, b'k');
    }

    #[test]
    fn partial_clear_to_end_of_screen_is_not_promoted_to_full_clear() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(5, 2);
        write_visible_bytes(b"abc\nxyz\x1b[2;2H\x1b[J");

        assert_eq!(cell_char_for_tests(0, 0), b'a');
        assert_eq!(cell_char_for_tests(0, 1), b'b');
        assert_eq!(cell_char_for_tests(0, 2), b'c');
        assert_eq!(cell_char_for_tests(1, 0), b'x');
        for col in 1..5 {
            assert_eq!(cell_char_for_tests(1, col), b' ');
        }

        let batch = dirty_batch_for_tests(usize::MAX).expect("partial clear batch");
        assert!(batch.clear.is_none());
        assert!(batch.dirty_rows.iter().any(|row| row.row == 1));
    }

    #[test]
    fn partial_clear_line_keeps_linux_csi_k_behavior() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(5, 2);
        write_visible_bytes(b"abcde\x1b[1;3H\x1b[K");

        assert_eq!(cell_char_for_tests(0, 0), b'a');
        assert_eq!(cell_char_for_tests(0, 1), b'b');
        for col in 2..5 {
            assert_eq!(cell_char_for_tests(0, col), b' ');
        }

        let batch = dirty_batch_for_tests(usize::MAX).expect("line clear batch");
        assert!(batch.clear.is_none());
        assert!(batch.dirty_rows.iter().any(|row| row.row == 0));
    }

    #[test]
    fn nano_replay_places_title_edit_row_and_shortcuts_on_distinct_rows() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(80, 25);
        write_visible_bytes(
            b"\x1b[H\x1b[J\x1b)0\x1b[1;25r\
              \x1b[0;10;7mGNU nano\x1b[m\r\
              \x1b[24d^G Help\r\
              \x1b[25d^X Exit\r\
              \x1b[2dedit",
        );

        for (col, byte) in b"GNU nano".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        for (col, byte) in b"edit".iter().enumerate() {
            assert_eq!(cell_char_for_tests(1, col), *byte);
        }
        for (col, byte) in b"^G Help".iter().enumerate() {
            assert_eq!(cell_char_for_tests(23, col), *byte);
        }
        for (col, byte) in b"^X Exit".iter().enumerate() {
            assert_eq!(cell_char_for_tests(24, col), *byte);
        }
        assert_eq!(cursor_for_tests(), Some((4, 1)));
    }

    #[test]
    fn reverse_sgr_swaps_rendered_cell_colors_until_reset() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(6, 2);
        write_visible_bytes(b"\x1b[31;47mA\x1b[7mB\x1b[27mC");

        let cells = with_console_mut(|console| {
            [
                console.cells[console.cell_index(0, 0)],
                console.cells[console.cell_index(1, 0)],
                console.cells[console.cell_index(2, 0)],
            ]
        });
        assert_eq!((cells[0].fg, cells[0].bg), (0x00aa_0000, 0x00aa_aaaa));
        assert_eq!((cells[1].fg, cells[1].bg), (0x00aa_aaaa, 0x00aa_0000));
        assert_eq!((cells[2].fg, cells[2].bg), (0x00aa_0000, 0x00aa_aaaa));
    }

    #[test]
    fn pending_wrap_is_cancelled_by_carriage_return() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(3, 2);
        write_visible_bytes(b"abc\rX");

        for (col, byte) in b"Xbc".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        assert_eq!(cell_char_for_tests(1, 0), b' ');
        assert_eq!(cursor_for_tests(), Some((1, 0)));
    }

    #[test]
    fn printable_after_right_margin_performs_deferred_wrap() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(3, 2);
        write_visible_bytes(b"abcD");

        for (col, byte) in b"abc".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        assert_eq!(cell_char_for_tests(1, 0), b'D');
        assert_eq!(cursor_for_tests(), Some((1, 1)));
    }

    #[test]
    fn tab_at_final_tab_stop_clamps_without_wrapping_or_filling() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 2);
        write_visible_bytes(b"12345678\tX");

        assert_eq!(cell_char_for_tests(0, 8), b' ');
        assert_eq!(cell_char_for_tests(0, 9), b'X');
        assert_eq!(cursor_for_tests(), Some((9, 0)));
    }

    #[test]
    fn scroll_region_line_feed_and_reverse_index_preserve_outside_rows() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(5, 5);
        write_visible_bytes(
            b"\x1b[1;1HA\x1b[2;1HB\x1b[3;1HC\x1b[4;1HD\x1b[5;1HE\
              \x1b[2;4r\x1b[4;1H\x1bD\x1b[2;1H\x1bM",
        );

        assert_eq!(cell_char_for_tests(0, 0), b'A');
        assert_eq!(cell_char_for_tests(1, 0), b' ');
        assert_eq!(cell_char_for_tests(2, 0), b'C');
        assert_eq!(cell_char_for_tests(3, 0), b'D');
        assert_eq!(cell_char_for_tests(4, 0), b'E');
        assert_eq!(cursor_for_tests(), Some((0, 1)));
    }

    #[test]
    fn insert_delete_lines_and_erase_chars_follow_active_region() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(5, 5);
        write_visible_bytes(
            b"\x1b[1;1HA\x1b[2;1HB\x1b[3;1HC\x1b[4;1HD\x1b[5;1HE\
              \x1b[2;5r\x1b[3;1H\x1b[L\x1b[M",
        );

        for (row, byte) in b"ABCD ".iter().enumerate() {
            assert_eq!(cell_char_for_tests(row, 0), *byte);
        }

        write_visible_bytes(b"\x1b[1;1Habcde\x1b[1;2H\x1b[2X");
        assert_eq!(cell_char_for_tests(0, 0), b'a');
        assert_eq!(cell_char_for_tests(0, 1), b' ');
        assert_eq!(cell_char_for_tests(0, 2), b' ');
        assert_eq!(cell_char_for_tests(0, 3), b'd');
        assert_eq!(cell_char_for_tests(0, 4), b'e');
    }

    #[test]
    fn esc_and_csi_save_restore_cursor_positions() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(8, 4);
        write_visible_bytes(b"A\x1b7\x1b[3;4HB\x1b8C\x1b[s\x1b[4;5HD\x1b[uE");

        assert_eq!(cell_char_for_tests(0, 0), b'A');
        assert_eq!(cell_char_for_tests(0, 1), b'C');
        assert_eq!(cell_char_for_tests(0, 2), b'E');
        assert_eq!(cell_char_for_tests(2, 3), b'B');
        assert_eq!(cell_char_for_tests(3, 4), b'D');
        assert_eq!(cursor_for_tests(), Some((3, 0)));
    }

    #[test]
    fn linux_csi_at_inserts_cells_on_current_line() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(6, 2);
        write_visible_bytes(b"abcde\x1b[1;3H\x1b[@X");

        for (col, byte) in b"abXcde".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        assert_eq!(cursor_for_tests(), Some((3, 0)));
    }

    #[test]
    fn bash_readline_insert_mode_shifts_existing_text() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(6, 2);
        write_visible_bytes(b"abde\x1b[1;3H\x1b[4hc\x1b[4l");

        for (col, byte) in b"abcde ".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        assert_eq!(cursor_for_tests(), Some((3, 0)));

        write_visible_bytes(b"Z");

        for (col, byte) in b"abcZe ".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        assert_eq!(cursor_for_tests(), Some((4, 0)));
    }

    #[test]
    fn sgr_colors_are_kept_in_cells() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"\x1b[32mO\x1b[0mK");
        let batch = dirty_batch_for_tests(1).expect("dirty row");
        assert_eq!(batch.dirty_rows[0].cells[0].fg, 0x0000_aa00);
        assert_eq!(batch.dirty_rows[0].cells[1].fg, DEFAULT_FG);
    }

    #[test]
    fn bash_prompt_sgr_sequences_color_cells_without_artifacts() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(20, 3);
        write_visible_bytes(b"\x1b[01;32m[root@lupos\x1b[00m \x1b[01;34m/\x1b[00m]# ");

        let batch = dirty_batch_for_tests(1).expect("dirty row");
        let cells = &batch.dirty_rows[0].cells;
        let rendered = cells
            .iter()
            .take(16)
            .map(|cell| cell.ch)
            .collect::<Vec<_>>();
        assert_eq!(rendered.as_slice(), b"[root@lupos /]# ");
        assert_eq!(cells[0].fg, 0x0055_ff55);
        assert_eq!(cells[10].fg, 0x0055_ff55);
        assert_eq!(cells[11].fg, DEFAULT_FG);
        assert_eq!(cells[12].fg, 0x0055_55ff);
        assert_eq!(cells[13].fg, DEFAULT_FG);
    }

    #[test]
    fn systemd_terminal_reset_controls_do_not_render_artifacts() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(12, 3);
        write_visible_bytes(b"\x1b[!p\x1b]104\x07\x1b[?7hWelcome");

        for (col, byte) in b"Welcome".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        assert_eq!(cell_char_for_tests(0, 7), b' ');
        assert_eq!(cursor_for_tests(), Some((7, 0)));
    }

    #[test]
    fn terminal_cursor_probe_reset_rehomes_before_banner() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(12, 3);
        write_visible_bytes(
            b"old\nlogs\n\x1b[18t\x1b[6n\x1b[32766;32766H\x1b[6n\x1b[!p\
              \x1b]104\x07\x1b[0m\x1b[?7h\x1b[1G\x1b[0JWelcome",
        );

        for (col, byte) in b"Welcome".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        for col in 7..12 {
            assert_eq!(cell_char_for_tests(0, col), b' ');
        }
        assert_eq!(cursor_for_tests(), Some((7, 0)));
    }

    #[test]
    fn agetty_terminal_probe_does_not_render_dcs_or_leave_cursor_at_probe_edge() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(12, 3);
        write_visible_bytes(
            b"old\nlogs\n\x1b[18t\x1b[6n\x1b[32766;32766H\x1b[6n\
              \x1b[1;1H\x1b[!p\x1b]104\x1b\\\x1b[0m\x1b[?7h\x1b[1G\x1b[0J\
              \x1bP+q6E616D65\x1b\\Welcome",
        );

        for (col, byte) in b"Welcome".iter().enumerate() {
            assert_eq!(cell_char_for_tests(0, col), *byte);
        }
        for col in 7..12 {
            assert_eq!(cell_char_for_tests(0, col), b' ');
        }
        assert_eq!(cursor_for_tests(), Some((7, 0)));
    }

    #[test]
    fn ground_bel_arms_speaker_without_rendering_a_glyph() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        assert!(!bell_armed_for_tests());

        write_visible_bytes(b"a\x07b");

        // BEL must not advance the cursor or paint a cell.
        assert_eq!(cell_char_for_tests(0, 0), b'a');
        assert_eq!(cell_char_for_tests(0, 1), b'b');
        assert_eq!(cursor_for_tests(), Some((2, 0)));
        assert!(bell_armed_for_tests());
    }

    #[test]
    fn bell_tick_silences_speaker_after_duration() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        write_visible_bytes(b"\x07");
        assert!(bell_armed_for_tests());

        // The test `read_tsc` always reports a time past the deadline.
        bell_tick();
        assert!(!bell_armed_for_tests());
    }

    #[test]
    fn osc_terminator_bel_does_not_ring_the_bell() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(12, 3);
        // BEL here is the OSC string terminator, not a ground-state bell.
        write_visible_bytes(b"\x1b]0;title\x07hi");
        assert!(!bell_armed_for_tests());
        assert_eq!(cell_char_for_tests(0, 0), b'h');
        assert_eq!(cell_char_for_tests(0, 1), b'i');
    }

    #[test]
    fn kd_graphics_suppresses_render_batch_until_text_repaint() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        reset_for_tests(10, 3);
        set_fbcon_enabled(false);
        write_visible_bytes(b"x");
        maintenance_budgeted();
        assert!(dirty_batch_for_tests(8).is_some());

        set_fbcon_enabled(true);
        let batch = dirty_batch_for_tests(8).expect("text mode repaint");
        assert_eq!(batch.dirty_rows.len(), 3);
    }

    #[test]
    fn write_bytes_flushes_a_serial_budget_for_interactive_echo() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        crate::linux_driver_abi::tty::serial::clear_capture_for_tests();
        reset_for_tests(10, 3);
        set_fbcon_enabled(false);

        write_bytes(b"systemctl status\n");

        assert_eq!(
            crate::linux_driver_abi::tty::serial::captured_bytes_for_tests(),
            b"systemctl status\r\n"
        );
        assert_eq!(crate::linux_driver_abi::tty::serial::queued_len(), 0);
    }

    #[test]
    fn deferred_write_queues_serial_without_spinning_on_input_echo() {
        let _guard = TEST_CONSOLE_LOCK.lock();
        crate::linux_driver_abi::tty::serial::clear_capture_for_tests();
        reset_for_tests(20, 3);

        write_bytes_deferred(b"echo login-stack\n");

        assert_eq!(cell_char_for_tests(0, 0), b'e');
        assert!(crate::linux_driver_abi::tty::serial::captured_bytes_for_tests().is_empty());
        assert_eq!(
            crate::linux_driver_abi::tty::serial::queued_len(),
            b"echo login-stack\r\n".len()
        );

        flush_serial_budgeted();
        assert_eq!(
            crate::linux_driver_abi::tty::serial::captured_bytes_for_tests(),
            b"echo login-stack\r\n"
        );
        assert_eq!(crate::linux_driver_abi::tty::serial::queued_len(), 0);
    }
}
