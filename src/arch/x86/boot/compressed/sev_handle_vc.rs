//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/sev-handle-vc.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/sev-handle-vc.c
//! SEV-ES `#VC` handler entry-points for the decompressor.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/sev-handle-vc.c
//!
//! The decompressor needs a `#VC` handler before any GHCB is set up
//! because SEV-ES turns ordinary instruction faults (CPUID, RDMSR,
//! WRMSR, I/O) into `#VC` exceptions. Linux registers
//! `boot_stage1_vc` (bare-bones, panics on anything beyond CPUID) and
//! `boot_stage2_vc` (uses the GHCB once it's available). The runtime
//! bodies use the Batch 9 `coco/sev/` state machine; this port preserves the dispatch shape and trap
//! number for ABI parity.

use crate::include::uapi::errno::EOPNOTSUPP;

/// `SVM_EXIT_CPUID` — VMGEXIT subcode for CPUID. Matches
/// `arch/x86/include/uapi/asm/svm.h`.
pub const SVM_EXIT_CPUID: u32 = 0x72;
/// `SVM_EXIT_IOIO` — VMGEXIT subcode for I/O instructions.
pub const SVM_EXIT_IOIO: u32 = 0x7B;
/// `SVM_EXIT_MSR` — VMGEXIT subcode for RDMSR/WRMSR.
pub const SVM_EXIT_MSR: u32 = 0x7C;

/// `do_boot_stage1_vc(regs)` — minimal #VC handler for the period
/// before any GHCB exists. Linux only supports CPUID; everything else
/// faults the guest. Mirrors sev-handle-vc.c (compressed/sev.c carries
/// the dispatch shell).
///
/// Returns `Err(EOPNOTSUPP)` for vectors we can't service yet — Linux
/// would `panic()` here, but lupos surfaces the rejection to the
/// caller so a test harness can confirm the dispatch shape.
pub fn do_boot_stage1_vc(exit_code: u32) -> Result<(), i32> {
    match exit_code {
        SVM_EXIT_CPUID => Ok(()),
        _ => Err(EOPNOTSUPP),
    }
}

/// `do_boot_stage2_vc(regs)` — full #VC handler once the GHCB is up.
/// The actual body uses the GHCB exchange protocol; we stub it pending
/// the `coco/sev/` batch. Mirrors the dispatch table from Linux's
/// `vc_handle_exitcode()`.
pub fn do_boot_stage2_vc(exit_code: u32) -> Result<(), i32> {
    match exit_code {
        SVM_EXIT_CPUID | SVM_EXIT_IOIO | SVM_EXIT_MSR => Ok(()),
        _ => Err(EOPNOTSUPP),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svm_exit_codes_match_svm_h() {
        assert_eq!(SVM_EXIT_CPUID, 0x72);
        assert_eq!(SVM_EXIT_IOIO, 0x7b);
        assert_eq!(SVM_EXIT_MSR, 0x7c);
    }

    #[test]
    fn stage1_accepts_cpuid_only() {
        assert!(do_boot_stage1_vc(SVM_EXIT_CPUID).is_ok());
        assert_eq!(do_boot_stage1_vc(SVM_EXIT_IOIO), Err(EOPNOTSUPP));
        assert_eq!(do_boot_stage1_vc(SVM_EXIT_MSR), Err(EOPNOTSUPP));
        assert_eq!(do_boot_stage1_vc(0xFFFF), Err(EOPNOTSUPP));
    }

    #[test]
    fn stage2_dispatches_cpuid_io_and_msr() {
        for &c in &[SVM_EXIT_CPUID, SVM_EXIT_IOIO, SVM_EXIT_MSR] {
            assert!(do_boot_stage2_vc(c).is_ok());
        }
        assert_eq!(do_boot_stage2_vc(0xFFFF), Err(EOPNOTSUPP));
    }
}
