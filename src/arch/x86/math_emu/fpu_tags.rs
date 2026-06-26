//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/math-emu/fpu_tags.c
//! test-origin: linux:vendor/linux/arch/x86/math-emu/fpu_tags.c
//! x87 software-emulator tag word helpers.

pub const TAG_VALID: u8 = 0;
pub const TAG_ZERO: u8 = 1;
pub const TAG_SPECIAL: u8 = 2;
pub const TAG_EMPTY: u8 = 3;
pub const TAG_ERROR: u8 = 0x80;

pub const TW_DENORMAL: u8 = 4;
pub const TW_INFINITY: u8 = 5;
pub const TW_NAN: u8 = 6;

pub const EXP_BIAS: i32 = 0;
pub const EXP_UNDER: i32 = -0x3fff;
pub const EXP_OVER: i32 = 0x4000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FpuReg {
    pub sigl: u32,
    pub sigh: u32,
    pub exponent: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FpuTagState {
    pub fpu_tag_word: u16,
    pub top: u8,
}

impl FpuTagState {
    pub const fn new(fpu_tag_word: u16, top: u8) -> Self {
        Self {
            fpu_tag_word,
            top: top & 7,
        }
    }

    pub fn fpu_pop(&mut self) {
        self.fpu_tag_word |= 3 << ((self.top & 7) * 2);
        self.top = self.top.wrapping_add(1) & 7;
    }

    pub const fn fpu_gettag0(self) -> u8 {
        ((self.fpu_tag_word >> ((self.top & 7) * 2)) & 3) as u8
    }

    pub const fn fpu_gettagi(self, stnr: i32) -> u8 {
        let regnr = ((self.top as i32 + stnr) & 7) as u8;
        ((self.fpu_tag_word >> (regnr * 2)) & 3) as u8
    }

    pub const fn fpu_gettag(self, regnr: i32) -> u8 {
        ((self.fpu_tag_word >> (((regnr & 7) as u8) * 2)) & 3) as u8
    }

    pub fn fpu_settag0(&mut self, tag: u8) {
        let regnr = self.top & 7;
        self.set_tag_for_reg(regnr, tag);
    }

    pub fn fpu_settagi(&mut self, stnr: i32, tag: u8) {
        let regnr = ((self.top as i32 + stnr) & 7) as u8;
        self.set_tag_for_reg(regnr, tag);
    }

    pub fn fpu_settag(&mut self, regnr: i32, tag: u8) {
        self.set_tag_for_reg((regnr & 7) as u8, tag);
    }

    pub const fn fpu_empty_i(self, stnr: i32) -> bool {
        self.fpu_gettagi(stnr) == TAG_EMPTY
    }

    pub const fn fpu_stackoverflow(self) -> bool {
        let regnr = self.top.wrapping_sub(1) & 7;
        ((self.fpu_tag_word >> (regnr * 2)) & 3) as u8 != TAG_EMPTY
    }

    fn set_tag_for_reg(&mut self, regnr: u8, tag: u8) {
        self.fpu_tag_word &= !(3 << (regnr * 2));
        self.fpu_tag_word |= ((tag as u16) & 3) << (regnr * 2);
    }
}

pub const fn fpu_special(reg: FpuReg) -> u8 {
    let exp = reg.exponent;
    if exp == EXP_BIAS + EXP_UNDER {
        TW_DENORMAL
    } else if exp != EXP_BIAS + EXP_OVER {
        TW_NAN
    } else if reg.sigh == 0x8000_0000 && reg.sigl == 0 {
        TW_INFINITY
    } else {
        TW_NAN
    }
}

pub const fn is_nan(reg: FpuReg) -> bool {
    reg.exponent == EXP_BIAS + EXP_OVER && !(reg.sigh == 0x8000_0000 && reg.sigl == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fpu_tag_word_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/math-emu/fpu_tags.c"
        ));
        assert!(source.contains("void FPU_pop(void)"));
        assert!(source.contains("fpu_tag_word |= 3 << ((top & 7) * 2);"));
        assert!(source.contains("return (fpu_tag_word >> ((top & 7) * 2)) & 3;"));
        assert!(source.contains("return (fpu_tag_word >> (((top + stnr) & 7) * 2)) & 3;"));
        assert!(source.contains("fpu_tag_word &= ~(3 << (regnr * 2));"));
        assert!(source.contains("fpu_tag_word |= (tag & 3) << (regnr * 2);"));
        assert!(source.contains("return TW_Denormal;"));
        assert!(source.contains("return TW_Infinity;"));
        assert!(source.contains("return ((exponent(ptr) == EXP_BIAS + EXP_OVER)"));
        assert!(source.contains("return ((fpu_tag_word >> (regnr * 2)) & 3) == TAG_Empty;"));
        assert!(source.contains("*st_new_ptr = &st(-1);"));
        assert!(source.contains("FPU_copy_to_reg0(FPU_REG const *r, u_char tag)"));

        let mut state = FpuTagState::new(0, 3);
        assert_eq!(state.fpu_gettag0(), TAG_VALID);
        state.fpu_settag0(TAG_ZERO);
        assert_eq!(state.fpu_gettag(3), TAG_ZERO);
        state.fpu_settagi(2, TAG_SPECIAL);
        assert_eq!(state.fpu_gettag(5), TAG_SPECIAL);
        state.fpu_pop();
        assert_eq!(state.top, 4);
        assert_eq!(state.fpu_gettag(3), TAG_EMPTY);
    }

    #[test]
    fn fpu_special_classifies_denormal_infinity_and_nan() {
        assert_eq!(
            fpu_special(FpuReg {
                sigl: 1,
                sigh: 0,
                exponent: EXP_UNDER,
            }),
            TW_DENORMAL
        );
        assert_eq!(
            fpu_special(FpuReg {
                sigl: 0,
                sigh: 0x8000_0000,
                exponent: EXP_OVER,
            }),
            TW_INFINITY
        );
        let qnan = FpuReg {
            sigl: 1,
            sigh: 0x8000_0000,
            exponent: EXP_OVER,
        };
        assert_eq!(fpu_special(qnan), TW_NAN);
        assert!(is_nan(qnan));
    }
}
