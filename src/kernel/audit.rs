//! linux-parity: partial
//! linux-source: vendor/linux/kernel/audit.c
//! test-origin: linux:vendor/linux/kernel/audit.c
//! Linux audit subsystem (M64 — minimal subset).
//!
//! Mirrors `vendor/linux/kernel/audit.c`.  Implements:
//! - `AuditRecord` ring buffer (kernel-only — no netlink yet).
//! - `audit_log()` formatter.
//! - `AuditRule` syscall-number + pid filter.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use spin::Mutex;

pub use crate::kernel::auditfilter::{AuditRule, audit_add_rule, audit_filter_syscall};

pub const AUDIT_RING_SIZE: usize = 256;
pub const AUDIT_STATUS_ENABLED: u32 = 0x0001;
pub const AUDIT_STATUS_FAILURE: u32 = 0x0002;
pub const AUDIT_STATUS_PID: u32 = 0x0004;
pub const AUDIT_STATUS_RATE_LIMIT: u32 = 0x0008;
pub const AUDIT_STATUS_BACKLOG_LIMIT: u32 = 0x0010;
pub const AUDIT_STATUS_BACKLOG_WAIT_TIME: u32 = 0x0020;
pub const AUDIT_STATUS_LOST: u32 = 0x0040;
pub const AUDIT_STATUS_BACKLOG_WAIT_TIME_ACTUAL: u32 = 0x0080;

const AUDIT_FAIL_PRINTK: u32 = 1;
const AUDIT_VERSION_LATEST: u32 = 0x7f;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuditStatus {
    pub mask: u32,
    pub enabled: u32,
    pub failure: u32,
    pub pid: u32,
    pub rate_limit: u32,
    pub backlog_limit: u32,
    pub lost: u32,
    pub backlog: u32,
    pub feature_bitmap: u32,
    pub backlog_wait_time: u32,
    pub backlog_wait_time_actual: u32,
}

#[derive(Clone)]
pub struct AuditRecord {
    pub seq: u64,
    pub text: String,
}

struct AuditRing {
    records: Vec<AuditRecord>,
    seq: u64,
}

impl AuditRing {
    const fn new() -> Self {
        Self {
            records: Vec::new(),
            seq: 0,
        }
    }
}

static AUDIT_RING: Mutex<AuditRing> = Mutex::new(AuditRing::new());
static MATCH_COUNT: AtomicU64 = AtomicU64::new(0);
static AUDIT_ENABLED: AtomicU32 = AtomicU32::new(1);
static AUDIT_FAILURE: AtomicU32 = AtomicU32::new(AUDIT_FAIL_PRINTK);
static AUDIT_DAEMON_PID: AtomicU32 = AtomicU32::new(0);
static AUDIT_RATE_LIMIT: AtomicU32 = AtomicU32::new(0);
static AUDIT_BACKLOG_LIMIT: AtomicU32 = AtomicU32::new(AUDIT_RING_SIZE as u32);
static AUDIT_BACKLOG_WAIT_TIME: AtomicU32 = AtomicU32::new(0);
static AUDIT_LOST: AtomicU32 = AtomicU32::new(0);

#[cfg(test)]
lazy_static::lazy_static! {
    static ref TEST_LOCK: Mutex<()> = Mutex::new(());
}

/// Push a record into the audit ring.
pub fn audit_log(text: &str) {
    let record = {
        let mut g = AUDIT_RING.lock();
        let seq = g.seq;
        g.seq += 1;
        let record = AuditRecord {
            seq,
            text: String::from(text),
        };
        g.records.push(record.clone());
        if g.records.len() > AUDIT_RING_SIZE {
            g.records.remove(0);
            AUDIT_LOST.fetch_add(1, Ordering::AcqRel);
        }
        record
    };
    crate::net::socket::broadcast_audit_record(&record);
}

/// Drain the audit ring into a Vec for tests / future netlink consumer.
pub fn drain() -> Vec<AuditRecord> {
    let mut g = AUDIT_RING.lock();
    let out = g.records.clone();
    g.records.clear();
    out
}

pub fn netlink_snapshot() -> Vec<String> {
    let g = AUDIT_RING.lock();
    g.records
        .iter()
        .map(|r| alloc::format!("audit({}): {}", r.seq, r.text))
        .collect()
}

pub fn record_snapshot() -> Vec<AuditRecord> {
    AUDIT_RING.lock().records.clone()
}

