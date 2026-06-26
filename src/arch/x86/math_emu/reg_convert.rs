//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/math-emu/reg_convert.c
//! test-origin: linux:vendor/linux/arch/x86/math-emu/reg_convert.c
//! x87 emulator register conversion to 16-bit exponent form.

pub const EXP_UNDER: i32 = -0x3fff;
pub const EX_INTERNAL_180: u16 = 0x180;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FpuReg {
    pub sigl: u32,
    pub sigh: u32,
    pub exponent: i32,
    pub sign: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Exp16Conversion {
    pub result: FpuReg,
    pub sign: bool,
    pub normalized_denormal: bool,
    pub pseudodenormal_promoted: bool,
    pub internal_exception: Option<u16>,
}

pub fn fpu_to_exp16(input: FpuReg) -> Exp16Conversion {
    let mut result = input;
    let mut normalized_denormal = false;
    let mut pseudodenormal_promoted = false;

    if result.exponent == EXP_UNDER {
        if result.sigh & 0x8000_0000 != 0 {
            result.exponent += 1;
            pseudodenormal_promoted = true;
        } else {
            result.exponent += 1;
            result = normalize_no_underflow(result);
            normalized_denormal = true;
        }
    }

    let internal_exception = if result.sigh & 0x8000_0000 == 0 {
        Some(EX_INTERNAL_180)
    } else {
        None
    };

    Exp16Conversion {
        result,
        sign: input.sign,
        normalized_denormal,
        pseudodenormal_promoted,
        internal_exception,
    }
}

fn normalize_no_underflow(mut value: FpuReg) -> FpuReg {
    while value.sigh & 0x8000_0000 == 0 && (value.sigh != 0 || value.sigl != 0) {
        value.sigh = (value.sigh << 1) | (value.sigl >> 31);
        value.sigl <<= 1;
        value.exponent -= 1;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fpu_to_exp16_matches_linux_denormal_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/math-emu/reg_convert.c"
        ));
        assert!(source.contains("int FPU_to_exp16(FPU_REG const *a, FPU_REG *x)"));
        assert!(source.contains("int sign = getsign(a);"));
        assert!(source.contains("setexponent16(x, exponent(a));"));
        assert!(source.contains("if (exponent16(x) == EXP_UNDER)"));
        assert!(source.contains("x->sigh & 0x80000000"));
        assert!(source.contains("FPU_normalize_nuo(x);"));
        assert!(source.contains("EXCEPTION(EX_INTERNAL | 0x180);"));
        assert!(source.contains("return sign;"));

        let pseudo = fpu_to_exp16(FpuReg {
            sigl: 1,
            sigh: 0x8000_0000,
            exponent: EXP_UNDER,
            sign: true,
        });
        assert!(pseudo.sign);
        assert!(pseudo.pseudodenormal_promoted);
        assert_eq!(pseudo.result.exponent, EXP_UNDER + 1);

        let denormal = fpu_to_exp16(FpuReg {
            sigl: 0,
            sigh: 0x4000_0000,
            exponent: EXP_UNDER,
            sign: false,
        });
        assert!(denormal.normalized_denormal);
        assert_eq!(denormal.internal_exception, None);
    }
}
