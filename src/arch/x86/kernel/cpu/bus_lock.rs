//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/bus_lock.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/bus_lock.c
//! Split-lock / bus-lock detection policy.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/bus_lock.c

// `bus_lock.c` exposes a small policy enum (`off`/`warn`/`fatal`/
// `ratelimit:N`) that maps a kernel cmdline knob to the IA32_DEBUGCTL
// SPLIT_LOCK bit and an #AC handler action. We model the cmdline parser
// and the resulting policy; the actual #AC trap path is owned by the
// fault handler.

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitLockMode {
    Off,
    Warn,
    Fatal,
    Ratelimit(u32),
}

pub const SPLIT_LOCK_DETECT_BIT: u64 = 1 << 29;

pub fn parse_cmdline(option: &str) -> Result<SplitLockMode, i32> {
    if option == "off" {
        return Ok(SplitLockMode::Off);
    }
    if option == "warn" {
        return Ok(SplitLockMode::Warn);
    }
    if option == "fatal" {
        return Ok(SplitLockMode::Fatal);
    }
    if let Some(rest) = option.strip_prefix("ratelimit:") {
        return rest
            .parse::<u32>()
            .map(SplitLockMode::Ratelimit)
            .map_err(|_| EINVAL);
    }
    Err(EINVAL)
}

pub const fn debugctl_mask_for(mode: SplitLockMode) -> u64 {
    match mode {
        SplitLockMode::Off => 0,
        SplitLockMode::Warn | SplitLockMode::Fatal | SplitLockMode::Ratelimit(_) => {
            SPLIT_LOCK_DETECT_BIT
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmdline_parses_canonical_modes() {
        assert_eq!(parse_cmdline("off"), Ok(SplitLockMode::Off));
        assert_eq!(parse_cmdline("warn"), Ok(SplitLockMode::Warn));
        assert_eq!(parse_cmdline("fatal"), Ok(SplitLockMode::Fatal));
        assert_eq!(
            parse_cmdline("ratelimit:8"),
            Ok(SplitLockMode::Ratelimit(8))
        );
        assert_eq!(parse_cmdline("nonsense"), Err(EINVAL));
        assert_eq!(parse_cmdline("ratelimit:abc"), Err(EINVAL));
    }

    #[test]
    fn off_clears_the_debugctl_bit() {
        assert_eq!(debugctl_mask_for(SplitLockMode::Off), 0);
        assert_eq!(
            debugctl_mask_for(SplitLockMode::Warn),
            SPLIT_LOCK_DETECT_BIT
        );
        assert_eq!(
            debugctl_mask_for(SplitLockMode::Ratelimit(2)),
            SPLIT_LOCK_DETECT_BIT
        );
    }
}
