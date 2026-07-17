//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cet.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cet.c
//! Control-flow Enforcement Technology exception policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/cet.c
//!
//! This mirrors Linux's #CP error-code decoding and high-level dispatch.
//! It also owns the per-CPU supervisor CET enable sequence used after the IDT
//! is live and after all Rust code has been compiled with branch protection.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::include::uapi::errno::EINVAL;

pub const CP_EC: u64 = (1 << 15) - 1;
pub const CP_RET: u64 = 1;
pub const CP_IRET: u64 = 2;
pub const CP_ENDBR: u64 = 3;
pub const CP_RSTRORSSP: u64 = 4;
pub const CP_SETSSBSY: u64 = 5;
pub const CP_ENCL: u64 = 1 << 15;

pub const MSR_IA32_PL3_SSP: u32 = 0x0000_06a7;
pub const MSR_IA32_S_CET: u32 = 0x0000_06a2;
pub const CET_ENDBR_EN: u64 = 1 << 2;
pub const X86_CR0_WP: u64 = 1 << 16;
pub const X86_CR4_CET: u64 = 1 << 23;
pub const X86_TRAP_CP: u8 = 21;
pub const SEGV_CPERR: i32 = 10;

pub const X86_FEATURE_USER_SHSTK: u32 = 11 * 32 + 23;
pub const X86_FEATURE_IBT: u32 = 18 * 32 + 20;

static KERNEL_IBT_ACTIVE: [AtomicBool; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicBool::new(false) }; crate::kernel::sched::MAX_CPUS];

// Mirrors arch/x86/kernel/ibt_selftest.S. If hardware enforcement is active,
// the indirect jump raises #CP at `lupos_ibt_selftest_noendbr`; the handler
// clears RAX and resumes at the target RET. Without enforcement it returns 1.
core::arch::global_asm!(
    ".pushsection .text.lupos.ibt_selftest, \"ax\"",
    ".balign 16",
    ".global lupos_ibt_selftest_noendbr",
    ".type lupos_ibt_selftest_noendbr,@function",
    "lupos_ibt_selftest_noendbr:",
    "ret",
    ".size lupos_ibt_selftest_noendbr,.-lupos_ibt_selftest_noendbr",
    ".balign 16",
    ".global lupos_ibt_selftest",
    ".type lupos_ibt_selftest,@function",
    "lupos_ibt_selftest:",
    "endbr64",
    "lea rdx, [rip + lupos_ibt_selftest_noendbr]",
    "mov eax, 1",
    "jmp rdx",
    ".size lupos_ibt_selftest,.-lupos_ibt_selftest",
    ".popsection",
);

unsafe extern "C" {
    fn lupos_ibt_selftest() -> i32;
    fn lupos_ibt_selftest_noendbr();
}

pub fn ibt_selftest_noendbr_addr() -> usize {
    lupos_ibt_selftest_noendbr as usize
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CetEnablePlan {
    pub write_s_cet: bool,
    pub set_cr4_cet: bool,
}

pub const fn cet_enable_plan(hardware_ibt: bool, cr0_wp: bool) -> Result<CetEnablePlan, i32> {
    if !hardware_ibt {
        return Ok(CetEnablePlan {
            write_s_cet: false,
            set_cr4_cet: false,
        });
    }
    if !cr0_wp {
        return Err(EINVAL);
    }
    Ok(CetEnablePlan {
        write_s_cet: true,
        set_cr4_cet: true,
    })
}

/// Linux `setup_cet()` for one physical CPU.
///
/// This runs only after that CPU has a valid #CP IDT entry.  The project-wide
/// rustflags emit ENDBR64 at Rust indirect targets, while assembly targets
/// which are called indirectly carry explicit ENDBR64 instructions.
#[cfg(not(test))]
pub fn setup_cet_cpu(cpu: usize) -> Result<bool, i32> {
    use crate::arch::x86::kernel::cpu::common::{boot_cpu_has, setup_clear_cpu_cap};

    let hardware_ibt = boot_cpu_has(X86_FEATURE_IBT);
    let cr0: u64;
    let mut cr4: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
    }
    let plan = cet_enable_plan(hardware_ibt, cr0 & X86_CR0_WP != 0)?;
    if !plan.write_s_cet {
        KERNEL_IBT_ACTIVE[cpu.min(KERNEL_IBT_ACTIVE.len() - 1)].store(false, Ordering::Release);
        return Ok(false);
    }

    // Linux writes IA32_S_CET before setting CR4.CET.  Until CR4.CET is set,
    // the programmed policy is inert and cannot fault between the two writes.
    unsafe {
        crate::arch::x86::kernel::msr::wrmsr_safe(MSR_IA32_S_CET, CET_ENDBR_EN)?;
        cr4 |= X86_CR4_CET;
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack, preserves_flags));
    }
    let active_cet = unsafe { crate::arch::x86::kernel::msr::rdmsr_safe(MSR_IA32_S_CET)? };
    let active_cr4: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) active_cr4, options(nomem, nostack, preserves_flags));
    }
    if active_cet & CET_ENDBR_EN == 0 || active_cr4 & X86_CR4_CET == 0 {
        unsafe {
            let _ = crate::arch::x86::kernel::msr::wrmsr_safe(MSR_IA32_S_CET, 0);
        }
        setup_clear_cpu_cap(X86_FEATURE_IBT);
        return Err(EINVAL);
    }
    let cpu = cpu.min(KERNEL_IBT_ACTIVE.len() - 1);
    // Publish enforcement before the deliberate fault so the #CP handler can
    // distinguish this kernel-mode check from an unexpected exception.
    KERNEL_IBT_ACTIVE[cpu].store(true, Ordering::Release);
    if unsafe { lupos_ibt_selftest() } != 0 {
        KERNEL_IBT_ACTIVE[cpu].store(false, Ordering::Release);
        unsafe {
            let _ = crate::arch::x86::kernel::msr::wrmsr_safe(MSR_IA32_S_CET, 0);
        }
        setup_clear_cpu_cap(X86_FEATURE_IBT);
        return Err(EINVAL);
    }
    Ok(true)
}

#[cfg(test)]
pub fn setup_cet_cpu(cpu: usize) -> Result<bool, i32> {
    KERNEL_IBT_ACTIVE[cpu.min(KERNEL_IBT_ACTIVE.len() - 1)].store(false, Ordering::Release);
    Ok(false)
}

pub fn kernel_ibt_enabled_on(cpu: usize) -> bool {
    KERNEL_IBT_ACTIVE[cpu.min(KERNEL_IBT_ACTIVE.len() - 1)].load(Ordering::Acquire)
}

pub fn kernel_ibt_enabled() -> bool {
    kernel_ibt_enabled_on(crate::kernel::sched::current_cpu() as usize)
}

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
        assert_eq!(MSR_IA32_S_CET, 0x6a2);
        assert_eq!(CET_ENDBR_EN, 4);
        assert_eq!(X86_CR4_CET, 1 << 23);
        assert_eq!(X86_TRAP_CP, 21);
        assert_eq!(X86_FEATURE_USER_SHSTK, 375);
        assert_eq!(X86_FEATURE_IBT, 596);
    }

    #[test]
    fn cet_enable_requires_write_protect_and_hardware_ibt() {
        assert_eq!(cet_enable_plan(false, false), Ok(CetEnablePlan::default()));
        assert_eq!(cet_enable_plan(true, false), Err(EINVAL));
        assert_eq!(
            cet_enable_plan(true, true),
            Ok(CetEnablePlan {
                write_s_cet: true,
                set_cr4_cet: true,
            })
        );
    }
}
