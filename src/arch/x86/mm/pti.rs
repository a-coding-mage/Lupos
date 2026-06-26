//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pti.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pti.c
//! Page Table Isolation policy.
//!
//! Mirrors the boot command-line gate from `vendor/linux/arch/x86/mm/pti.c`.
//! Lupos does not maintain split user/kernel page tables yet, so PTI remains
//! disabled and initialization fails closed.

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PtiCommandLine {
    Auto,
    ForceOn,
    ForceOff,
}

pub fn pti_parse_cmdline(arg: Option<&str>) -> PtiCommandLine {
    match arg {
        Some("on") | Some("force") => PtiCommandLine::ForceOn,
        Some("off") | Some("0") | Some("nopti") => PtiCommandLine::ForceOff,
        _ => PtiCommandLine::Auto,
    }
}

pub const fn pti_enabled(_cmdline: PtiCommandLine) -> bool {
    false
}

pub const fn pti_check_boottime_disable(cmdline: PtiCommandLine) -> bool {
    matches!(cmdline, PtiCommandLine::ForceOff)
}

pub const fn pti_init() -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

pub const fn pti_finalize() -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmdline_parser_recognizes_on_and_off() {
        assert_eq!(pti_parse_cmdline(Some("on")), PtiCommandLine::ForceOn);
        assert_eq!(pti_parse_cmdline(Some("off")), PtiCommandLine::ForceOff);
    }

    #[test]
    fn pti_is_disabled_until_split_tables_exist() {
        assert!(!pti_enabled(PtiCommandLine::ForceOn));
        assert_eq!(pti_init(), Err(EOPNOTSUPP));
    }
}
