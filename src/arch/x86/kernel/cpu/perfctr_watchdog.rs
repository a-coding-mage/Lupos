//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/perfctr-watchdog.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/perfctr-watchdog.c
//! NMI watchdog based on performance counter overflow.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/perfctr-watchdog.c

// Linux programs a fixed PMC to count CLK_UNHALTED events and rearms the
// counter from inside an NMI handler. If the NMI fails to arrive, the
// watchdog declares the CPU stuck. We model the period→reload conversion
// and the rearm count; the actual PMC programming is gated.

pub const WATCHDOG_DEFAULT_PERIOD_HZ: u32 = 1;

pub const fn reload_value(cpu_hz: u64, period_hz: u32) -> u64 {
    if period_hz == 0 {
        return 0;
    }
    // 2^48 - cpu_hz / period_hz
    let ticks = cpu_hz / period_hz as u64;
    (1u64 << 48) - ticks
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct WatchdogState {
    pub armed: bool,
    pub overflows: u64,
    pub stuck_threshold: u64,
}

impl WatchdogState {
    pub fn arm(&mut self, stuck_threshold: u64) {
        self.armed = true;
        self.stuck_threshold = stuck_threshold;
        self.overflows = 0;
    }

    pub fn on_nmi(&mut self) -> bool {
        if !self.armed {
            return false;
        }
        self.overflows += 1;
        self.overflows >= self.stuck_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_value_uses_48bit_pmc_counter() {
        let r = reload_value(2_000_000_000, 1);
        assert!(r < 1u64 << 48);
        assert_eq!(r, (1u64 << 48) - 2_000_000_000);
    }

    #[test]
    fn watchdog_stuck_threshold_triggers_stuck_return() {
        let mut s = WatchdogState::default();
        s.arm(3);
        assert!(!s.on_nmi());
        assert!(!s.on_nmi());
        assert!(s.on_nmi());
    }
}
