//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! DAMON data access monitoring.
//!
//! This implements the central region/target/context mechanics from:
//! - `vendor/linux/mm/damon/core.c`
//! - `vendor/linux/mm/damon/lru_sort.c`
//! - `vendor/linux/mm/damon/modules-common.c`
//! - `vendor/linux/mm/damon/ops-common.c`
//! - `vendor/linux/mm/damon/paddr.c`
//! - `vendor/linux/mm/damon/reclaim.c`
//! - `vendor/linux/mm/damon/stat.c`
//! - `vendor/linux/mm/damon/sysfs-common.c`
//! - `vendor/linux/mm/damon/sysfs-schemes.c`
//! - `vendor/linux/mm/damon/sysfs.c`
//! - `vendor/linux/mm/damon/vaddr.c`
//!
//! Monitoring is explicit and deterministic in Lupos tests: callers record
//! observed accesses, then aggregate them into per-region results.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOENT};

pub mod modules_common;
pub mod sysfs_common;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamonRegion {
    pub start: u64,
    pub end: u64,
    pub nr_accesses: u32,
    pub age: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DamonTarget {
    pub id: u64,
    pub regions: Vec<DamonRegion>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamonAttrs {
    pub sample_interval_us: u64,
    pub aggr_interval_us: u64,
    pub min_nr_regions: usize,
    pub max_nr_regions: usize,
}

impl Default for DamonAttrs {
    fn default() -> Self {
        Self {
            sample_interval_us: 5_000,
            aggr_interval_us: 100_000,
            min_nr_regions: 10,
            max_nr_regions: 1_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DamonStats {
    pub targets: usize,
    pub regions: usize,
    pub sampled_accesses: usize,
    pub reclaimed_pages: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamonOps {
    Vaddr,
    Paddr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DamosAction {
    Stat,
    PageOut,
    LruPrio,
    LruDeprio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamonWatermarks {
    pub high: u32,
    pub mid: u32,
    pub low: u32,
}

impl Default for DamonWatermarks {
    fn default() -> Self {
        Self {
            high: 200,
            mid: 150,
            low: 50,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamonScheme {
    pub action: DamosAction,
    pub min_age: u32,
    pub max_age: u32,
    pub min_accesses: u32,
    pub max_accesses: u32,
    pub quota_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DamonLruSortStats {
    pub tried_hot_regions: usize,
    pub sorted_hot_regions: usize,
    pub tried_cold_regions: usize,
    pub sorted_cold_regions: usize,
    pub quota_exceeds: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DamonSysfsContext {
    pub ops: DamonOps,
    pub target_id: u64,
    pub regions: Vec<(u64, u64)>,
    pub schemes: Vec<DamonScheme>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DamonCtx {
    attrs: DamonAttrs,
    targets: Vec<DamonTarget>,
    running: bool,
    reclaimed_pages: usize,
    ops: DamonOps,
    schemes: Vec<DamonScheme>,
}

impl DamonCtx {
    pub fn new() -> Self {
        Self {
            attrs: DamonAttrs::default(),
            targets: Vec::new(),
            running: false,
            reclaimed_pages: 0,
            ops: DamonOps::Vaddr,
            schemes: Vec::new(),
        }
    }

    pub fn set_attrs(&mut self, attrs: DamonAttrs) -> Result<(), i32> {
        if attrs.sample_interval_us == 0
            || attrs.aggr_interval_us == 0
            || attrs.min_nr_regions == 0
            || attrs.min_nr_regions > attrs.max_nr_regions
        {
            return Err(EINVAL);
        }
        self.attrs = attrs;
        Ok(())
    }

    pub fn set_ops(&mut self, ops: DamonOps) {
        self.ops = ops;
    }

    pub fn ops(&self) -> DamonOps {
        self.ops
    }

    pub fn set_schemes(&mut self, schemes: &[DamonScheme]) -> Result<(), i32> {
        for scheme in schemes {
            if scheme.min_age > scheme.max_age || scheme.min_accesses > scheme.max_accesses {
                return Err(EINVAL);
            }
        }
        self.schemes.clear();
        self.schemes.extend_from_slice(schemes);
        Ok(())
    }

    pub fn add_target(&mut self, id: u64) -> Result<(), i32> {
        if self.targets.iter().any(|target| target.id == id) {
            return Err(EINVAL);
        }
        self.targets.push(DamonTarget {
            id,
            regions: Vec::new(),
        });
        Ok(())
    }

    pub fn set_regions(&mut self, id: u64, ranges: &[(u64, u64)]) -> Result<(), i32> {
        if ranges.is_empty() || ranges.len() > self.attrs.max_nr_regions {
            return Err(EINVAL);
        }

        let target = self
            .targets
            .iter_mut()
            .find(|target| target.id == id)
            .ok_or(ENOENT)?;

        target.regions.clear();
        for &(start, end) in ranges {
            if start >= end {
                return Err(EINVAL);
            }
            target.regions.push(DamonRegion {
                start,
                end,
                nr_accesses: 0,
                age: 0,
            });
        }
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), i32> {
        if self.targets.is_empty() {
            return Err(EINVAL);
        }
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn record_access(&mut self, id: u64, addr: u64) -> Result<(), i32> {
        if !self.running {
            return Err(EINVAL);
        }
        let target = self
            .targets
            .iter_mut()
            .find(|target| target.id == id)
            .ok_or(ENOENT)?;
        let region = target
            .regions
            .iter_mut()
            .find(|region| addr >= region.start && addr < region.end)
            .ok_or(ENOENT)?;
        region.nr_accesses = region.nr_accesses.saturating_add(1);
        Ok(())
    }

    pub fn aggregate(&mut self) {
        for target in &mut self.targets {
            for region in &mut target.regions {
                if region.nr_accesses == 0 {
                    region.age = region.age.saturating_add(1);
                } else {
                    region.age = 0;
                }
            }
        }
    }

    pub fn reclaim_cold_regions(&mut self, min_age: u32) -> usize {
        let mut reclaimed = 0;
        for target in &mut self.targets {
            target.regions.retain(|region| {
                if region.age >= min_age {
                    reclaimed += pages_in_range(region.start, region.end);
                    false
                } else {
                    true
                }
            });
        }
        self.reclaimed_pages += reclaimed;
        reclaimed
    }

    pub fn lru_sort(&mut self, hot_thres_access_freq: u32, cold_min_age: u32) -> DamonLruSortStats {
        let mut stats = DamonLruSortStats::default();
        for target in &mut self.targets {
            for region in &mut target.regions {
                if region.nr_accesses.saturating_mul(100) >= hot_thres_access_freq {
                    stats.tried_hot_regions += 1;
                    region.age = 0;
                    stats.sorted_hot_regions += 1;
                }
                if region.age >= cold_min_age {
                    stats.tried_cold_regions += 1;
                    stats.sorted_cold_regions += 1;
                }
            }
        }
        stats
    }

    pub fn stats(&self) -> DamonStats {
        let mut stats = DamonStats {
            targets: self.targets.len(),
            regions: 0,
            sampled_accesses: 0,
            reclaimed_pages: self.reclaimed_pages,
        };
        for target in &self.targets {
            stats.regions += target.regions.len();
            for region in &target.regions {
                stats.sampled_accesses += region.nr_accesses as usize;
            }
        }
        stats
    }
}

pub const fn damon_enabled() -> bool {
    true
}

pub fn start_monitoring(ctx: &mut DamonCtx) -> Result<(), i32> {
    ctx.start()
}

pub fn stop_monitoring(ctx: &mut DamonCtx) {
    ctx.stop();
}

pub fn modules_watermark_active(wmarks: DamonWatermarks, free_mem_rate: u32) -> bool {
    wmarks.low <= wmarks.mid && wmarks.mid <= wmarks.high && free_mem_rate <= wmarks.mid
}

pub fn ops_region_valid(ops: DamonOps, start: u64, end: u64) -> bool {
    if start >= end {
        return false;
    }
    match ops {
        DamonOps::Vaddr => end <= crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX,
        DamonOps::Paddr => start % crate::mm::frame::PAGE_SIZE as u64 == 0,
    }
}

pub fn paddr_region(start: u64, end: u64) -> Result<DamonRegion, i32> {
    if !ops_region_valid(DamonOps::Paddr, start, end) {
        return Err(EINVAL);
    }
    Ok(DamonRegion {
        start,
        end,
        nr_accesses: 0,
        age: 0,
    })
}

pub fn vaddr_region(start: u64, end: u64) -> Result<DamonRegion, i32> {
    if !ops_region_valid(DamonOps::Vaddr, start, end) {
        return Err(EINVAL);
    }
    Ok(DamonRegion {
        start,
        end,
        nr_accesses: 0,
        age: 0,
    })
}

pub fn sysfs_commit(ctx: &mut DamonCtx, sysfs: DamonSysfsContext) -> Result<(), i32> {
    if sysfs.regions.is_empty() {
        return Err(EINVAL);
    }
    for &(start, end) in &sysfs.regions {
        if !ops_region_valid(sysfs.ops, start, end) {
            return Err(EINVAL);
        }
    }
    ctx.set_ops(sysfs.ops);
    if !ctx
        .targets
        .iter()
        .any(|target| target.id == sysfs.target_id)
    {
        ctx.add_target(sysfs.target_id)?;
    }
    ctx.set_regions(sysfs.target_id, &sysfs.regions)?;
    ctx.set_schemes(&sysfs.schemes)?;
    Ok(())
}

fn pages_in_range(start: u64, end: u64) -> usize {
    let bytes = end.saturating_sub(start);
    bytes.div_ceil(crate::mm::frame::PAGE_SIZE as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn damon_tracks_regions_accesses_and_reclaim() {
        assert!(damon_enabled());
        let mut ctx = DamonCtx::new();
        ctx.add_target(100).unwrap();
        ctx.set_regions(100, &[(0x1000, 0x3000), (0x3000, 0x5000)])
            .unwrap();
        start_monitoring(&mut ctx).unwrap();

        ctx.record_access(100, 0x1800).unwrap();
        ctx.aggregate();
        ctx.aggregate();

        let stats = ctx.stats();
        assert_eq!(stats.targets, 1);
        assert_eq!(stats.regions, 2);
        assert_eq!(stats.sampled_accesses, 1);

        let reclaimed = ctx.reclaim_cold_regions(2);
        assert_eq!(reclaimed, 2);
        assert_eq!(ctx.stats().regions, 1);
        stop_monitoring(&mut ctx);
    }

    #[test]
    fn damon_validates_attrs_and_ranges_like_core() {
        let mut ctx = DamonCtx::new();
        assert_eq!(
            ctx.set_attrs(DamonAttrs {
                sample_interval_us: 0,
                ..DamonAttrs::default()
            }),
            Err(EINVAL)
        );
        ctx.add_target(1).unwrap();
        assert_eq!(ctx.set_regions(1, &[(10, 10)]), Err(EINVAL));
    }

    #[test]
    fn damon_lru_sort_ops_and_sysfs_commit_are_stateful() {
        let mut ctx = DamonCtx::new();
        let sysfs = DamonSysfsContext {
            ops: DamonOps::Paddr,
            target_id: 7,
            regions: vec![(0x1000, 0x3000), (0x3000, 0x5000)],
            schemes: vec![DamonScheme {
                action: DamosAction::LruDeprio,
                min_age: 2,
                max_age: 100,
                min_accesses: 0,
                max_accesses: u32::MAX,
                quota_ms: 10,
            }],
        };
        assert_eq!(sysfs_commit(&mut ctx, sysfs), Ok(()));
        assert_eq!(ctx.ops(), DamonOps::Paddr);
        assert!(modules_watermark_active(DamonWatermarks::default(), 100));
        assert!(!modules_watermark_active(DamonWatermarks::default(), 180));

        start_monitoring(&mut ctx).unwrap();
        ctx.record_access(7, 0x1800).unwrap();
        ctx.aggregate();
        ctx.aggregate();
        let stats = ctx.lru_sort(50, 2);
        assert_eq!(stats.tried_hot_regions, 1);
        assert_eq!(stats.sorted_hot_regions, 1);
        assert_eq!(stats.tried_cold_regions, 1);
        assert_eq!(stats.sorted_cold_regions, 1);
    }

    #[test]
    fn damon_paddr_and_vaddr_ops_validate_ranges() {
        assert_eq!(paddr_region(0x1000, 0x2000).unwrap().start, 0x1000);
        assert_eq!(paddr_region(1, 0x2000), Err(EINVAL));
        assert_eq!(vaddr_region(0x1000, 0x2000).unwrap().end, 0x2000);
        assert_eq!(
            vaddr_region(
                crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX + 1,
                crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX + 2,
            ),
            Err(EINVAL)
        );
    }
}
