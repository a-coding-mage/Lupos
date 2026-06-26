//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/debug.c
//! test-origin: linux:vendor/linux/kernel/sched/debug.c
//! Scheduler debug formatting.
//!
//! Mirrors `vendor/linux/kernel/sched/debug.c`. Full debugfs plumbing belongs
//! to the VFS/debugfs phases; this module provides stable runqueue formatting
//! for tests and later `/proc/sched_debug` output.

use core::fmt::{self, Write};

use super::rq::Rq;

pub struct SliceWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> SliceWriter<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub const fn written(&self) -> usize {
        self.pos
    }
}

impl Write for SliceWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let available = self.buf.len().saturating_sub(self.pos);
        let n = available.min(s.len());
        self.buf[self.pos..self.pos + n].copy_from_slice(&s.as_bytes()[..n]);
        self.pos += n;
        Ok(())
    }
}

pub fn format_rq_debug(rq: &Rq, buf: &mut [u8]) -> usize {
    let mut out = SliceWriter::new(buf);
    let _ = write!(
        out,
        "cpu#{}\n  nr_running: {}\n  clock: {}\n  cfs.nr_running: {}\n  rt.nr_running: {}\n  dl.nr_running: {}\n",
        rq.cpu, rq.nr_running, rq.clock, rq.cfs.nr_running, rq.rt.nr_running, rq.dl.nr_running
    );
    out.written()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rq_debug_includes_cpu_and_running_count() {
        let rq = Rq::new(2);
        let mut buf = [0u8; 160];
        let n = format_rq_debug(&rq, &mut buf);
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(text.contains("cpu#2"));
        assert!(text.contains("nr_running"));
    }
}
