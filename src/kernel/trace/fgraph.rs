//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/fgraph.c
//! test-origin: linux:vendor/linux/kernel/trace/fgraph.c
//! Function-graph tracer infrastructure.
//!
//! Records function entry / exit pairs with a per-cpu stack so callers can
//! reconstruct call trees.
//!
//! Ref: vendor/linux/kernel/trace/fgraph.c

extern crate alloc;
use alloc::vec::Vec;

use spin::Mutex;

#[derive(Clone, Copy, Debug)]
pub struct FgraphEntry {
    pub func: u64,
    pub depth: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct FgraphReturn {
    pub func: u64,
    pub calltime: u64,
    pub rettime: u64,
    pub depth: u32,
}

static STACK: Mutex<Vec<FgraphEntry>> = Mutex::new(Vec::new());

/// `ftrace_push_return_trace`.
pub fn push(entry: FgraphEntry) {
    STACK.lock().push(entry);
}

/// `ftrace_pop_return_trace`.
pub fn pop(now: u64, calltime: u64) -> Option<FgraphReturn> {
    let e = STACK.lock().pop()?;
    Some(FgraphReturn {
        func: e.func,
        calltime,
        rettime: now,
        depth: e.depth,
    })
}

pub fn depth() -> usize {
    STACK.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_round_trip() {
        let d0 = depth();
        push(FgraphEntry {
            func: 0x1000,
            depth: 0,
        });
        push(FgraphEntry {
            func: 0x2000,
            depth: 1,
        });
        assert_eq!(depth(), d0 + 2);
        let r = pop(100, 50).unwrap();
        assert_eq!(r.func, 0x2000);
        assert_eq!(r.rettime, 100);
        let _ = pop(101, 50);
        assert_eq!(depth(), d0);
    }
}
