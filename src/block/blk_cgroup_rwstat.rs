//! linux-parity: complete
//! linux-source: vendor/linux/block/blk-cgroup-rwstat.c
//! test-origin: linux:vendor/linux/block/blk-cgroup-rwstat.c
//! Legacy block-cgroup read/write statistic helpers.

extern crate alloc;

use alloc::string::String;
use core::fmt::Write;

pub const BLKG_RWSTAT_NR: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlkgRwstatType {
    Read = 0,
    Write = 1,
    Sync = 2,
    Async = 3,
    Discard = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlkRwOp {
    Read,
    Write,
    Discard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlkOpFlags {
    pub op: BlkRwOp,
    pub sync: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlkgRwstat {
    cpu_cnt: [u64; BLKG_RWSTAT_NR],
    aux_cnt: [u64; BLKG_RWSTAT_NR],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlkgRwstatSample {
    pub cnt: [u64; BLKG_RWSTAT_NR],
}

impl BlkgRwstat {
    pub const fn new() -> Self {
        Self {
            cpu_cnt: [0; BLKG_RWSTAT_NR],
            aux_cnt: [0; BLKG_RWSTAT_NR],
        }
    }

    pub fn add(&mut self, opf: BlkOpFlags, val: u64) {
        let data_idx = match opf.op {
            BlkRwOp::Discard => BlkgRwstatType::Discard,
            BlkRwOp::Write => BlkgRwstatType::Write,
            BlkRwOp::Read => BlkgRwstatType::Read,
        } as usize;
        self.cpu_cnt[data_idx] = self.cpu_cnt[data_idx].saturating_add(val);

        let sync_idx = if opf.sync {
            BlkgRwstatType::Sync
        } else {
            BlkgRwstatType::Async
        } as usize;
        self.cpu_cnt[sync_idx] = self.cpu_cnt[sync_idx].saturating_add(val);
    }

    pub const fn read(&self) -> BlkgRwstatSample {
        BlkgRwstatSample { cnt: self.cpu_cnt }
    }

    pub const fn read_counter(&self, idx: usize) -> u64 {
        self.aux_cnt[idx].saturating_add(self.cpu_cnt[idx])
    }

    pub const fn total(&self) -> u64 {
        self.cpu_cnt[BlkgRwstatType::Read as usize]
            .saturating_add(self.cpu_cnt[BlkgRwstatType::Write as usize])
    }

    pub fn reset(&mut self) {
        self.cpu_cnt = [0; BLKG_RWSTAT_NR];
        self.aux_cnt = [0; BLKG_RWSTAT_NR];
    }

    pub fn add_aux(&mut self, from: &BlkgRwstat) {
        for idx in 0..BLKG_RWSTAT_NR {
            self.aux_cnt[idx] =
                self.aux_cnt[idx].saturating_add(from.cpu_cnt[idx] + from.aux_cnt[idx]);
        }
    }
}

pub fn blkg_rwstat_recursive_sum<'a>(
    stats: impl IntoIterator<Item = &'a BlkgRwstat>,
) -> BlkgRwstatSample {
    let mut sum = BlkgRwstatSample::default();
    for rwstat in stats {
        for idx in 0..BLKG_RWSTAT_NR {
            sum.cnt[idx] = sum.cnt[idx].saturating_add(rwstat.read_counter(idx));
        }
    }
    sum
}

pub fn blkg_rwstat_init(init_result: Result<(), i32>) -> Result<BlkgRwstat, i32> {
    init_result?;
    Ok(BlkgRwstat::new())
}

pub fn blkg_rwstat_exit(rwstat: &mut BlkgRwstat) {
    rwstat.reset();
}

pub fn blkg_prfill_rwstat(device_name: Option<&str>, sample: &BlkgRwstatSample) -> (String, u64) {
    let Some(device_name) = device_name else {
        return (String::new(), 0);
    };
    let labels = ["Read", "Write", "Sync", "Async", "Discard"];
    let mut out = String::new();
    for (idx, label) in labels.iter().enumerate() {
        let _ = writeln!(&mut out, "{device_name} {label} {}", sample.cnt[idx]);
    }
    let total = sample.cnt[BlkgRwstatType::Read as usize]
        + sample.cnt[BlkgRwstatType::Write as usize]
        + sample.cnt[BlkgRwstatType::Discard as usize];
    let _ = writeln!(&mut out, "{device_name} Total {total}");
    (out, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rwstat_add_read_total_and_prfill_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/blk-cgroup-rwstat.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/blk-cgroup-rwstat.h"
        ));
        assert!(
            source.contains("percpu_counter_init_many(rwstat->cpu_cnt, 0, gfp, BLKG_RWSTAT_NR);")
        );
        assert!(source.contains("atomic64_set(&rwstat->aux_cnt[i], 0);"));
        assert!(source.contains("percpu_counter_destroy_many(rwstat->cpu_cnt, BLKG_RWSTAT_NR);"));
        assert!(source.contains("static const char *rwstr[] = {"));
        assert!(source.contains("[BLKG_RWSTAT_READ]\t= \"Read\""));
        assert!(source.contains("if (!dname)"));
        assert!(source.contains("seq_printf(sf, \"%s %s %llu\\n\", dname, rwstr[i],"));
        assert!(source.contains("rwstat->cnt[BLKG_RWSTAT_READ] +"));
        assert!(source.contains("rwstat->cnt[BLKG_RWSTAT_DISCARD]"));
        assert!(source.contains("blkg_rwstat_recursive_sum"));
        assert!(source.contains("lockdep_assert_held(&blkg->q->queue_lock);"));
        assert!(source.contains("memset(sum, 0, sizeof(*sum));"));
        assert!(source.contains("rcu_read_lock();"));
        assert!(source.contains("blkg_for_each_descendant_pre(pos_blkg, pos_css, blkg)"));
        assert!(source.contains("if (!pos_blkg->online)"));
        assert!(source.contains("blkg_to_pd(pos_blkg, pol)"));
        assert!(source.contains("rcu_read_unlock();"));
        assert!(header.contains("enum blkg_rwstat_type"));
        assert!(header.contains("static inline void blkg_rwstat_add"));
        assert!(header.contains("blkg_rwstat_total"));
        assert!(header.contains("blkg_rwstat_add_aux"));

        let mut stat = BlkgRwstat::new();
        stat.add(
            BlkOpFlags {
                op: BlkRwOp::Read,
                sync: true,
            },
            5,
        );
        stat.add(
            BlkOpFlags {
                op: BlkRwOp::Write,
                sync: false,
            },
            7,
        );
        stat.add(
            BlkOpFlags {
                op: BlkRwOp::Discard,
                sync: false,
            },
            3,
        );

        let sample = stat.read();
        assert_eq!(sample.cnt[BlkgRwstatType::Read as usize], 5);
        assert_eq!(sample.cnt[BlkgRwstatType::Write as usize], 7);
        assert_eq!(sample.cnt[BlkgRwstatType::Sync as usize], 5);
        assert_eq!(sample.cnt[BlkgRwstatType::Async as usize], 10);
        assert_eq!(sample.cnt[BlkgRwstatType::Discard as usize], 3);
        assert_eq!(stat.total(), 12);

        let (printed, total) = blkg_prfill_rwstat(Some("vda"), &sample);
        assert_eq!(total, 15);
        assert!(printed.contains("vda Read 5\n"));
        assert!(printed.contains("vda Total 15\n"));
        assert_eq!(blkg_prfill_rwstat(None, &sample), (String::new(), 0));
    }

    #[test]
    fn rwstat_init_and_exit_follow_linux_counter_lifecycle() {
        let mut rwstat = blkg_rwstat_init(Ok(())).expect("rwstat init");
        rwstat.add(
            BlkOpFlags {
                op: BlkRwOp::Read,
                sync: false,
            },
            4,
        );
        assert_eq!(rwstat.total(), 4);
        blkg_rwstat_exit(&mut rwstat);
        assert_eq!(rwstat.read(), BlkgRwstatSample::default());
        assert_eq!(blkg_rwstat_init(Err(-12)), Err(-12));
    }

    #[test]
    fn rwstat_aux_and_recursive_sum_include_dead_child_counts() {
        let mut parent = BlkgRwstat::new();
        let mut child = BlkgRwstat::new();
        child.add(
            BlkOpFlags {
                op: BlkRwOp::Read,
                sync: false,
            },
            11,
        );
        parent.add_aux(&child);
        assert_eq!(parent.read().cnt[BlkgRwstatType::Read as usize], 0);
        assert_eq!(parent.read_counter(BlkgRwstatType::Read as usize), 11);

        let sum = blkg_rwstat_recursive_sum([&parent, &child]);
        assert_eq!(sum.cnt[BlkgRwstatType::Read as usize], 22);
        parent.reset();
        assert_eq!(parent.read_counter(BlkgRwstatType::Read as usize), 0);
    }
}
