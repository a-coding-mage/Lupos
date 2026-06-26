//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_seq.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_seq.c
//! `struct trace_seq` — bounded write buffer used by trace_output formatters.
//!
//! Ref: vendor/linux/kernel/trace/trace_seq.c

extern crate alloc;
use alloc::string::String;

pub struct TraceSeq {
    pub buf: String,
    pub full: bool,
}

impl TraceSeq {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            full: false,
        }
    }

    pub fn puts(&mut self, s: &str) -> bool {
        if self.full {
            return false;
        }
        self.buf.push_str(s);
        true
    }

    pub fn putc(&mut self, c: char) -> bool {
        if self.full {
            return false;
        }
        self.buf.push(c);
        true
    }

    pub fn set_full(&mut self) {
        self.full = true;
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puts_appends_when_not_full() {
        let mut s = TraceSeq::new();
        assert!(s.puts("hello "));
        assert!(s.putc('w'));
        assert_eq!(s.buf, "hello w");
    }

    #[test]
    fn puts_drops_when_full() {
        let mut s = TraceSeq::new();
        s.set_full();
        assert!(!s.puts("ignored"));
        assert_eq!(s.buf, "");
    }
}
