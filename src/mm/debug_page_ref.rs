//! linux-parity: complete
//! linux-source: vendor/linux/mm/debug_page_ref.c
//! test-origin: linux:vendor/linux/mm/debug_page_ref.c
//! Page reference trace wrapper names.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageRefTrace {
    pub function: &'static str,
    pub tracepoint: &'static str,
    pub exports_symbol: bool,
    pub exports_tracepoint: bool,
}

pub const PAGE_REF_TRACES: &[PageRefTrace] = &[
    PageRefTrace {
        function: "__page_ref_set",
        tracepoint: "page_ref_set",
        exports_symbol: true,
        exports_tracepoint: true,
    },
    PageRefTrace {
        function: "__page_ref_mod",
        tracepoint: "page_ref_mod",
        exports_symbol: true,
        exports_tracepoint: true,
    },
    PageRefTrace {
        function: "__page_ref_mod_and_test",
        tracepoint: "page_ref_mod_and_test",
        exports_symbol: true,
        exports_tracepoint: true,
    },
    PageRefTrace {
        function: "__page_ref_mod_and_return",
        tracepoint: "page_ref_mod_and_return",
        exports_symbol: true,
        exports_tracepoint: true,
    },
    PageRefTrace {
        function: "__page_ref_mod_unless",
        tracepoint: "page_ref_mod_unless",
        exports_symbol: true,
        exports_tracepoint: true,
    },
    PageRefTrace {
        function: "__page_ref_freeze",
        tracepoint: "page_ref_freeze",
        exports_symbol: true,
        exports_tracepoint: true,
    },
    PageRefTrace {
        function: "__page_ref_unfreeze",
        tracepoint: "page_ref_unfreeze",
        exports_symbol: true,
        exports_tracepoint: true,
    },
];

pub fn page_ref_trace(function: &str) -> Option<PageRefTrace> {
    PAGE_REF_TRACES
        .iter()
        .copied()
        .find(|trace| trace.function == function)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_ref_trace_wrappers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/debug_page_ref.c"
        ));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        for trace in PAGE_REF_TRACES {
            assert!(source.contains(trace.function));
            assert!(source.contains(trace.tracepoint));
            assert!(source.contains("EXPORT_TRACEPOINT_SYMBOL"));
        }
        assert_eq!(PAGE_REF_TRACES.len(), 7);
        assert_eq!(
            page_ref_trace("__page_ref_freeze").unwrap().tracepoint,
            "page_ref_freeze"
        );
    }
}
