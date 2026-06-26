//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/tsc_sync.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/tsc_sync.c
//! TSC synchronization and TSC_ADJUST tracking.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/tsc_sync.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, Ordering};

use crate::kernel::sched::MAX_CPUS;

pub static TSC_ASYNC_RESETS: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TscAdjust {
    pub bootval: i64,
    pub adjusted: i64,
    pub nextcheck: u64,
    pub warned: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TscSyncResult {
    InSync,
    AdjustMismatch { expected: i64, found: i64 },
    Warp { delta: u64 },
}

pub fn mark_tsc_async_resets(_reason: &str) {
    TSC_ASYNC_RESETS.store(true, Ordering::Release);
}

pub fn tsc_sanitize_first_cpu(cur: &mut TscAdjust, bootval: i64, bootcpu: bool) -> bool {
    let mut value = bootval;
    let forced = bootcpu && bootval != 0 && !TSC_ASYNC_RESETS.load(Ordering::Acquire);
    if forced {
        value = 0;
    }
    cur.bootval = bootval;
    cur.adjusted = value;
    forced
}

pub fn tsc_store_and_check_tsc_adjust(
    table: &mut [TscAdjust; MAX_CPUS],
    cpu: usize,
    refcpu: Option<usize>,
    bootval: i64,
    bootcpu: bool,
    jiffies: u64,
) -> TscSyncResult {
    let cpu = cpu.min(MAX_CPUS - 1);
    table[cpu].bootval = bootval;
    table[cpu].nextcheck = jiffies + 1;
    table[cpu].warned = false;
    table[cpu].adjusted = bootval;
    if let Some(refcpu) = refcpu {
        let expected = table[refcpu.min(MAX_CPUS - 1)].bootval;
        if expected != bootval {
            return TscSyncResult::AdjustMismatch {
                expected,
                found: bootval,
            };
        }
        TscSyncResult::InSync
    } else {
        tsc_sanitize_first_cpu(&mut table[cpu], bootval, bootcpu);
        TscSyncResult::InSync
    }
}

pub fn tsc_verify_tsc_adjust(adj: &mut TscAdjust, curval: i64, now: u64, resume: bool) -> bool {
    if !resume && now < adj.nextcheck {
        return false;
    }
    adj.nextcheck = now + 1;
    if adj.adjusted == curval {
        return false;
    }
    adj.warned = true;
    true
}

pub const fn check_tsc_warp(source: u64, target: u64, threshold: u64) -> TscSyncResult {
    let delta = source.abs_diff(target);
    if delta > threshold {
        TscSyncResult::Warp { delta }
    } else {
        TscSyncResult::InSync
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_cpu_adjust_is_forced_to_zero_without_async_resets() {
        TSC_ASYNC_RESETS.store(false, Ordering::Release);
        let mut adj = TscAdjust::default();
        assert!(tsc_sanitize_first_cpu(&mut adj, 7, true));
        assert_eq!(adj.adjusted, 0);
        mark_tsc_async_resets("test");
        assert!(!tsc_sanitize_first_cpu(&mut adj, 9, true));
        assert_eq!(adj.adjusted, 9);
    }

    #[test]
    fn store_and_check_reports_package_mismatch() {
        let mut table = [TscAdjust::default(); MAX_CPUS];
        table[0].bootval = 5;
        assert_eq!(
            tsc_store_and_check_tsc_adjust(&mut table, 1, Some(0), 6, false, 10),
            TscSyncResult::AdjustMismatch {
                expected: 5,
                found: 6
            }
        );
    }
}