pub fn status() -> AuditStatus {
    let backlog = AUDIT_RING.lock().records.len() as u32;
    AuditStatus {
        mask: AUDIT_STATUS_ENABLED
            | AUDIT_STATUS_FAILURE
            | AUDIT_STATUS_PID
            | AUDIT_STATUS_RATE_LIMIT
            | AUDIT_STATUS_BACKLOG_LIMIT
            | AUDIT_STATUS_LOST
            | AUDIT_STATUS_BACKLOG_WAIT_TIME
            | AUDIT_STATUS_BACKLOG_WAIT_TIME_ACTUAL,
        enabled: AUDIT_ENABLED.load(Ordering::Acquire),
        failure: AUDIT_FAILURE.load(Ordering::Acquire),
        pid: AUDIT_DAEMON_PID.load(Ordering::Acquire),
        rate_limit: AUDIT_RATE_LIMIT.load(Ordering::Acquire),
        backlog_limit: AUDIT_BACKLOG_LIMIT.load(Ordering::Acquire),
        lost: AUDIT_LOST.load(Ordering::Acquire),
        backlog,
        feature_bitmap: AUDIT_VERSION_LATEST,
        backlog_wait_time: AUDIT_BACKLOG_WAIT_TIME.load(Ordering::Acquire),
        backlog_wait_time_actual: 0,
    }
}

pub fn auditd_pid() -> u32 {
    AUDIT_DAEMON_PID.load(Ordering::Acquire)
}

pub fn apply_status(next: AuditStatus) {
    if next.mask & AUDIT_STATUS_ENABLED != 0 {
        AUDIT_ENABLED.store(next.enabled, Ordering::Release);
    }
    if next.mask & AUDIT_STATUS_FAILURE != 0 {
        AUDIT_FAILURE.store(next.failure, Ordering::Release);
    }
    if next.mask & AUDIT_STATUS_PID != 0 {
        AUDIT_DAEMON_PID.store(next.pid, Ordering::Release);
    }
    if next.mask & AUDIT_STATUS_RATE_LIMIT != 0 {
        AUDIT_RATE_LIMIT.store(next.rate_limit, Ordering::Release);
    }
    if next.mask & AUDIT_STATUS_BACKLOG_LIMIT != 0 {
        AUDIT_BACKLOG_LIMIT.store(next.backlog_limit, Ordering::Release);
    }
    if next.mask & AUDIT_STATUS_BACKLOG_WAIT_TIME != 0 {
        AUDIT_BACKLOG_WAIT_TIME.store(next.backlog_wait_time, Ordering::Release);
    }
    if next.mask & AUDIT_STATUS_LOST != 0 && next.lost == 0 {
        AUDIT_LOST.store(0, Ordering::Release);
    }
}

/// Test: does any record's text contain `needle`?
pub fn ring_contains(needle: &str) -> bool {
    let g = AUDIT_RING.lock();
    g.records.iter().any(|r| r.text.contains(needle))
}

pub fn bump_match_count() {
    MATCH_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn match_count() -> u64 {
    MATCH_COUNT.load(Ordering::Acquire)
}

pub fn init() {
    audit_log("audit: initialised");
}

#[cfg(test)]
pub fn reset_for_test() {
    let mut g = AUDIT_RING.lock();
    g.records.clear();
    g.seq = 0;
    MATCH_COUNT.store(0, Ordering::SeqCst);
    AUDIT_ENABLED.store(1, Ordering::SeqCst);
    AUDIT_FAILURE.store(AUDIT_FAIL_PRINTK, Ordering::SeqCst);
    AUDIT_DAEMON_PID.store(0, Ordering::SeqCst);
    AUDIT_RATE_LIMIT.store(0, Ordering::SeqCst);
    AUDIT_BACKLOG_LIMIT.store(AUDIT_RING_SIZE as u32, Ordering::SeqCst);
    AUDIT_BACKLOG_WAIT_TIME.store(0, Ordering::SeqCst);
    AUDIT_LOST.store(0, Ordering::SeqCst);
    crate::kernel::auditfilter::clear_for_test();
}

#[cfg(test)]
pub fn test_lock() -> spin::MutexGuard<'static, ()> {
    TEST_LOCK.lock()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_log_appends_record() {
        let _guard = test_lock();
        reset_for_test();
        audit_log("type=SYSCALL syscall=2 success=yes");
        assert!(ring_contains("syscall=2"));
        let recs = drain();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].seq, 0);
    }

    #[test]
    fn ring_caps_at_size() {
        let _guard = test_lock();
        reset_for_test();
        for i in 0..(AUDIT_RING_SIZE as u64 + 50) {
            audit_log(&alloc::format!("rec={}", i));
        }
        let recs = drain();
        assert!(recs.len() <= AUDIT_RING_SIZE);
    }

    #[test]
    fn netlink_snapshot_formats_records() {
        let _guard = test_lock();
        reset_for_test();
        audit_log("type=USER msg=login");
        let out = netlink_snapshot();
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("type=USER"));
    }

    #[test]
    fn audit_status_tracks_daemon_pid_and_backlog() {
        let _guard = test_lock();
        reset_for_test();
        audit_log("type=DAEMON_START msg=auditd");
        assert_eq!(status().backlog, 1);

        let mut next = status();
        next.mask = AUDIT_STATUS_PID | AUDIT_STATUS_ENABLED;
        next.pid = 4242;
        next.enabled = 1;
        apply_status(next);

        let status = status();
        assert_eq!(status.pid, 4242);
        assert_eq!(auditd_pid(), 4242);
        assert_eq!(status.enabled, 1);
    }
}
