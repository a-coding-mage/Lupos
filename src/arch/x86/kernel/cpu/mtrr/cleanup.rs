//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr/cleanup.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/mtrr/cleanup.c
//! Variable-MTRR cleanup / consolidation policy.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/mtrr/cleanup.c

// Linux scans the variable MTRRs at boot, detects holes/conflicts in the
// range coverage, and attempts to coalesce overlapping ranges so the
// total register usage drops below 8. We model the merge predicate.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MtrrRange {
    pub base: u64,
    pub size: u64,
    pub memory_type: u8,
}

pub const fn end(range: MtrrRange) -> u64 {
    range.base.saturating_add(range.size)
}

pub const fn overlaps(a: MtrrRange, b: MtrrRange) -> bool {
    !(end(a) <= b.base || end(b) <= a.base)
}

pub const fn mergeable(a: MtrrRange, b: MtrrRange) -> bool {
    a.memory_type == b.memory_type
        && (end(a) >= b.base && b.base >= a.base || end(b) >= a.base && a.base >= b.base)
}

pub fn merge(a: MtrrRange, b: MtrrRange) -> Option<MtrrRange> {
    if !mergeable(a, b) {
        return None;
    }
    let base = a.base.min(b.base);
    let endpoint = end(a).max(end(b));
    Some(MtrrRange {
        base,
        size: endpoint - base,
        memory_type: a.memory_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_contiguous_ranges_with_same_type() {
        let a = MtrrRange {
            base: 0,
            size: 0x1000,
            memory_type: 6,
        };
        let b = MtrrRange {
            base: 0x1000,
            size: 0x1000,
            memory_type: 6,
        };
        let merged = merge(a, b).unwrap();
        assert_eq!(merged.base, 0);
        assert_eq!(merged.size, 0x2000);
    }

    #[test]
    fn refuses_to_merge_different_memory_types() {
        let a = MtrrRange {
            base: 0,
            size: 0x1000,
            memory_type: 6,
        };
        let b = MtrrRange {
            base: 0x1000,
            size: 0x1000,
            memory_type: 0,
        };
        assert!(merge(a, b).is_none());
    }

    #[test]
    fn overlap_detects_intersecting_ranges() {
        let a = MtrrRange {
            base: 0,
            size: 0x2000,
            memory_type: 6,
        };
        let b = MtrrRange {
            base: 0x1000,
            size: 0x2000,
            memory_type: 6,
        };
        assert!(overlaps(a, b));
    }
}
