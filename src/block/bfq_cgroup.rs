//! linux-parity: partial
//! linux-source: vendor/linux/block/bfq-cgroup.c
//! test-origin: linux:vendor/linux/block/bfq-cgroup.c
//! BFQ block-cgroup policy data and statistics helpers.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ERANGE};

use super::bfq_iosched::{BFQ_DEFAULT_GRP_CLASS, BFQ_MAX_WEIGHT, BFQ_MIN_WEIGHT};
use super::bfq_wf2q::{BfqEntity, BfqServiceTree, bfq_ioprio_to_weight};
use super::blk_cgroup_rwstat::{BlkOpFlags, BlkgRwstat};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BfqStat {
    cpu_cnt: u64,
    aux_cnt: u64,
}

impl BfqStat {
    pub fn add(&mut self, val: u64) {
        self.cpu_cnt = self.cpu_cnt.saturating_add(val);
    }

    pub fn read(&self) -> u64 {
        self.cpu_cnt
    }

    pub fn reset(&mut self) {
        self.cpu_cnt = 0;
        self.aux_cnt = 0;
    }

    pub fn add_aux(&mut self, from: &Self) {
        self.aux_cnt = self
            .aux_cnt
            .saturating_add(from.cpu_cnt.saturating_add(from.aux_cnt));
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BfqgStats {
    pub bytes: BlkgRwstat,
    pub ios: BlkgRwstat,
    pub merged: BlkgRwstat,
    pub service_time: BlkgRwstat,
    pub wait_time: BlkgRwstat,
    pub queued: BlkgRwstat,
    pub time: BfqStat,
    pub avg_queue_size_sum: BfqStat,
    pub avg_queue_size_samples: BfqStat,
    pub dequeue: BfqStat,
    pub group_wait_time: BfqStat,
    pub idle_time: BfqStat,
    pub empty_time: BfqStat,
    pub flags: u32,
    pub start_group_wait_time: u64,
    pub start_idle_time: u64,
    pub start_empty_time: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BfqgStatsFlag {
    Waiting = 0,
    Idling = 1,
    Empty = 2,
}

impl BfqgStats {
    pub fn mark(&mut self, flag: BfqgStatsFlag) {
        self.flags |= 1 << flag as u8;
    }

    pub fn clear(&mut self, flag: BfqgStatsFlag) {
        self.flags &= !(1 << flag as u8);
    }

    pub fn flagged(&self, flag: BfqgStatsFlag) -> bool {
        self.flags & (1 << flag as u8) != 0
    }

    pub fn set_start_idle_time(&mut self, now_ns: u64) {
        self.start_idle_time = now_ns;
        self.mark(BfqgStatsFlag::Idling);
    }

    pub fn update_idle_time(&mut self, now_ns: u64) {
        if self.flagged(BfqgStatsFlag::Idling) {
            if now_ns > self.start_idle_time {
                self.idle_time.add(now_ns - self.start_idle_time);
            }
            self.clear(BfqgStatsFlag::Idling);
        }
    }

    pub fn set_start_empty_time(&mut self, now_ns: u64) {
        if self.queued.total() == 0 && !self.flagged(BfqgStatsFlag::Empty) {
            self.start_empty_time = now_ns;
            self.mark(BfqgStatsFlag::Empty);
        }
    }

    pub fn end_empty_time(&mut self, now_ns: u64) {
        if self.flagged(BfqgStatsFlag::Empty) {
            if now_ns > self.start_empty_time {
                self.empty_time.add(now_ns - self.start_empty_time);
            }
            self.clear(BfqgStatsFlag::Empty);
        }
    }

    pub fn update_avg_queue_size(&mut self) {
        self.avg_queue_size_sum.add(self.queued.total());
        self.avg_queue_size_samples.add(1);
    }

    pub fn avg_queue_size(&self) -> u64 {
        let samples = self.avg_queue_size_samples.read();
        if samples == 0 {
            0
        } else {
            self.avg_queue_size_sum.read() / samples
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BfqGroup {
    pub entity: BfqEntity,
    pub sched_data: [BfqServiceTree; 3],
    pub stats: BfqgStats,
    pub weight: u16,
    pub dev_weight: u16,
    pub active_entities: u32,
    pub num_queues_with_pending_reqs: u32,
}

impl BfqGroup {
    pub fn new(weight: u16) -> Self {
        let weight = weight.clamp(BFQ_MIN_WEIGHT, BFQ_MAX_WEIGHT);
        Self {
            entity: BfqEntity::new(weight, 0),
            sched_data: [
                BfqServiceTree::new(),
                BfqServiceTree::new(),
                BfqServiceTree::new(),
            ],
            stats: BfqgStats::default(),
            weight,
            dev_weight: 0,
            active_entities: 0,
            num_queues_with_pending_reqs: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BfqGroupData {
    pub weight: u16,
}

impl Default for BfqGroupData {
    fn default() -> Self {
        Self { weight: 100 }
    }
}

pub fn bfq_init_entity(entity: &mut BfqEntity, group: &BfqGroup) {
    entity.weight = entity.new_weight;
    entity.orig_weight = entity.new_weight;
    entity.min_start = group.entity.start;
}

pub fn bfq_group_set_weight(group: &mut BfqGroup, weight: u16, dev_weight: u16) {
    let effective = if dev_weight != 0 { dev_weight } else { weight };
    group.dev_weight = dev_weight;
    if effective != group.entity.new_weight {
        group.entity.new_weight = effective;
        group.entity.prio_changed = true;
    }
}

pub fn bfq_io_set_weight_legacy(group_data: &mut BfqGroupData, val: u64) -> Result<(), i32> {
    if val < BFQ_MIN_WEIGHT as u64 || val > BFQ_MAX_WEIGHT as u64 {
        return Err(-ERANGE);
    }
    group_data.weight = val as u16;
    Ok(())
}

pub fn bfq_io_set_weight(input: &str, group_data: &mut BfqGroupData) -> Result<(), i32> {
    let trimmed = input.trim();
    let value = if let Some(rest) = trimmed.strip_prefix("default ") {
        rest.trim().parse::<u64>().map_err(|_| -EINVAL)?
    } else {
        trimmed.parse::<u64>().map_err(|_| -EINVAL)?
    };
    bfq_io_set_weight_legacy(group_data, value)
}

pub fn bfqg_stats_update_legacy_io(stats: &mut BfqgStats, opf: BlkOpFlags, bytes: u64) {
    stats.bytes.add(opf, bytes);
    stats.ios.add(opf, 1);
}

pub fn bfqg_stats_update_io_add(stats: &mut BfqgStats, opf: BlkOpFlags, now_ns: u64) {
    stats.queued.add(opf, 1);
    stats.end_empty_time(now_ns);
}

pub fn bfqg_stats_update_io_remove(stats: &mut BfqgStats, opf: BlkOpFlags) {
    stats.queued.add(opf, u64::MAX);
}

pub fn bfqg_stats_update_io_merged(stats: &mut BfqgStats, opf: BlkOpFlags) {
    stats.merged.add(opf, 1);
}

pub fn bfqg_stats_update_completion(
    stats: &mut BfqgStats,
    start_time_ns: u64,
    io_start_time_ns: u64,
    opf: BlkOpFlags,
    now_ns: u64,
) {
    if now_ns > io_start_time_ns {
        stats.service_time.add(opf, now_ns - io_start_time_ns);
    }
    if io_start_time_ns > start_time_ns {
        stats.wait_time.add(opf, io_start_time_ns - start_time_ns);
    }
}

pub fn bfq_create_group_hierarchy(weights: &[u16]) -> Vec<BfqGroup> {
    if weights.is_empty() {
        return alloc::vec![BfqGroup::new(bfq_ioprio_to_weight(0))];
    }
    weights.iter().copied().map(BfqGroup::new).collect()
}

#[cfg(test)]
mod tests {
    use super::super::blk_cgroup_rwstat::{BlkRwOp, BlkgRwstatType};
    use super::*;

    #[test]
    fn cgroup_weight_paths_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bfq-cgroup.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bfq-iosched.h"
        ));
        assert!(source.contains("bfq_io_set_weight_legacy"));
        assert!(source.contains("if (val < BFQ_MIN_WEIGHT || val > BFQ_MAX_WEIGHT)"));
        assert!(source.contains("bfq_group_set_weight(struct bfq_group *bfqg"));
        assert!(source.contains(".name = \"bfq.weight\""));
        assert!(header.contains("#define BFQ_MIN_WEIGHT"));

        let mut data = BfqGroupData::default();
        assert_eq!(bfq_io_set_weight_legacy(&mut data, 0), Err(-ERANGE));
        assert_eq!(bfq_io_set_weight("default 250", &mut data), Ok(()));
        assert_eq!(data.weight, 250);

        let mut group = BfqGroup::new(100);
        bfq_group_set_weight(&mut group, 200, 0);
        assert_eq!(group.entity.new_weight, 200);
        assert!(group.entity.prio_changed);
        bfq_group_set_weight(&mut group, 200, 300);
        assert_eq!(group.dev_weight, 300);
        assert_eq!(group.entity.new_weight, 300);
    }

    #[test]
    fn bfqg_stats_account_rwstat_and_timing() {
        let opf = BlkOpFlags {
            op: BlkRwOp::Write,
            sync: false,
        };
        let mut stats = BfqgStats::default();
        bfqg_stats_update_legacy_io(&mut stats, opf, 4096);
        assert_eq!(stats.bytes.read().cnt[BlkgRwstatType::Write as usize], 4096);
        assert_eq!(stats.ios.read().cnt[BlkgRwstatType::Write as usize], 1);

        stats.set_start_empty_time(10);
        bfqg_stats_update_io_add(&mut stats, opf, 25);
        assert_eq!(stats.empty_time.read(), 15);
        stats.update_avg_queue_size();
        assert_eq!(stats.avg_queue_size(), 1);
        bfqg_stats_update_completion(&mut stats, 1, 4, opf, 10);
        assert_eq!(
            stats.service_time.read().cnt[BlkgRwstatType::Write as usize],
            6
        );
        assert_eq!(
            stats.wait_time.read().cnt[BlkgRwstatType::Write as usize],
            3
        );
    }
}
