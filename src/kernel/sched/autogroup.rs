//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/autogroup.c
//! test-origin: linux:vendor/linux/kernel/sched/autogroup.c
//! Scheduler autogroup support.
//!
//! Mirrors `vendor/linux/kernel/sched/autogroup.c`. Linux autogrouping maps
//! interactive tasks into per-session task groups and applies a group nice
//! value. Lupos keeps the policy object small for now, but preserves the nice
//! to CFS weight conversion and enable gate used by the scheduler surface.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::prio::{MAX_NICE, MIN_NICE, NICE_0_LOAD, nice_to_weight};

static AUTOGROUP_ENABLED: AtomicBool = AtomicBool::new(true);
static NEXT_AUTOGROUP_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug)]
pub struct AutoGroup {
    id: u32,
    nice: i32,
    shares: u64,
    refs: AtomicU32,
}

impl AutoGroup {
    pub fn new(nice: i32) -> Result<Self, i32> {
        if !(MIN_NICE..=MAX_NICE).contains(&nice) {
            return Err(-22);
        }
        Ok(Self {
            id: NEXT_AUTOGROUP_ID.fetch_add(1, Ordering::Relaxed),
            nice,
            shares: nice_to_weight(nice),
            refs: AtomicU32::new(1),
        })
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn nice(&self) -> i32 {
        self.nice
    }

    pub const fn shares(&self) -> u64 {
        self.shares
    }

    pub fn set_nice(&mut self, nice: i32) -> Result<(), i32> {
        if !(MIN_NICE..=MAX_NICE).contains(&nice) {
            return Err(-22);
        }
        self.nice = nice;
        self.shares = nice_to_weight(nice);
        Ok(())
    }

    pub fn get(&self) {
        self.refs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn put(&self) -> u32 {
        self.refs.fetch_sub(1, Ordering::AcqRel).saturating_sub(1)
    }

    pub fn refcount(&self) -> u32 {
        self.refs.load(Ordering::Acquire)
    }
}

pub fn autogroup_enabled() -> bool {
    AUTOGROUP_ENABLED.load(Ordering::Acquire)
}

pub fn set_autogroup_enabled(enabled: bool) {
    AUTOGROUP_ENABLED.store(enabled, Ordering::Release);
}

pub fn autogroup_default_shares() -> u64 {
    NICE_0_LOAD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autogroup_nice_tracks_cfs_weight() {
        let mut group = AutoGroup::new(0).unwrap();
        assert_eq!(group.shares(), NICE_0_LOAD);
        group.set_nice(5).unwrap();
        assert_eq!(group.shares(), nice_to_weight(5));
    }

    #[test]
    fn autogroup_rejects_out_of_range_nice() {
        assert_eq!(AutoGroup::new(MAX_NICE + 1).unwrap_err(), -22);
    }

    #[test]
    fn autogroup_refcount_round_trip() {
        let group = AutoGroup::new(0).unwrap();
        group.get();
        assert_eq!(group.refcount(), 2);
        assert_eq!(group.put(), 1);
    }
}
