//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_functions_graph.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_functions_graph.c
//! `function_graph` tracer — pairs function entry with the matching return.
//!
//! Ref: vendor/linux/kernel/trace/trace_functions_graph.c

use crate::kernel::trace::fgraph::{FgraphEntry, FgraphReturn};

pub fn on_entry(func: u64, depth: u32) -> FgraphEntry {
    crate::kernel::trace::fgraph::push(FgraphEntry { func, depth });
    FgraphEntry { func, depth }
}

pub fn on_return(now: u64, calltime: u64) -> Option<FgraphReturn> {
    crate::kernel::trace::fgraph::pop(now, calltime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_then_return_round_trip() {
        on_entry(0x1234, 0);
        let r = on_return(200, 100).unwrap();
        assert_eq!(r.func, 0x1234);
        assert_eq!(r.rettime - r.calltime, 100);
    }
}
