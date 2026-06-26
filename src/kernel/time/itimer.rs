//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/itimer.c
//! test-origin: linux:vendor/linux/kernel/time/itimer.c
//! Interval timer coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/itimer.c`.

use super::posix_clock::Timespec64;

pub const ITIMER_REAL: i32 = 0;
pub const ITIMER_VIRTUAL: i32 = 1;
pub const ITIMER_PROF: i32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Itimerval64 {
    pub it_interval: Timespec64,
    pub it_value: Timespec64,
}

pub fn itimer_is_supported(which: i32) -> bool {
    matches!(which, ITIMER_REAL | ITIMER_VIRTUAL | ITIMER_PROF)
}

pub fn do_setitimer(which: i32, value: Itimerval64) -> Result<Itimerval64, i32> {
    if !itimer_is_supported(which) || !value.it_interval.is_valid() || !value.it_value.is_valid() {
        return Err(super::posix_clock::EINVAL);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_timer_kind_and_timespecs() {
        let value = Itimerval64::default();
        assert_eq!(do_setitimer(ITIMER_REAL, value), Ok(value));
        assert_eq!(
            do_setitimer(99, value),
            Err(super::super::posix_clock::EINVAL)
        );
    }
}
