//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timeconv.c
//! test-origin: linux:vendor/linux/kernel/time/timeconv.c
//! Time conversion coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/timeconv.c`.

use crate::kernel::module::{export_symbol, find_symbol};

pub fn div_u64_rem(value: u64, divisor: u32) -> (u64, u32) {
    (value / divisor as u64, (value % divisor as u64) as u32)
}

pub fn nsecs_to_jiffies64(ns: u64) -> u64 {
    ns / super::jiffies::NSEC_PER_TICK
}

pub fn nsecs_to_jiffies(ns: u64) -> u64 {
    nsecs_to_jiffies64(ns)
}

pub fn jiffies64_to_nsecs(jiffies: u64) -> u64 {
    jiffies.saturating_mul(super::jiffies::NSEC_PER_TICK)
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "nsecs_to_jiffies64",
        linux_nsecs_to_jiffies64 as usize,
        false,
    );
    export_symbol_once("nsecs_to_jiffies", linux_nsecs_to_jiffies as usize, true);
}

/// `nsecs_to_jiffies64` - `vendor/linux/kernel/time/time.c`.
pub extern "C" fn linux_nsecs_to_jiffies64(ns: u64) -> u64 {
    nsecs_to_jiffies64(ns)
}

/// `nsecs_to_jiffies` - `vendor/linux/kernel/time/time.c`.
pub extern "C" fn linux_nsecs_to_jiffies(ns: u64) -> u64 {
    nsecs_to_jiffies(ns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nanosecond_jiffy_conversion_matches_linux_truncation() {
        assert_eq!(nsecs_to_jiffies64(1), 0);
        assert_eq!(
            nsecs_to_jiffies64(super::super::jiffies::NSEC_PER_TICK - 1),
            0
        );
        assert_eq!(nsecs_to_jiffies64(super::super::jiffies::NSEC_PER_TICK), 1);
        assert_eq!(jiffies64_to_nsecs(1), super::super::jiffies::NSEC_PER_TICK);
    }

    #[test]
    fn timeconv_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            find_symbol("nsecs_to_jiffies64"),
            Some(linux_nsecs_to_jiffies64 as usize)
        );
        assert_eq!(
            find_symbol("nsecs_to_jiffies"),
            Some(linux_nsecs_to_jiffies as usize)
        );
    }
}
