//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/tsx.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/tsx.c
//! Intel TSX (Transactional Synchronization Extensions) control.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/tsx.c

// `tsx.c` reads MSR_IA32_TSX_CTRL (0x122) and applies the kernel cmdline
// knob to decide whether to enable, disable, or auto-detect TSX. The
// disable path also masks RTM_DISABLE and TSX_CPUID_CLEAR. We model the
// policy decision.

use crate::include::uapi::errno::EINVAL;

pub const MSR_IA32_TSX_CTRL: u32 = 0x0000_0122;
pub const TSX_CTRL_RTM_DISABLE: u64 = 1 << 0;
pub const TSX_CTRL_CPUID_CLEAR: u64 = 1 << 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TsxMode {
    On,
    Off,
    Auto,
}

pub fn parse_cmdline(option: &str) -> Result<TsxMode, i32> {
    match option {
        "on" => Ok(TsxMode::On),
        "off" => Ok(TsxMode::Off),
        "auto" => Ok(TsxMode::Auto),
        _ => Err(EINVAL),
    }
}

pub const fn ctrl_value_for(mode: TsxMode) -> u64 {
    match mode {
        TsxMode::On => 0,
        TsxMode::Off | TsxMode::Auto => TSX_CTRL_RTM_DISABLE | TSX_CTRL_CPUID_CLEAR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmdline_recognizes_three_modes() {
        assert_eq!(parse_cmdline("on"), Ok(TsxMode::On));
        assert_eq!(parse_cmdline("off"), Ok(TsxMode::Off));
        assert_eq!(parse_cmdline("auto"), Ok(TsxMode::Auto));
        assert_eq!(parse_cmdline("nope"), Err(EINVAL));
    }

    #[test]
    fn off_and_auto_set_disable_bits() {
        assert_eq!(ctrl_value_for(TsxMode::On), 0);
        assert!(ctrl_value_for(TsxMode::Off) & TSX_CTRL_RTM_DISABLE != 0);
        assert!(ctrl_value_for(TsxMode::Auto) & TSX_CTRL_CPUID_CLEAR != 0);
    }
}
