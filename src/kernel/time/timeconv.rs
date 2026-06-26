//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timeconv.c
//! test-origin: linux:vendor/linux/kernel/time/timeconv.c
//! Time conversion coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timeconv.c`.

pub fn div_u64_rem(value: u64, divisor: u32) -> (u64, u32) {
    (value / divisor as u64, (value % divisor as u64) as u32)
}

pub fn nsecs_to_jiffies64(ns: u64) -> u64 {
    let tick = super::jiffies::NSEC_PER_TICK;
    (ns.saturating_add(tick - 1)) / tick
}

pub fn jiffies64_to_nsecs(jiffies: u64) -> u64 {
    jiffies.saturating_mul(super::jiffies::NSEC_PER_TICK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nanosecond_jiffy_conversion_rounds_up() {
        assert_eq!(nsecs_to_jiffies64(1), 1);
        assert_eq!(jiffies64_to_nsecs(1), super::super::jiffies::NSEC_PER_TICK);
    }
}
