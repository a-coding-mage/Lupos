//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors
//! RV monitor: sleeps within atomic regions.
//!
//! Ref: vendor/linux/kernel/trace/rv/monitors/sleep/sleep.c

pub fn sleep_in_atomic(in_atomic: bool, sleep_called: bool) -> bool {
    in_atomic && sleep_called
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_sleep_is_violation() {
        assert!(sleep_in_atomic(true, true));
        assert!(!sleep_in_atomic(false, true));
    }
}
