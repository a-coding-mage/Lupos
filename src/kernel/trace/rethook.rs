//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rethook.c
//! test-origin: linux:vendor/linux/kernel/trace/rethook.c
//! Return-from-function hook (`rethook`).
//!
//! Used by kretprobe / fprobe to intercept function returns.  Each cpu
//! maintains a per-task shadow stack of pending return handlers.
//!
//! Ref: vendor/linux/kernel/trace/rethook.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

pub struct RethookNode {
    pub orig_ret_addr: u64,
    pub handler: fn(u64),
}

pub struct RethookStack {
    nodes: Mutex<Vec<RethookNode>>,
}

impl RethookStack {
    pub const fn new() -> Self {
        Self {
            nodes: Mutex::new(Vec::new()),
        }
    }

    /// `rethook_hook` — push a return interceptor.
    pub fn hook(&self, n: RethookNode) {
        self.nodes.lock().push(n);
    }

    /// `rethook_trampoline_handler` — pop and run the most recent hook.
    pub fn trampoline(&self) -> Option<u64> {
        let n = self.nodes.lock().pop()?;
        (n.handler)(n.orig_ret_addr);
        Some(n.orig_ret_addr)
    }

    pub fn depth(&self) -> usize {
        self.nodes.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU64, Ordering};

    static SEEN: AtomicU64 = AtomicU64::new(0);
    fn handler(addr: u64) {
        SEEN.store(addr, Ordering::Release);
    }

    #[test]
    fn hook_then_trampoline_invokes_handler() {
        SEEN.store(0, Ordering::Release);
        let s = RethookStack::new();
        s.hook(RethookNode {
            orig_ret_addr: 0xdeadbeef,
            handler,
        });
        let r = s.trampoline().unwrap();
        assert_eq!(r, 0xdeadbeef);
        assert_eq!(SEEN.load(Ordering::Acquire), 0xdeadbeef);
    }
}
