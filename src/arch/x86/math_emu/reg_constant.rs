//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/math-emu/reg_constant.c
//! test-origin: linux:vendor/linux/arch/x86/math-emu/reg_constant.c
//! x87 emulator constant table and rounding adjustments.

use super::fpu_tags::{EXP_OVER, EXP_UNDER, TAG_VALID, TAG_ZERO};

pub const EXTENDED_EBIAS: i32 = 0x3fff;
pub const SIGN_POS: u16 = 0;
pub const SIGN_NEG: u16 = 0x8000;

pub const RC_RND: u16 = 0x0000;
pub const RC_DOWN: u16 = 0x0400;
pub const RC_UP: u16 = 0x0800;
pub const RC_CHOP: u16 = 0x0c00;
pub const CW_RC: u16 = 0x0c00;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FpuRegConstant {
    pub sigl: u32,
    pub sigh: u32,
    pub exp: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FpuConstantOpcode {
    One,
    Log2Ten,
    Log2E,
    Pi,
    Log10Two,
    Ln2,
    Zero,
    Illegal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadedConstant {
    pub reg: FpuRegConstant,
    pub tag: u8,
    pub clear_c1: bool,
}

pub const fn make_reg(negative: bool, exponent: i32, sigl: u32, sigh: u32) -> FpuRegConstant {
    let sign = if negative { SIGN_NEG } else { SIGN_POS };
    FpuRegConstant {
        sigl,
        sigh,
        exp: ((EXTENDED_EBIAS + exponent) as u16) | sign,
    }
}

pub const CONST_1: FpuRegConstant = make_reg(false, 0, 0x0000_0000, 0x8000_0000);
pub const CONST_L2T: FpuRegConstant = make_reg(false, 1, 0xcd1b_8afe, 0xd49a_784b);
pub const CONST_L2E: FpuRegConstant = make_reg(false, 0, 0x5c17_f0bc, 0xb8aa_3b29);
pub const CONST_PI: FpuRegConstant = make_reg(false, 1, 0x2168_c235, 0xc90f_daa2);
pub const CONST_PI2: FpuRegConstant = make_reg(false, 0, 0x2168_c235, 0xc90f_daa2);
pub const CONST_PI4: FpuRegConstant = make_reg(false, -1, 0x2168_c235, 0xc90f_daa2);
pub const CONST_LG2: FpuRegConstant = make_reg(false, -2, 0xfbcf_f799, 0x9a20_9a84);
pub const CONST_LN2: FpuRegConstant = make_reg(false, -1, 0xd1cf_79ac, 0xb172_17f7);
pub const CONST_PI2EXTRA: FpuRegConstant = make_reg(true, -66, 0xfc8f_8cbb, 0xece6_75d1);
pub const CONST_Z: FpuRegConstant = make_reg(false, EXP_UNDER, 0, 0);
pub const CONST_QNAN: FpuRegConstant = make_reg(true, EXP_OVER, 0, 0xc000_0000);
pub const CONST_INF: FpuRegConstant = make_reg(false, EXP_OVER, 0, 0x8000_0000);

pub const fn down_or_chop(rc: u16) -> bool {
    rc & RC_DOWN != 0
}

pub fn fld_const(mut reg: FpuRegConstant, adjustment: i32, tag: u8) -> LoadedConstant {
    if adjustment < 0 {
        reg.sigl = reg.sigl.wrapping_sub((-adjustment) as u32);
    } else {
        reg.sigl = reg.sigl.wrapping_add(adjustment as u32);
    }
    LoadedConstant {
        reg,
        tag,
        clear_c1: true,
    }
}

pub fn fconst(opcode: FpuConstantOpcode, control_word: u16) -> Option<LoadedConstant> {
    let rc = control_word & CW_RC;
    match opcode {
        FpuConstantOpcode::One => Some(fld_const(CONST_1, 0, TAG_VALID)),
        FpuConstantOpcode::Log2Ten => Some(fld_const(
            CONST_L2T,
            if rc == RC_UP { 1 } else { 0 },
            TAG_VALID,
        )),
        FpuConstantOpcode::Log2E => Some(fld_const(
            CONST_L2E,
            if down_or_chop(rc) { -1 } else { 0 },
            TAG_VALID,
        )),
        FpuConstantOpcode::Pi => Some(fld_const(
            CONST_PI,
            if down_or_chop(rc) { -1 } else { 0 },
            TAG_VALID,
        )),
        FpuConstantOpcode::Log10Two => Some(fld_const(
            CONST_LG2,
            if down_or_chop(rc) { -1 } else { 0 },
            TAG_VALID,
        )),
        FpuConstantOpcode::Ln2 => Some(fld_const(
            CONST_LN2,
            if down_or_chop(rc) { -1 } else { 0 },
            TAG_VALID,
        )),
        FpuConstantOpcode::Zero => Some(fld_const(CONST_Z, 0, TAG_ZERO)),
        FpuConstantOpcode::Illegal => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fpu_constants_match_linux_source_literals() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/math-emu/reg_constant.c"
        ));
        assert!(source.contains("#define MAKE_REG(s, e, l, h)"));
        assert!(
            source.contains("FPU_REG const CONST_1 = MAKE_REG(POS, 0, 0x00000000, 0x80000000);")
        );
        assert!(source.contains(
            "static FPU_REG const CONST_L2T = MAKE_REG(POS, 1, 0xcd1b8afe, 0xd49a784b);"
        ));
        assert!(
            source.contains("FPU_REG const CONST_PI = MAKE_REG(POS, 1, 0x2168c235, 0xc90fdaa2);")
        );
        assert!(source.contains("FPU_REG const CONST_PI2extra = MAKE_REG(NEG, -66,"));
        assert!(source.contains("FPU_REG const CONST_Z = MAKE_REG(POS, EXP_UNDER, 0x0, 0x0);"));
        assert!(source.contains(
            "FPU_REG const CONST_QNaN = MAKE_REG(NEG, EXP_OVER, 0x00000000, 0xC0000000);"
        ));
        assert!(source.contains(
            "FPU_REG const CONST_INF = MAKE_REG(POS, EXP_OVER, 0x00000000, 0x80000000);"
        ));
        assert!(source.contains("st_new_ptr->sigl += adj;"));
        assert!(source.contains("#define DOWN_OR_CHOP(x)  (x & RC_DOWN)"));
        assert!(source.contains("fld_const(&CONST_L2T, (rc == RC_UP) ? 1 : 0, TAG_Valid);"));
        assert!(source.contains("constants_table[FPU_rm]"));

        assert_eq!(CONST_1, make_reg(false, 0, 0, 0x8000_0000));
        assert_eq!(CONST_PI.sigl, 0x2168_c235);
        assert_eq!(CONST_PI.sigh, 0xc90f_daa2);
        assert_eq!(CONST_PI2EXTRA.exp & SIGN_NEG, SIGN_NEG);
        assert_eq!(CONST_Z.exp & 0x7fff, 0);
        assert_eq!(CONST_QNAN.exp & SIGN_NEG, SIGN_NEG);
        assert_eq!(CONST_INF.sigh, 0x8000_0000);
    }

    #[test]
    fn fconst_applies_rounding_adjustments_and_tags() {
        assert_eq!(
            fconst(FpuConstantOpcode::Log2Ten, RC_UP).unwrap().reg.sigl,
            CONST_L2T.sigl + 1
        );
        assert_eq!(
            fconst(FpuConstantOpcode::Pi, RC_DOWN).unwrap().reg.sigl,
            CONST_PI.sigl - 1
        );
        assert_eq!(
            fconst(FpuConstantOpcode::Zero, RC_RND).unwrap().tag,
            TAG_ZERO
        );
        assert_eq!(fconst(FpuConstantOpcode::Illegal, RC_RND), None);
        assert!(down_or_chop(RC_CHOP));
        assert!(!down_or_chop(RC_UP));
    }
}
