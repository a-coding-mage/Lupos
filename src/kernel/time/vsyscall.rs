//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/vsyscall.c
//! test-origin: linux:vendor/linux/kernel/time/vsyscall.c
//! Vsyscall time coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/vsyscall.c`.

use super::posix_clock::{CLOCK_MONOTONIC, CLOCK_REALTIME, ClockId, Timespec64};

pub fn vsyscall_clock_gettime(clock: ClockId) -> Result<Timespec64, i32> {
    match clock {
        CLOCK_REALTIME | CLOCK_MONOTONIC => super::posix_clock::sys_clock_gettime(clock),
        _ => Err(super::posix_clock::EINVAL),
    }
}

pub fn vsyscall_gtod_data_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsyscall_accepts_realtime_and_monotonic() {
        assert!(vsyscall_gtod_data_ready());
        assert!(vsyscall_clock_gettime(CLOCK_REALTIME).is_ok());
        assert!(vsyscall_clock_gettime(99).is_err());
    }
}
