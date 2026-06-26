//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 perf register helpers.
//!
//! Lupos currently implements software perf events in `kernel::events`. Hardware
//! PMU programming, RAPL, BTS/PEBS/Last Branch Record, and uncore devices are
//! x86 optional PMU facilities and return `-EOPNOTSUPP` until the generic perf
//! core grows sampling/ring-buffer support.
//!
//! References:
//! - `vendor/linux/arch/x86/kernel/perf_regs.c`

use crate::include::uapi::errno::EINVAL;
use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86PmuFacility {
    CoreCounters,
    FixedCounters,
    Rapl,
    IntelBts,
    IntelPt,
    IntelUncore,
    AmdIbs,
    AmdLbr,
    AmdUncore,
    Zhaoxin,
    HardwareBreakpoint,
}

pub const fn hardware_pmu_enabled(_facility: X86PmuFacility) -> bool {
    false
}

pub const fn hardware_pmu_errno(_facility: X86PmuFacility) -> i32 {
    EOPNOTSUPP
}

pub const fn perf_reg_ip_index() -> usize {
    PERF_REG_X86_IP
}

pub const PERF_REG_X86_AX: usize = 0;
pub const PERF_REG_X86_BX: usize = 1;
pub const PERF_REG_X86_CX: usize = 2;
pub const PERF_REG_X86_DX: usize = 3;
pub const PERF_REG_X86_SI: usize = 4;
pub const PERF_REG_X86_DI: usize = 5;
pub const PERF_REG_X86_BP: usize = 6;
pub const PERF_REG_X86_SP: usize = 7;
pub const PERF_REG_X86_IP: usize = 8;
pub const PERF_REG_X86_FLAGS: usize = 9;
pub const PERF_REG_X86_CS: usize = 10;
pub const PERF_REG_X86_SS: usize = 11;
pub const PERF_REG_X86_DS: usize = 12;
pub const PERF_REG_X86_ES: usize = 13;
pub const PERF_REG_X86_FS: usize = 14;
pub const PERF_REG_X86_GS: usize = 15;
pub const PERF_REG_X86_R8: usize = 16;
pub const PERF_REG_X86_R9: usize = 17;
pub const PERF_REG_X86_R10: usize = 18;
pub const PERF_REG_X86_R11: usize = 19;
pub const PERF_REG_X86_R12: usize = 20;
pub const PERF_REG_X86_R13: usize = 21;
pub const PERF_REG_X86_R14: usize = 22;
pub const PERF_REG_X86_R15: usize = 23;
pub const PERF_REG_X86_64_MAX: usize = PERF_REG_X86_R15 + 1;
pub const PERF_REG_X86_XMM0: usize = 32;

pub const PERF_REG_X86_64_ABI: u64 = 2;
pub const PERF_REG_X86_32_ABI: u64 = 1;

const PERF_REG_X86_RESERVED_MASK: u64 =
    ((1u64 << PERF_REG_X86_XMM0) - 1) & !((1u64 << PERF_REG_X86_64_MAX) - 1);
const PERF_REG_X86_64_UNSUPPORTED_MASK: u64 = (1u64 << PERF_REG_X86_DS)
    | (1u64 << PERF_REG_X86_ES)
    | (1u64 << PERF_REG_X86_FS)
    | (1u64 << PERF_REG_X86_GS);

