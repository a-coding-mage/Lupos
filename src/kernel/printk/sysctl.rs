//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk/sysctl.c
//! test-origin: linux:vendor/linux/kernel/printk/sysctl.c
//! `/proc/sys/kernel/printk` tunables.
//!
//! Linux exposes a 4-int array: current_loglevel, default_message_loglevel,
//! minimum_console_loglevel, default_console_loglevel.
//!
//! Ref: vendor/linux/kernel/printk/sysctl.c

use core::sync::atomic::{AtomicI32, Ordering};

pub static CONSOLE_LOGLEVEL: AtomicI32 = AtomicI32::new(7);
pub static DEFAULT_MESSAGE_LOGLEVEL: AtomicI32 = AtomicI32::new(4);
pub static MINIMUM_CONSOLE_LOGLEVEL: AtomicI32 = AtomicI32::new(1);
pub static DEFAULT_CONSOLE_LOGLEVEL: AtomicI32 = AtomicI32::new(7);

/// Read the 4-tuple in Linux order.
pub fn read() -> [i32; 4] {
    [
        CONSOLE_LOGLEVEL.load(Ordering::Acquire),
        DEFAULT_MESSAGE_LOGLEVEL.load(Ordering::Acquire),
        MINIMUM_CONSOLE_LOGLEVEL.load(Ordering::Acquire),
        DEFAULT_CONSOLE_LOGLEVEL.load(Ordering::Acquire),
    ]
}

/// `proc_dointvec_minmax`-style writer; rejects values outside [0, 7].
pub fn write(values: [i32; 4]) -> Result<(), i32> {
    for &v in &values {
        if !(0..=7).contains(&v) {
            return Err(-22); // -EINVAL
        }
    }
    CONSOLE_LOGLEVEL.store(values[0], Ordering::Release);
    DEFAULT_MESSAGE_LOGLEVEL.store(values[1], Ordering::Release);
    MINIMUM_CONSOLE_LOGLEVEL.store(values[2], Ordering::Release);
    DEFAULT_CONSOLE_LOGLEVEL.store(values[3], Ordering::Release);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_linux() {
        let v = read();
        assert_eq!(v[0], 7);
        assert_eq!(v[1], 4);
        assert_eq!(v[2], 1);
        assert_eq!(v[3], 7);
    }

    #[test]
    fn write_rejects_out_of_range() {
        assert_eq!(write([8, 4, 1, 7]).unwrap_err(), -22);
        assert_eq!(write([7, 4, 1, -1]).unwrap_err(), -22);
    }

    #[test]
    fn write_then_read_round_trip() {
        write([6, 5, 2, 6]).unwrap();
        assert_eq!(read(), [6, 5, 2, 6]);
        // Restore defaults so other tests don't see drift.
        let _ = write([7, 4, 1, 7]);
    }
}
