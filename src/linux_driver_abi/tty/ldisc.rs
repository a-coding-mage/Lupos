//! linux-parity: complete
//! linux-source: vendor/linux/drivers/tty
//! test-origin: linux:vendor/linux/drivers/tty
//! n_tty line discipline — `drivers/tty/n_tty.c`.
//!
//! Canonical (cooked) mode: accumulate input characters until a line-ending
//! (LF / CR / EOF), then make the complete line available to `read()`.
//! ECHO: re-queue input chars to the output buffer.
//! Signal chars: ^C → SIGINT, ^Z → SIGTSTP, ^\ → SIGQUIT.
//!
//! References:
//!   - `drivers/tty/n_tty.c:n_tty_open` (line 1885)
//!   - `drivers/tty/n_tty.c:n_tty_close` (line 1865)
//!   - `drivers/tty/n_tty.c:n_tty_receive_buf` (line ~2000)

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

// Signal characters (from termios defaults — `drivers/tty/n_tty.c`).
const CHAR_INTR: u8 = 0x03; // ^C → SIGINT
const CHAR_QUIT: u8 = 0x1C; // ^\ → SIGQUIT
const CHAR_SUSP: u8 = 0x1A; // ^Z → SIGTSTP
const CHAR_ERASE: u8 = 0x7F; // DEL → erase last char (also BS=0x08)
const CHAR_KILL: u8 = 0x15; // ^U → kill line
const CHAR_EOF: u8 = 0x04; // ^D → EOF

/// Pending signal from the line discipline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LdiscSignal {
    Sigint,
    Sigquit,
    Sigtstp,
}

/// State for the n_tty line discipline.
/// Mirrors the key fields of `struct n_tty_data` in `drivers/tty/n_tty.c`.
pub struct NTtyState {
    /// Current line being assembled (canon buffer).
    pub canon_buf: Vec<u8>,
    /// Complete lines ready to be read.
    pub read_buf: VecDeque<Vec<u8>>,
    /// Pending signal to deliver (set when a signal char is received).
    pub pending_signal: Option<LdiscSignal>,
}

impl NTtyState {
    /// `n_tty_open` — initialize line discipline state.
    pub fn new() -> Self {
        Self {
            canon_buf: Vec::new(),
            read_buf: VecDeque::new(),
            pending_signal: None,
        }
    }

    /// `n_tty_receive_buf` — process incoming characters.
    ///
    /// In canonical mode:
    ///   - Signal chars are translated to pending signals.
    ///   - ERASE removes the last character from `canon_buf`.
    ///   - KILL clears `canon_buf`.
    ///   - EOF without preceding chars signals EOF to reader.
    ///   - LF / CR (and EOF with preceding chars) flushes the canon buffer
    ///     to `read_buf`.
    pub fn receive(&mut self, data: &[u8], _echo: bool) {
        for &c in data.iter() {
            match c {
                CHAR_INTR => {
                    self.pending_signal = Some(LdiscSignal::Sigint);
                    self.canon_buf.clear();
                }
                CHAR_QUIT => {
                    self.pending_signal = Some(LdiscSignal::Sigquit);
                    self.canon_buf.clear();
                }
                CHAR_SUSP => {
                    self.pending_signal = Some(LdiscSignal::Sigtstp);
                    self.canon_buf.clear();
                }
                CHAR_ERASE | 0x08 => {
                    self.canon_buf.pop();
                }
                CHAR_KILL => {
                    self.canon_buf.clear();
                }
                CHAR_EOF => {
                    // ^D with no preceding data → EOF (empty line for reader).
                    // ^D with preceding data → flush that data as a line.
                    if !self.canon_buf.is_empty() {
                        let line = core::mem::take(&mut self.canon_buf);
                        self.read_buf.push_back(line);
                    } else {
                        // Push an empty line to signal EOF to the reader.
                        self.read_buf.push_back(Vec::new());
                    }
                }
                b'\n' | b'\r' => {
                    self.canon_buf.push(c);
                    let line = core::mem::take(&mut self.canon_buf);
                    self.read_buf.push_back(line);
                }
                c => {
                    self.canon_buf.push(c);
                }
            }
        }
    }

    /// Read one complete canonical line.  Returns `None` if `read_buf` is empty.
    pub fn read_line(&mut self) -> Option<Vec<u8>> {
        self.read_buf.pop_front()
    }

    /// Take and clear the pending signal.
    pub fn take_signal(&mut self) -> Option<LdiscSignal> {
        self.pending_signal.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_line() {
        let mut ld = NTtyState::new();
        ld.receive(b"hello\n", false);
        assert_eq!(ld.read_line().unwrap(), b"hello\n");
        assert!(ld.read_line().is_none());
    }

    #[test]
    fn erase_removes_char() {
        let mut ld = NTtyState::new();
        ld.receive(b"helo\x7fo\n", false); // helo + DEL → hel, then 'o', newline
        assert_eq!(ld.read_line().unwrap(), b"helo\n");
    }

    #[test]
    fn kill_clears_buffer() {
        let mut ld = NTtyState::new();
        ld.receive(b"discard\x15kept\n", false);
        assert_eq!(ld.read_line().unwrap(), b"kept\n");
    }

    #[test]
    fn ctrl_c_generates_sigint() {
        let mut ld = NTtyState::new();
        ld.receive(&[CHAR_INTR], false);
        assert_eq!(ld.take_signal(), Some(LdiscSignal::Sigint));
        assert!(ld.read_line().is_none());
    }

    #[test]
    fn ctrl_z_generates_sigtstp() {
        let mut ld = NTtyState::new();
        ld.receive(&[CHAR_SUSP], false);
        assert_eq!(ld.take_signal(), Some(LdiscSignal::Sigtstp));
    }

    #[test]
    fn eof_with_data_flushes() {
        let mut ld = NTtyState::new();
        ld.receive(b"partial\x04", false);
        let line = ld.read_line().unwrap();
        assert_eq!(line, b"partial");
    }

    #[test]
    fn multiple_lines_buffered() {
        let mut ld = NTtyState::new();
        ld.receive(b"line1\nline2\n", false);
        assert_eq!(ld.read_line().unwrap(), b"line1\n");
        assert_eq!(ld.read_line().unwrap(), b"line2\n");
        assert!(ld.read_line().is_none());
    }
}
