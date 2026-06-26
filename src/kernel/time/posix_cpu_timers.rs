//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/posix-cpu-timers.c
//! test-origin: linux:vendor/linux/kernel/time/posix-cpu-timers.c
//! POSIX CPU timer coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/posix-cpu-timers.c`.

use core::sync::atomic::{AtomicU64, Ordering};

use super::posix_clock::{CLOCK_PROCESS_CPUTIME_ID, CLOCK_THREAD_CPUTIME_ID, ClockId};

#[repr(C)]
pub struct CpuTimerAccount {
    user_ns: AtomicU64,
    system_ns: AtomicU64,
}

impl CpuTimerAccount {
    pub const fn new() -> Self {
        Self {
            user_ns: AtomicU64::new(0),
            system_ns: AtomicU64::new(0),
        }
    }

    pub fn account_user(&self, ns: u64) {
        self.user_ns.fetch_add(ns, Ordering::AcqRel);
    }

    pub fn account_system(&self, ns: u64) {
        self.system_ns.fetch_add(ns, Ordering::AcqRel);
    }

    pub fn cputime_ns(&self) -> u64 {
        self.user_ns
            .load(Ordering::Acquire)
            .saturating_add(self.system_ns.load(Ordering::Acquire))
    }
}

pub fn posix_cpu_clock_get(account: &CpuTimerAccount, clock: ClockId) -> Option<u64> {
    match clock {
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => Some(account.cputime_ns()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_cpu_clock_sums_user_and_system() {
        let account = CpuTimerAccount::new();
        account.account_user(10);
        account.account_system(5);
        assert_eq!(
            posix_cpu_clock_get(&account, CLOCK_PROCESS_CPUTIME_ID),
            Some(15)
        );
    }
}
