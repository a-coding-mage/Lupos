//! linux-parity: complete
//! linux-source: vendor/linux/mm/failslab.c
//! test-origin: linux:vendor/linux/mm/failslab.c
//! Slab allocation fault-injection gates.

use crate::include::uapi::errno::ENOMEM;

pub const GFP_NOFAIL: u32 = 1 << 0;
pub const GFP_DIRECT_RECLAIM: u32 = 1 << 1;
pub const GFP_NOWARN: u32 = 1 << 2;
pub const SLAB_FAILSLAB: u32 = 1 << 3;
pub const FAULT_NOWARN: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailSlabPolicy {
    pub ignore_gfp_reclaim: bool,
    pub cache_filter: bool,
}

pub const DEFAULT_FAILSLAB: FailSlabPolicy = FailSlabPolicy {
    ignore_gfp_reclaim: true,
    cache_filter: false,
};

pub const fn failslab_fault_flags(gfpflags: u32) -> u32 {
    if gfpflags & GFP_NOWARN != 0 {
        FAULT_NOWARN
    } else {
        0
    }
}

pub const fn should_failslab(
    policy: FailSlabPolicy,
    is_bootstrap_cache: bool,
    cache_flags: u32,
    gfpflags: u32,
    fault_engine_would_fail: bool,
) -> i32 {
    if is_bootstrap_cache {
        return 0;
    }
    if gfpflags & GFP_NOFAIL != 0 {
        return 0;
    }
    if policy.ignore_gfp_reclaim && (gfpflags & GFP_DIRECT_RECLAIM != 0) {
        return 0;
    }
    if policy.cache_filter && (cache_flags & SLAB_FAILSLAB == 0) {
        return 0;
    }
    if fault_engine_would_fail { -ENOMEM } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failslab_gates_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/failslab.c"
        ));
        assert!(source.contains("ignore_gfp_reclaim = true"));
        assert!(source.contains("cache_filter = false"));
        assert!(source.contains("if (unlikely(s == kmem_cache))"));
        assert!(source.contains("gfpflags & __GFP_NOFAIL"));
        assert!(source.contains("gfpflags & __GFP_DIRECT_RECLAIM"));
        assert!(source.contains("failslab.cache_filter && !(s->flags & SLAB_FAILSLAB)"));
        assert!(source.contains("gfpflags & __GFP_NOWARN"));
        assert!(source.contains(
            "return should_fail_ex(&failslab.attr, s->object_size, flags) ? -ENOMEM : 0;"
        ));
        assert!(source.contains("ALLOW_ERROR_INJECTION(should_failslab, ERRNO)"));
        assert!(source.contains("__setup(\"failslab=\", setup_failslab);"));

        let policy = DEFAULT_FAILSLAB;
        assert_eq!(should_failslab(policy, true, 0, 0, true), 0);
        assert_eq!(should_failslab(policy, false, 0, GFP_NOFAIL, true), 0);
        assert_eq!(
            should_failslab(policy, false, 0, GFP_DIRECT_RECLAIM, true),
            0
        );
        assert_eq!(should_failslab(policy, false, 0, GFP_NOWARN, true), -ENOMEM);
        assert_eq!(failslab_fault_flags(GFP_NOWARN), FAULT_NOWARN);
        assert_eq!(
            should_failslab(
                FailSlabPolicy {
                    cache_filter: true,
                    ..policy
                },
                false,
                0,
                0,
                true,
            ),
            0
        );
        assert_eq!(
            should_failslab(
                FailSlabPolicy {
                    cache_filter: true,
                    ..policy
                },
                false,
                SLAB_FAILSLAB,
                0,
                true,
            ),
            -ENOMEM
        );
    }
}
