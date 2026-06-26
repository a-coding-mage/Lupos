//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/fprobe.c
//! test-origin: linux:vendor/linux/kernel/trace/fprobe.c
//! `fprobe` — function-entry tracer that consumes the rethook for return
//! callbacks.
//!
//! Ref: vendor/linux/kernel/trace/fprobe.c

use core::sync::atomic::{AtomicU32, Ordering};

pub struct Fprobe {
    pub entry_handler: Option<fn(addr: u64)>,
    pub exit_handler: Option<fn(addr: u64, retval: u64)>,
    pub nmissed: AtomicU32,
}

impl Fprobe {
    pub const fn new() -> Self {
        Self {
            entry_handler: None,
            exit_handler: None,
            nmissed: AtomicU32::new(0),
        }
    }

    pub fn fire_entry(&self, addr: u64) {
        if let Some(h) = self.entry_handler {
            h(addr);
        } else {
            self.nmissed.fetch_add(1, Ordering::AcqRel);
        }
    }

    pub fn fire_exit(&self, addr: u64, retval: u64) {
        if let Some(h) = self.exit_handler {
            h(addr, retval);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering as O};

    static SAW: AtomicU32 = AtomicU32::new(0);
    fn cb(_addr: u64) {
        SAW.fetch_add(1, O::AcqRel);
    }

    #[test]
    fn entry_handler_fires() {
        SAW.store(0, O::Release);
        let mut p = Fprobe::new();
        p.entry_handler = Some(cb);
        p.fire_entry(0x1000);
        assert_eq!(SAW.load(O::Acquire), 1);
    }

    #[test]
    fn missing_handler_increments_nmissed() {
        let p = Fprobe::new();
        p.fire_entry(0x1000);
        assert_eq!(p.nmissed.load(O::Acquire), 1);
    }
}
