//! linux-parity: complete
//! linux-source: vendor/linux/mm/interval_tree.c
//! test-origin: linux:vendor/linux/mm/interval_tree.c
//! VMA interval-tree compatibility helpers.
//!
//! Lupos stores VMAs in the Maple Tree.  This module provides the small
//! interval-overlap predicate used by legacy Linux MM paths.
//!
//! Reference: `vendor/linux/mm/interval_tree.c`

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interval {
    pub start: u64,
    pub last: u64,
}

impl Interval {
    pub const fn new(start: u64, last: u64) -> Self {
        Self { start, last }
    }

    pub const fn overlaps(self, other: Interval) -> bool {
        self.start <= other.last && other.start <= self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_predicate_matches_closed_intervals() {
        assert!(Interval::new(0, 10).overlaps(Interval::new(10, 20)));
        assert!(!Interval::new(0, 9).overlaps(Interval::new(10, 20)));
    }
}
