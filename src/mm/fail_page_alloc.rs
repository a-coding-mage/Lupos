//! linux-parity: complete
//! linux-source: vendor/linux/mm/fail_page_alloc.c
//! test-origin: linux:vendor/linux/mm/fail_page_alloc.c
//! Page allocation fault-injection gates.

pub const GFP_NOFAIL: u32 = 1 << 0;
pub const GFP_HIGHMEM: u32 = 1 << 1;
pub const GFP_DIRECT_RECLAIM: u32 = 1 << 2;
pub const GFP_NOWARN: u32 = 1 << 3;
pub const FAULT_NOWARN: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailPageAlloc {
    pub ignore_gfp_highmem: bool,
    pub ignore_gfp_reclaim: bool,
    pub min_order: u32,
}

pub const DEFAULT_FAIL_PAGE_ALLOC: FailPageAlloc = FailPageAlloc {
    ignore_gfp_highmem: true,
    ignore_gfp_reclaim: true,
    min_order: 1,
};

pub const fn fail_page_alloc_flags(gfp_mask: u32) -> u32 {
    if gfp_mask & GFP_NOWARN != 0 {
        FAULT_NOWARN
    } else {
        0
    }
}

pub const fn should_fail_alloc_page_gate(
    policy: FailPageAlloc,
    gfp_mask: u32,
    order: u32,
    fault_engine_would_fail: bool,
) -> bool {
    if order < policy.min_order {
        return false;
    }
    if gfp_mask & GFP_NOFAIL != 0 {
        return false;
    }
    if policy.ignore_gfp_highmem && (gfp_mask & GFP_HIGHMEM != 0) {
        return false;
    }
    if policy.ignore_gfp_reclaim && (gfp_mask & GFP_DIRECT_RECLAIM != 0) {
        return false;
    }
    fault_engine_would_fail
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_alloc_fault_gates_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/fail_page_alloc.c"
        ));
        assert!(source.contains("ignore_gfp_reclaim = true"));
        assert!(source.contains("ignore_gfp_highmem = true"));
        assert!(source.contains(".min_order = 1"));
        assert!(source.contains("if (order < fail_page_alloc.min_order)"));
        assert!(source.contains("gfp_mask & __GFP_NOFAIL"));
        assert!(source.contains("gfp_mask & __GFP_HIGHMEM"));
        assert!(source.contains("gfp_mask & __GFP_DIRECT_RECLAIM"));
        assert!(source.contains("gfp_mask & __GFP_NOWARN"));
        assert!(source.contains("should_fail_ex(&fail_page_alloc.attr, 1 << order, flags)"));
        assert!(source.contains("ALLOW_ERROR_INJECTION(should_fail_alloc_page, TRUE)"));

        let policy = DEFAULT_FAIL_PAGE_ALLOC;
        assert!(!should_fail_alloc_page_gate(policy, 0, 0, true));
        assert!(!should_fail_alloc_page_gate(policy, GFP_NOFAIL, 1, true));
        assert!(!should_fail_alloc_page_gate(policy, GFP_HIGHMEM, 1, true));
        assert!(!should_fail_alloc_page_gate(
            policy,
            GFP_DIRECT_RECLAIM,
            1,
            true
        ));
        assert!(should_fail_alloc_page_gate(policy, GFP_NOWARN, 1, true));
        assert_eq!(fail_page_alloc_flags(GFP_NOWARN), FAULT_NOWARN);
    }
}
