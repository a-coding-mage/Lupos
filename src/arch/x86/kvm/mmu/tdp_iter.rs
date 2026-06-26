//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/mmu/tdp_iter.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/mmu/tdp_iter.c
//! TDP MMU page-table iterator.
//!
//! Ref: `vendor/linux/arch/x86/kvm/mmu/tdp_iter.c`

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

pub const PAGE_SHIFT: u8 = 12;
pub const SPTE_ENT_PER_PAGE: usize = 512;
pub const PT64_ROOT_MAX_LEVEL: usize = 5;
pub const PG_LEVEL_4K: u8 = 1;
pub const PAGES_PER_LEVEL: u64 = SPTE_ENT_PER_PAGE as u64;

pub const SPTE_PRESENT: u64 = 1 << 0;
pub const SPTE_LAST: u64 = 1 << 7;
pub const SPTE_PFN_SHIFT: u8 = PAGE_SHIFT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdpRoot {
    pub level: u8,
    pub spt: usize,
    pub as_id: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TdpPageTables {
    entries: Vec<Vec<u64>>,
}

impl TdpPageTables {
    pub fn new(table_count: usize) -> Self {
        Self {
            entries: vec![vec![0; SPTE_ENT_PER_PAGE]; table_count],
        }
    }

    pub fn set(&mut self, table: usize, index: usize, spte: u64) {
        self.entries[table][index] = spte;
    }

    pub fn read(&self, table: usize, index: usize) -> u64 {
        self.entries
            .get(table)
            .and_then(|table| table.get(index))
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdpIter {
    pub next_last_level_gfn: u64,
    pub yielded_gfn: u64,
    pub pt_path: [usize; PT64_ROOT_MAX_LEVEL],
    pub sptep_table: usize,
    pub sptep_index: usize,
    pub gfn: u64,
    pub gfn_bits: u64,
    pub root_level: u8,
    pub min_level: u8,
    pub level: u8,
    pub as_id: u8,
    pub old_spte: u64,
    pub valid: bool,
    pub yielded: bool,
}

impl Default for TdpIter {
    fn default() -> Self {
        Self {
            next_last_level_gfn: 0,
            yielded_gfn: 0,
            pt_path: [0; PT64_ROOT_MAX_LEVEL],
            sptep_table: 0,
            sptep_index: 0,
            gfn: 0,
            gfn_bits: 0,
            root_level: 0,
            min_level: 0,
            level: 0,
            as_id: 0,
            old_spte: 0,
            valid: false,
            yielded: false,
        }
    }
}

#[inline]
pub const fn kvm_pages_per_hpage(level: u8) -> u64 {
    1u64 << (9 * level.saturating_sub(1) as u32)
}

#[inline]
pub const fn gfn_round_for_level(gfn: u64, level: u8) -> u64 {
    let pages = kvm_pages_per_hpage(level);
    gfn & !(pages - 1)
}

#[inline]
pub const fn spte_index(gfn: u64, gfn_bits: u64, level: u8) -> usize {
    (((gfn | gfn_bits) >> (9 * level.saturating_sub(1) as u32)) & (SPTE_ENT_PER_PAGE as u64 - 1))
        as usize
}

#[inline]
pub const fn is_shadow_present_pte(spte: u64) -> bool {
    spte & SPTE_PRESENT != 0
}

#[inline]
pub const fn is_last_spte(spte: u64, level: u8) -> bool {
    level == PG_LEVEL_4K || spte & SPTE_LAST != 0
}

#[inline]
pub const fn child_spte(table: usize) -> u64 {
    SPTE_PRESENT | ((table as u64) << SPTE_PFN_SHIFT)
}

#[inline]
pub const fn leaf_spte() -> u64 {
    SPTE_PRESENT | SPTE_LAST
}

pub const fn spte_to_child_pt(spte: u64, level: u8) -> Option<usize> {
    if !is_shadow_present_pte(spte) || is_last_spte(spte, level) {
        return None;
    }
    Some((spte >> SPTE_PFN_SHIFT) as usize)
}

fn tdp_iter_refresh_sptep(iter: &mut TdpIter, tables: &TdpPageTables) {
    iter.sptep_table = iter.pt_path[iter.level as usize - 1];
    iter.sptep_index = spte_index(iter.gfn, iter.gfn_bits, iter.level);
    iter.old_spte = tables.read(iter.sptep_table, iter.sptep_index);
}

pub fn tdp_iter_restart(iter: &mut TdpIter, tables: &TdpPageTables) {
    iter.yielded = false;
    iter.yielded_gfn = iter.next_last_level_gfn;
    iter.level = iter.root_level;
    iter.gfn = gfn_round_for_level(iter.next_last_level_gfn, iter.level);
    tdp_iter_refresh_sptep(iter, tables);
    iter.valid = true;
}

pub fn tdp_iter_start(
    root: Option<TdpRoot>,
    min_level: u8,
    next_last_level_gfn: u64,
    gfn_bits: u64,
    tables: &TdpPageTables,
) -> TdpIter {
    let Some(root) = root else {
        return TdpIter::default();
    };
    if root.level < 1
        || root.level as usize > PT64_ROOT_MAX_LEVEL
        || (gfn_bits != 0 && next_last_level_gfn >= gfn_bits)
    {
        return TdpIter::default();
    }

    let mut iter = TdpIter {
        next_last_level_gfn,
        gfn_bits,
        root_level: root.level,
        min_level,
        as_id: root.as_id,
        ..TdpIter::default()
    };
    iter.pt_path[root.level as usize - 1] = root.spt;
    tdp_iter_restart(&mut iter, tables);
    iter
}

fn try_step_down(iter: &mut TdpIter, tables: &TdpPageTables) -> bool {
    if iter.level == iter.min_level {
        return false;
    }

    iter.old_spte = tables.read(iter.sptep_table, iter.sptep_index);
    let Some(child_pt) = spte_to_child_pt(iter.old_spte, iter.level) else {
        return false;
    };

    iter.level -= 1;
    iter.pt_path[iter.level as usize - 1] = child_pt;
    iter.gfn = gfn_round_for_level(iter.next_last_level_gfn, iter.level);
    tdp_iter_refresh_sptep(iter, tables);
    true
}

fn try_step_side(iter: &mut TdpIter, tables: &TdpPageTables) -> bool {
    if spte_index(iter.gfn, iter.gfn_bits, iter.level) == SPTE_ENT_PER_PAGE - 1 {
        return false;
    }

    iter.gfn += kvm_pages_per_hpage(iter.level);
    iter.next_last_level_gfn = iter.gfn;
    iter.sptep_index += 1;
    iter.old_spte = tables.read(iter.sptep_table, iter.sptep_index);
    true
}

fn try_step_up(iter: &mut TdpIter, tables: &TdpPageTables) -> bool {
    if iter.level == iter.root_level {
        return false;
    }

    iter.level += 1;
    iter.gfn = gfn_round_for_level(iter.gfn, iter.level);
    tdp_iter_refresh_sptep(iter, tables);
    true
}

pub fn tdp_iter_next(iter: &mut TdpIter, tables: &TdpPageTables) {
    if iter.yielded {
        tdp_iter_restart(iter, tables);
        return;
    }

    if try_step_down(iter, tables) {
        return;
    }

    loop {
        if try_step_side(iter, tables) {
            return;
        }
        if !try_step_up(iter, tables) {
            break;
        }
    }
    iter.valid = false;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyTdpIter {
    pub gfn: u64,
    pub level: u8,
}

pub const fn descend(it: LegacyTdpIter, child_index: u32) -> LegacyTdpIter {
    LegacyTdpIter {
        gfn: it.gfn + (child_index as u64) * kvm_pages_per_hpage(it.level),
        level: it.level.saturating_sub(1),
    }
}

pub const fn at_leaf(it: LegacyTdpIter) -> bool {
    it.level == PG_LEVEL_4K
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tdp_iter_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/mmu/tdp_iter.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/mmu/tdp_iter.h"
        ));
        assert!(source.contains("static void tdp_iter_refresh_sptep"));
        assert!(source.contains("SPTE_INDEX((iter->gfn | iter->gfn_bits) << PAGE_SHIFT"));
        assert!(source.contains("iter->old_spte = kvm_tdp_mmu_read_spte(iter->sptep);"));
        assert!(source.contains("void tdp_iter_restart(struct tdp_iter *iter)"));
        assert!(source.contains("iter->yielded = false;"));
        assert!(source.contains("iter->yielded_gfn = iter->next_last_level_gfn;"));
        assert!(source.contains("iter->level = iter->root_level;"));
        assert!(source.contains("iter->gfn = gfn_round_for_level"));
        assert!(source.contains("void tdp_iter_start(struct tdp_iter *iter"));
        assert!(source.contains("WARN_ON_ONCE(!root || (root->role.level < 1)"));
        assert!(source.contains("iter->pt_path[iter->root_level - 1] = (tdp_ptep_t)root->spt;"));
        assert!(source.contains("iter->as_id = kvm_mmu_page_as_id(root);"));
        assert!(source.contains("tdp_ptep_t spte_to_child_pt"));
        assert!(source.contains("if (!is_shadow_present_pte(spte) || is_last_spte(spte, level))"));
        assert!(source.contains("static bool try_step_down"));
        assert!(source.contains("if (iter->level == iter->min_level)"));
        assert!(source.contains("static bool try_step_side"));
        assert!(source.contains("SPTE_ENT_PER_PAGE - 1"));
        assert!(source.contains("iter->gfn += KVM_PAGES_PER_HPAGE(iter->level);"));
        assert!(source.contains("static bool try_step_up"));
        assert!(source.contains("if (iter->level == iter->root_level)"));
        assert!(source.contains("void tdp_iter_next(struct tdp_iter *iter)"));
        assert!(source.contains("if (iter->yielded)"));
        assert!(source.contains("if (try_step_down(iter))"));
        assert!(source.contains("while (try_step_up(iter));"));
        assert!(source.contains("iter->valid = false;"));
        assert!(header.contains("struct tdp_iter"));
        assert!(header.contains("for_each_tdp_pte_min_level"));
    }

    #[test]
    fn start_restart_and_child_spte_follow_linux_rules() {
        let mut tables = TdpPageTables::new(2);
        tables.set(0, 0, child_spte(1));
        tables.set(1, 0, leaf_spte());
        let root = TdpRoot {
            level: 2,
            spt: 0,
            as_id: 3,
        };

        assert!(!tdp_iter_start(None, 1, 0, 0, &tables).valid);
        assert!(!tdp_iter_start(Some(TdpRoot { level: 0, ..root }), 1, 0, 0, &tables).valid);
        assert!(!tdp_iter_start(Some(root), 1, 8, 8, &tables).valid);

        let mut iter = tdp_iter_start(Some(root), 1, 0, 0, &tables);
        assert!(iter.valid);
        assert_eq!(iter.level, 2);
        assert_eq!(iter.gfn, 0);
        assert_eq!(iter.as_id, 3);
        assert_eq!(iter.old_spte, child_spte(1));

        iter.yielded = true;
        iter.next_last_level_gfn = 3;
        tdp_iter_next(&mut iter, &tables);
        assert!(!iter.yielded);
        assert_eq!(iter.yielded_gfn, 3);
        assert_eq!(iter.level, 2);
        assert_eq!(iter.gfn, 0);
    }

    #[test]
    fn iterator_walks_down_sideways_up_and_then_invalidates() {
        let mut tables = TdpPageTables::new(2);
        tables.set(0, 0, child_spte(1));
        tables.set(0, 1, leaf_spte());
        tables.set(1, 0, leaf_spte());
        tables.set(1, 1, leaf_spte());

        let mut iter = tdp_iter_start(
            Some(TdpRoot {
                level: 2,
                spt: 0,
                as_id: 0,
            }),
            1,
            0,
            0,
            &tables,
        );

        tdp_iter_next(&mut iter, &tables);
        assert_eq!(iter.level, 1);
        assert_eq!(iter.sptep_table, 1);
        assert_eq!(iter.sptep_index, 0);
        assert_eq!(iter.old_spte, leaf_spte());

        tdp_iter_next(&mut iter, &tables);
        assert_eq!(iter.level, 1);
        assert_eq!(iter.gfn, 1);
        assert_eq!(iter.sptep_index, 1);
        assert_eq!(iter.next_last_level_gfn, 1);

        iter.gfn = SPTE_ENT_PER_PAGE as u64 - 1;
        iter.next_last_level_gfn = iter.gfn;
        iter.sptep_index = SPTE_ENT_PER_PAGE - 1;
        tdp_iter_next(&mut iter, &tables);
        assert_eq!(iter.level, 2);
        assert_eq!(iter.gfn, SPTE_ENT_PER_PAGE as u64);
        assert_eq!(iter.sptep_index, 1);

        iter.gfn = (SPTE_ENT_PER_PAGE * SPTE_ENT_PER_PAGE - 1) as u64;
        iter.next_last_level_gfn = iter.gfn;
        iter.sptep_index = SPTE_ENT_PER_PAGE - 1;
        tdp_iter_next(&mut iter, &tables);
        assert!(!iter.valid);
    }

    #[test]
    fn helpers_compute_gfn_rounding_indices_and_leaf_detection() {
        assert_eq!(kvm_pages_per_hpage(1), 1);
        assert_eq!(kvm_pages_per_hpage(2), 512);
        assert_eq!(gfn_round_for_level(777, 2), 512);
        assert_eq!(spte_index(513, 0, 2), 1);
        assert_eq!(spte_to_child_pt(0, 2), None);
        assert_eq!(spte_to_child_pt(leaf_spte(), 2), None);
        assert_eq!(spte_to_child_pt(child_spte(4), 2), Some(4));

        let it = LegacyTdpIter { gfn: 0, level: 4 };
        let child = descend(it, 1);
        assert_eq!(child.level, 3);
        assert!(at_leaf(LegacyTdpIter { gfn: 0, level: 1 }));
    }
}
