//! linux-parity: partial
//! linux-source: vendor/linux/kernel/auditfilter.c
//! test-origin: linux:vendor/linux/kernel/auditfilter.c
//! Audit rule matching — syscall number + pid filter.

extern crate alloc;

use alloc::vec::Vec;

use spin::Mutex;

use crate::kernel::audit::bump_match_count;

#[derive(Clone, Copy, Debug)]
pub struct AuditRule {
    pub syscall_nr: i32,
    pub pid: i32, // -1 = any
}

static RULES: Mutex<Vec<AuditRule>> = Mutex::new(Vec::new());

pub fn audit_add_rule(rule: AuditRule) {
    RULES.lock().push(rule);
}

/// Returns true if any rule matched (and bumps the match counter).
pub fn audit_filter_syscall(nr: i32, pid: i32) -> bool {
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
    RULES.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::audit::{match_count, reset_for_test};

    #[test]
    fn rule_matches_and_bumps_counter() {
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
        reset_for_test();
        audit_add_rule(AuditRule {
            syscall_nr: 5,
            pid: -1,
        });
        assert!(audit_filter_syscall(5, 1));
        assert!(audit_filter_syscall(5, 2));
        assert!(audit_filter_syscall(5, 1000));
    }
}
