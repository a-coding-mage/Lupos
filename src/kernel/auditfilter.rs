//! linux-parity: partial
//! linux-source: vendor/linux/kernel/auditfilter.c
//! test-origin: linux:vendor/linux/kernel/auditfilter.c
//! Audit rule matching — syscall number + pid filter.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use crate::kernel::audit::bump_match_count;

#[derive(Clone, Copy, Debug)]
pub struct AuditRule {
    pub syscall_nr: i32,
    pub pid: i32, // -1 = any
}

static RULES: Mutex<Vec<AuditRule>> = Mutex::new(Vec::new());

/// Linux's `audit_n_rules` fast-path state.
///
/// Writers publish the count while holding `RULES`, after changing the vector
/// and before dropping the mutex. An acquire load of zero can therefore skip
/// the writer mutex without missing a completed rule addition.
static AUDIT_N_RULES: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
static FILTER_LOCK_ACQUISITIONS: AtomicUsize = AtomicUsize::new(0);

pub fn audit_add_rule(rule: AuditRule) {
    let mut rules = RULES.lock();
    rules.push(rule);
    AUDIT_N_RULES.store(rules.len(), Ordering::Release);
}

/// Returns true if any rule matched (and bumps the match counter).
#[inline]
pub fn audit_filter_syscall(nr: i32, pid: i32) -> bool {
    // Linux keeps tasks' audit contexts dummy while `audit_n_rules == 0`.
    // Lupos has no per-task audit context yet, so use the same rule-count
    // predicate directly at the syscall hook.
    if AUDIT_N_RULES.load(Ordering::Acquire) == 0 {
        return false;
    }

    audit_filter_syscall_slow(nr, pid)
}

#[cold]
#[inline(never)]
fn audit_filter_syscall_slow(nr: i32, pid: i32) -> bool {
    #[cfg(test)]
    FILTER_LOCK_ACQUISITIONS.fetch_add(1, Ordering::Relaxed);
    let g = RULES.lock();
    let mut matched = false;
    for r in g.iter() {
        if r.syscall_nr == nr && (r.pid == -1 || r.pid == pid) {
            matched = true;
            bump_match_count();
        }
    }
    matched
}

#[cfg(test)]
pub fn clear_for_test() {
    let mut rules = RULES.lock();
    rules.clear();
    // Linux can let RCU readers finish on a removed entry. Lupos uses the count
    // as the direct reader predicate, so publish zero only after the vector is
    // empty, while still holding the writer mutex.
    AUDIT_N_RULES.store(0, Ordering::Release);
    FILTER_LOCK_ACQUISITIONS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::audit::{match_count, reset_for_test, test_lock};

    #[test]
    fn rule_matches_and_bumps_counter() {
        let _guard = test_lock();
        reset_for_test();
        audit_add_rule(AuditRule {
            syscall_nr: 2,
            pid: 42,
        });
        assert_eq!(match_count(), 0);
        assert!(audit_filter_syscall(2, 42));
        assert_eq!(match_count(), 1);
        // Wrong pid → no match.
        assert!(!audit_filter_syscall(2, 99));
        assert_eq!(match_count(), 1);
    }

    #[test]
    fn pid_minus_one_matches_any() {
        let _guard = test_lock();
        reset_for_test();
        audit_add_rule(AuditRule {
            syscall_nr: 5,
            pid: -1,
        });
        assert!(audit_filter_syscall(5, 1));
        assert!(audit_filter_syscall(5, 2));
        assert!(audit_filter_syscall(5, 1000));
    }

    #[test]
    fn empty_rule_set_skips_filter_mutex() {
        let _guard = test_lock();
        reset_for_test();

        assert!(!audit_filter_syscall(2, 42));
        assert_eq!(FILTER_LOCK_ACQUISITIONS.load(Ordering::Relaxed), 0);

        audit_add_rule(AuditRule {
            syscall_nr: 2,
            pid: 42,
        });
        assert!(audit_filter_syscall(2, 42));
        assert_eq!(FILTER_LOCK_ACQUISITIONS.load(Ordering::Relaxed), 1);

        clear_for_test();
        assert!(!audit_filter_syscall(2, 42));
        assert_eq!(FILTER_LOCK_ACQUISITIONS.load(Ordering::Relaxed), 0);
    }
}