pub fn perf_reg_value(regs: &crate::kernel::task::PtRegs, idx: usize) -> Option<u64> {
    match idx {
        PERF_REG_X86_AX => Some(regs.ax),
        PERF_REG_X86_BX => Some(regs.bx),
        PERF_REG_X86_CX => Some(regs.cx),
        PERF_REG_X86_DX => Some(regs.dx),
        PERF_REG_X86_SI => Some(regs.si),
        PERF_REG_X86_DI => Some(regs.di),
        PERF_REG_X86_BP => Some(regs.bp),
        PERF_REG_X86_SP => Some(regs.sp),
        PERF_REG_X86_IP => Some(regs.ip),
        PERF_REG_X86_FLAGS => Some(regs.flags),
        PERF_REG_X86_CS => Some(regs.cs),
        PERF_REG_X86_SS => Some(regs.ss),
        PERF_REG_X86_R8 => Some(regs.r8),
        PERF_REG_X86_R9 => Some(regs.r9),
        PERF_REG_X86_R10 => Some(regs.r10),
        PERF_REG_X86_R11 => Some(regs.r11),
        PERF_REG_X86_R12 => Some(regs.r12),
        PERF_REG_X86_R13 => Some(regs.r13),
        PERF_REG_X86_R14 => Some(regs.r14),
        PERF_REG_X86_R15 => Some(regs.r15),
        PERF_REG_X86_DS | PERF_REG_X86_ES | PERF_REG_X86_FS | PERF_REG_X86_GS => None,
        _ => None,
    }
}

pub const fn perf_reg_validate(mask: u64) -> Result<(), i32> {
    if mask == 0 {
        return Err(EINVAL);
    }
    if mask & (PERF_REG_X86_64_UNSUPPORTED_MASK | PERF_REG_X86_RESERVED_MASK) != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn perf_reg_abi_for_cs(cs: u64) -> u64 {
    if cs == 0x23 {
        PERF_REG_X86_32_ABI
    } else {
        PERF_REG_X86_64_ABI
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regs() -> crate::kernel::task::PtRegs {
        crate::kernel::task::PtRegs {
            r15: 15,
            r14: 14,
            r13: 13,
            r12: 12,
            bp: 6,
            bx: 1,
            r11: 11,
            r10: 10,
            r9: 9,
            r8: 8,
            ax: 0,
            cx: 2,
            dx: 3,
            si: 4,
            di: 5,
            orig_ax: 39,
            ip: 0x401000,
            cs: 0x33,
            flags: 0x202,
            sp: 0x7fff_fff0,
            ss: 0x2b,
        }
    }

    #[test]
    fn hardware_pmu_is_not_registered_yet() {
        assert!(!hardware_pmu_enabled(X86PmuFacility::CoreCounters));
        assert_eq!(hardware_pmu_errno(X86PmuFacility::IntelPt), EOPNOTSUPP);
    }

    #[test]
    fn perf_ip_register_index_is_stable() {
        assert_eq!(perf_reg_ip_index(), PERF_REG_X86_IP);
        assert_eq!(PERF_REG_X86_IP, 8);
        assert_eq!(PERF_REG_X86_R15, 23);
    }

    #[test]
    fn perf_reg_value_uses_linux_x86_indices() {
        let regs = regs();
        assert_eq!(perf_reg_value(&regs, PERF_REG_X86_AX), Some(0));
        assert_eq!(perf_reg_value(&regs, PERF_REG_X86_IP), Some(0x401000));
        assert_eq!(perf_reg_value(&regs, PERF_REG_X86_R12), Some(12));
        assert_eq!(perf_reg_value(&regs, PERF_REG_X86_DS), None);
    }

    #[test]
    fn perf_reg_validate_rejects_x86_64_unsupported_slots() {
        assert_eq!(perf_reg_validate(0), Err(EINVAL));
        assert_eq!(perf_reg_validate(1u64 << PERF_REG_X86_AX), Ok(()));
        assert_eq!(perf_reg_validate(1u64 << PERF_REG_X86_DS), Err(EINVAL));
        assert_eq!(perf_reg_validate(1u64 << 24), Err(EINVAL));
    }

    #[test]
    fn perf_reg_abi_uses_user_code_selector_shape() {
        assert_eq!(perf_reg_abi_for_cs(0x33), PERF_REG_X86_64_ABI);
        assert_eq!(perf_reg_abi_for_cs(0x23), PERF_REG_X86_32_ABI);
    }
}
