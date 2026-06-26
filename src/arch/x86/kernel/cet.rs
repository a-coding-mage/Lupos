//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cet.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cet.c
//! Control-flow Enforcement Technology exception policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cet.c
//!
//! This mirrors Linux's #CP error-code decoding and high-level dispatch.
//! Signal delivery and live IBT selftests remain owned by later trap/signal
//! integration, so this module returns explicit actions.

pub const CP_EC: u64 = (1 << 15) - 1;
pub const CP_RET: u64 = 1;
pub const CP_IRET: u64 = 2;
pub const CP_ENDBR: u64 = 3;
pub const CP_RSTRORSSP: u64 = 4;
pub const CP_SETSSBSY: u64 = 5;
pub const CP_ENCL: u64 = 1 << 15;

pub const MSR_IA32_PL3_SSP: u32 = 0x0000_06a7;
pub const X86_TRAP_CP: u8 = 21;
pub const SEGV_CPERR: i32 = 10;

pub const X86_FEATURE_USER_SHSTK: u32 = 11 * 32 + 23;
pub const X86_FEATURE_IBT: u32 = 18 * 32 + 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlProtectionAction {
    Unexpected,
    ForceSigsegv { code: i32 },
    ClearSelftestAndResume,
    WarnAndResume,
    KernelBug,
}

pub const fn cp_err_string(error_code: u64) -> &'static str {
    match error_code & CP_EC {
        CP_RET => "near ret",
        CP_IRET => "far/iret",
        CP_ENDBR => "endbranch",
        CP_RSTRORSSP => "rstorssp",
        CP_SETSSBSY => "setssbsy",
        _ => "unknown",
    }
}

pub const fn ibt_setup(arg: &str) -> (bool, bool) {
    match arg.as_bytes() {
        b"off" => (false, true),
        b"warn" => (true, false),
        _ => (true, true),
    }
}

pub const fn exc_control_protection_action(
    user_mode: bool,
    user_shstk: bool,
    kernel_ibt: bool,
    ibt_fatal: bool,
    hit_ibt_selftest: bool,
    error_code: u64,
) -> ControlProtectionAction {
    if user_mode {
        return if user_shstk {
            ControlProtectionAction::ForceSigsegv { code: SEGV_CPERR }
        } else {
            ControlProtectionAction::Unexpected
        };
    }

    if !kernel_ibt {
        return ControlProtectionAction::Unexpected;
    }

    if (error_code & CP_EC) != CP_ENDBR {
        return ControlProtectionAction::Unexpected;
    }

    if hit_ibt_selftest {
        return ControlProtectionAction::ClearSelftestAndResume;
    }

    if ibt_fatal {
        ControlProtectionAction::KernelBug
    } else {
        ControlProtectionAction::WarnAndResume
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp_error_strings_match_linux_table() {
        assert_eq!(cp_err_string(0), "unknown");
        assert_eq!(cp_err_string(CP_RET), "near ret");
        assert_eq!(cp_err_string(CP_ENDBR), "endbranch");
        assert_eq!(cp_err_string(99), "unknown");
    }

    #[test]
    fn ibt_setup_parses_off_and_warn() {
        assert_eq!(ibt_setup("off"), (false, true));
        assert_eq!(ibt_setup("warn"), (true, false));
        assert_eq!(ibt_setup("on"), (true, true));
    }

    #[test]
    fn user_cp_fault_requires_user_shadow_stack_feature() {
        assert_eq!(
            exc_control_protection_action(true, true, false, true, false, CP_RET),
            ControlProtectionAction::ForceSigsegv { code: SEGV_CPERR }
        );
        assert_eq!(
            exc_control_protection_action(true, false, false, true, false, CP_RET),
            ControlProtectionAction::Unexpected
        );
    }

    #[test]
    fn kernel_missing_endbr_respects_fatal_policy() {
        assert_eq!(
            exc_control_protection_action(false, false, true, true, false, CP_ENDBR),
            ControlProtectionAction::KernelBug
        );
        assert_eq!(
            exc_control_protection_action(false, false, true, false, false, CP_ENDBR),
            ControlProtectionAction::WarnAndResume
        );
    }

    #[test]
    fn feature_and_msr_constants_match_linux_headers() {
        assert_eq!(MSR_IA32_PL3_SSP, 0x6a7);
        assert_eq!(X86_TRAP_CP, 21);
        assert_eq!(X86_FEATURE_USER_SHSTK, 375);
        assert_eq!(X86_FEATURE_IBT, 596);
    }
}
