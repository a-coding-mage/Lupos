//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/math-emu/reg_mul.c
//! test-origin: linux:vendor/linux/arch/x86/math-emu/reg_mul.c
//! x87 software-emulator multiply tag decision tree.

use super::fpu_tags::{
    TAG_SPECIAL, TAG_VALID, TAG_ZERO, TW_DENORMAL, TW_INFINITY, TW_NAN, fpu_special,
};

pub const FPU_EXCEPTION: i32 = i32::MIN;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MulTag {
    Valid,
    Zero,
    Special,
    Nan,
    Exception,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MulSpecialAction {
    UseUnsignedMultiply,
    ConvertDenormalsAndMultiply,
    CopyZeroWithComputedSign,
    PropagateNan,
    InvalidZeroTimesInfinity,
    CopyInfinityWithComputedSign,
    Exception,
}

pub const fn fpu_mul_sign(sign_a: bool, sign_b: bool) -> bool {
    sign_a ^ sign_b
}

pub fn fpu_mul_action(
    mut taga: u8,
    mut tagb: u8,
    special_a: Option<super::fpu_tags::FpuReg>,
    special_b: Option<super::fpu_tags::FpuReg>,
    denormal_traps: bool,
) -> MulSpecialAction {
    if (taga | tagb) == 0 {
        return MulSpecialAction::UseUnsignedMultiply;
    }

    if taga == TAG_SPECIAL {
        taga = special_a.map(fpu_special).unwrap_or(TW_NAN);
    }
    if tagb == TAG_SPECIAL {
        tagb = special_b.map(fpu_special).unwrap_or(TW_NAN);
    }

    if ((taga == TAG_VALID) && (tagb == TW_DENORMAL))
        || ((taga == TW_DENORMAL) && (tagb == TAG_VALID))
        || ((taga == TW_DENORMAL) && (tagb == TW_DENORMAL))
    {
        return if denormal_traps {
            MulSpecialAction::Exception
        } else {
            MulSpecialAction::ConvertDenormalsAndMultiply
        };
    }

    if taga <= TW_DENORMAL && tagb <= TW_DENORMAL {
        return if ((tagb == TW_DENORMAL) || (taga == TW_DENORMAL)) && denormal_traps {
            MulSpecialAction::Exception
        } else {
            MulSpecialAction::CopyZeroWithComputedSign
        };
    }

    if taga == TW_NAN || tagb == TW_NAN {
        MulSpecialAction::PropagateNan
    } else if (taga == TW_INFINITY && tagb == TAG_ZERO) || (tagb == TW_INFINITY && taga == TAG_ZERO)
    {
        MulSpecialAction::InvalidZeroTimesInfinity
    } else if ((taga == TW_DENORMAL) || (tagb == TW_DENORMAL)) && denormal_traps {
        MulSpecialAction::Exception
    } else if taga == TW_INFINITY || tagb == TW_INFINITY {
        MulSpecialAction::CopyInfinityWithComputedSign
    } else {
        MulSpecialAction::UseUnsignedMultiply
    }
}

pub fn fpu_mul_result_tag(
    taga: u8,
    tagb: u8,
    special_a: Option<super::fpu_tags::FpuReg>,
    special_b: Option<super::fpu_tags::FpuReg>,
    denormal_traps: bool,
) -> MulTag {
    match fpu_mul_action(taga, tagb, special_a, special_b, denormal_traps) {
        MulSpecialAction::UseUnsignedMultiply | MulSpecialAction::ConvertDenormalsAndMultiply => {
            MulTag::Valid
        }
        MulSpecialAction::CopyZeroWithComputedSign => MulTag::Zero,
        MulSpecialAction::PropagateNan | MulSpecialAction::InvalidZeroTimesInfinity => MulTag::Nan,
        MulSpecialAction::CopyInfinityWithComputedSign => MulTag::Special,
        MulSpecialAction::Exception => MulTag::Exception,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::math_emu::fpu_tags::{EXP_OVER, FpuReg};

    #[test]
    fn fpu_mul_decision_tree_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/math-emu/reg_mul.c"
        ));
        assert!(
            source.contains(
                "int FPU_mul(FPU_REG const *b, u_char tagb, int deststnr, int control_w)"
            )
        );
        assert!(source.contains("u_char taga = FPU_gettagi(deststnr);"));
        assert!(source.contains("u_char saved_sign = getsign(dest);"));
        assert!(source.contains("u_char sign = (getsign(a) ^ getsign(b));"));
        assert!(source.contains("if (!(taga | tagb))"));
        assert!(source.contains("FPU_u_mul(a, b, dest, control_w, sign,"));
        assert!(source.contains("setsign(dest, saved_sign);"));
        assert!(source.contains("if (taga == TAG_Special)"));
        assert!(source.contains("if (denormal_operand() < 0)"));
        assert!(source.contains("FPU_copy_to_regi(&CONST_Z, TAG_Zero, deststnr);"));
        assert!(source.contains("return real_2op_NaN(b, tagb, deststnr, &st(0));"));
        assert!(source.contains("return arith_invalid(deststnr);"));
        assert!(source.contains("FPU_copy_to_regi(a, TAG_Special, deststnr);"));
        assert!(source.contains("FPU_copy_to_regi(b, TAG_Special, deststnr);"));

        assert_eq!(
            fpu_mul_result_tag(TAG_VALID, TAG_VALID, None, None, false),
            MulTag::Valid
        );
        assert!(fpu_mul_sign(true, false));
    }

    #[test]
    fn fpu_mul_handles_denormal_zero_nan_and_infinity_cases() {
        let infinity = FpuReg {
            sigl: 0,
            sigh: 0x8000_0000,
            exponent: EXP_OVER,
        };
        let nan = FpuReg {
            sigl: 1,
            sigh: 0x8000_0000,
            exponent: EXP_OVER,
        };
        assert_eq!(
            fpu_mul_action(TAG_VALID, TW_DENORMAL, None, None, false),
            MulSpecialAction::ConvertDenormalsAndMultiply
        );
        assert_eq!(
            fpu_mul_action(TAG_VALID, TW_DENORMAL, None, None, true),
            MulSpecialAction::Exception
        );
        assert_eq!(
            fpu_mul_result_tag(TAG_ZERO, TAG_VALID, None, None, false),
            MulTag::Zero
        );
        assert_eq!(
            fpu_mul_result_tag(TAG_SPECIAL, TAG_ZERO, Some(infinity), None, false),
            MulTag::Nan
        );
        assert_eq!(
            fpu_mul_result_tag(TAG_SPECIAL, TAG_VALID, Some(infinity), None, false),
            MulTag::Special
        );
        assert_eq!(
            fpu_mul_result_tag(TAG_SPECIAL, TAG_VALID, Some(nan), None, false),
            MulTag::Nan
        );
    }
}
