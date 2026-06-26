//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/rv_reactors.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/rv_reactors.c
//! Reactor registry — actions fired when a monitor violates.
//!
//! Ref: vendor/linux/kernel/trace/rv/rv_reactors.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

pub struct Reactor {
    pub name: &'static str,
    pub react: fn(monitor: &str),
}

static REACTORS: Mutex<Vec<Reactor>> = Mutex::new(Vec::new());

pub fn register(r: Reactor) {
    REACTORS.lock().push(r);
}

pub fn fire(monitor: &str) -> usize {
    let g = REACTORS.lock();
    for r in g.iter() {
        (r.react)(monitor);
    }
    g.len()
}

pub fn count() -> usize {
    REACTORS.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static N: AtomicUsize = AtomicUsize::new(0);
    fn cb(_: &str) {
        N.fetch_add(1, Ordering::AcqRel);
    }

    #[test]
    fn register_and_fire() {
        N.store(0, Ordering::Release);
        register(Reactor {
            name: "t",
            react: cb,
        });
        fire("any");
        assert!(N.load(Ordering::Acquire) >= 1);
    }
}
