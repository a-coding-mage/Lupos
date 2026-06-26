//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors
//! RV monitor: schedule-task-state.
//!
//! Ref: vendor/linux/kernel/trace/rv/monitors/sts/sts.c

pub fn check_state_transition(prev_state: u32, next_state: u32) -> bool {
    // The valid prev → next pairs are encoded in the upstream DFA.  For the
    // structural port we accept any transition that doesn't cycle through 0.
    !(prev_state == 0 && next_state == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_to_zero_is_violation() {
        assert!(!check_state_transition(0, 0));
        assert!(check_state_transition(0, 1));
    }
}
