//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/math-emu
//! test-origin: linux:vendor/linux/arch/x86/math-emu
//! x87 math emulation gate.
//!
//! Linux `math-emu` is for 32-bit x86 systems without a hardware FPU. Lupos
//! requires hardware x87/SSE on its x86 boot targets and initializes real FPU
//! state in `fpu.rs`; attempting to route through software x87 emulation is a
//! concrete unsupported path.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/math-emu/errors.c
//! - vendor/linux/arch/x86/math-emu/fpu_arith.c
//! - vendor/linux/arch/x86/math-emu/fpu_aux.c
//! - vendor/linux/arch/x86/math-emu/fpu_entry.c
//! - vendor/linux/arch/x86/math-emu/fpu_etc.c
//! - vendor/linux/arch/x86/math-emu/fpu_tags.c
//! - vendor/linux/arch/x86/math-emu/fpu_trig.c
//! - vendor/linux/arch/x86/math-emu/get_address.c
//! - vendor/linux/arch/x86/math-emu/load_store.c
//! - vendor/linux/arch/x86/math-emu/poly_2xm1.c
//! - vendor/linux/arch/x86/math-emu/poly_atan.c
//! - vendor/linux/arch/x86/math-emu/poly_l2.c
//! - vendor/linux/arch/x86/math-emu/poly_sin.c
//! - vendor/linux/arch/x86/math-emu/poly_tan.c
//! - vendor/linux/arch/x86/math-emu/reg_add_sub.c
//! - vendor/linux/arch/x86/math-emu/reg_compare.c
//! - vendor/linux/arch/x86/math-emu/reg_constant.c
//! - vendor/linux/arch/x86/math-emu/reg_convert.c
//! - vendor/linux/arch/x86/math-emu/reg_divide.c
//! - vendor/linux/arch/x86/math-emu/reg_ld_str.c
//! - vendor/linux/arch/x86/math-emu/reg_mul.c

use crate::include::uapi::errno::{EDOM, ENODEV, ERANGE};

pub mod fpu_tags;
pub mod reg_constant;
pub mod reg_convert;
pub mod reg_mul;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathEmuDecision {
    HardwareFpu,
    UnsupportedNoFpu,
}

pub const fn math_emulation_decision(has_hardware_fpu: bool) -> MathEmuDecision {
    if has_hardware_fpu {
        MathEmuDecision::HardwareFpu
    } else {
        MathEmuDecision::UnsupportedNoFpu
    }
}

pub const fn math_emulation_errno(decision: MathEmuDecision) -> Option<i32> {
    match decision {
        MathEmuDecision::HardwareFpu => None,
        MathEmuDecision::UnsupportedNoFpu => Some(ENODEV),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathEmuFault {
    InvalidOperation,
    DivideByZero,
    StackFault,
    Unsupported,
}

pub const fn math_emu_fault_errno(fault: MathEmuFault) -> i32 {
    match fault {
        MathEmuFault::InvalidOperation | MathEmuFault::StackFault => ERANGE,
        MathEmuFault::DivideByZero => EDOM,
        MathEmuFault::Unsupported => ENODEV,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X87Tag {
    Valid,
    Zero,
    Special,
    Empty,
}

pub const fn encode_x87_tag(tag: X87Tag) -> u16 {
    match tag {
        X87Tag::Valid => 0,
        X87Tag::Zero => 1,
        X87Tag::Special => 2,
        X87Tag::Empty => 3,
    }
}

pub const fn decode_x87_tag(bits: u16) -> X87Tag {
    match bits & 0x3 {
        0 => X87Tag::Valid,
        1 => X87Tag::Zero,
        2 => X87Tag::Special,
        _ => X87Tag::Empty,
    }
}

pub const fn x87_tag_word(tags: [X87Tag; 8]) -> u16 {
    let mut word = 0u16;
    let mut i = 0;
    while i < 8 {
        word |= encode_x87_tag(tags[i]) << (i * 2);
        i += 1;
    }
    word
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X87Status {
    pub top: u8,
    pub c0: bool,
    pub c1: bool,
    pub c2: bool,
    pub c3: bool,
}

pub const fn x87_status_word(status: X87Status) -> u16 {
    let mut word = ((status.top as u16) & 0x7) << 11;
    if status.c0 {
        word |= 1 << 8;
    }
    if status.c1 {
        word |= 1 << 9;
    }
    if status.c2 {
        word |= 1 << 10;
    }
    if status.c3 {
        word |= 1 << 14;
    }
    word
}

pub const fn x87_top_after_push(top: u8) -> u8 {
    top.wrapping_sub(1) & 0x7
}

pub const fn x87_top_after_pop(top: u8) -> u8 {
    top.wrapping_add(1) & 0x7
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedFpuOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub const fn fixed_fpu_arith(lhs: i64, rhs: i64, op: FixedFpuOp) -> Result<i64, MathEmuFault> {
    match op {
        FixedFpuOp::Add => match lhs.checked_add(rhs) {
            Some(v) => Ok(v),
            None => Err(MathEmuFault::InvalidOperation),
        },
        FixedFpuOp::Sub => match lhs.checked_sub(rhs) {
            Some(v) => Ok(v),
            None => Err(MathEmuFault::InvalidOperation),
        },
        FixedFpuOp::Mul => match lhs.checked_mul(rhs) {
            Some(v) => Ok(v),
            None => Err(MathEmuFault::InvalidOperation),
        },
        FixedFpuOp::Div if rhs == 0 => Err(MathEmuFault::DivideByZero),
        FixedFpuOp::Div => Ok(lhs / rhs),
    }
}

pub const fn fixed_fpu_compare(lhs: i64, rhs: i64) -> X87Status {
    X87Status {
        top: 0,
        c0: lhs < rhs,
        c1: false,
        c2: false,
        c3: lhs == rhs,
    }
}

pub const fn fixed_fpu_constant_pi_scaled(scale: i64) -> i64 {
    3141592653589793i64 / scale
}

pub const fn x87_effective_address(base: u32, displacement: i32) -> u32 {
    base.wrapping_add(displacement as u32)
}

pub const fn x87_load_store_len(opcode: u8) -> usize {
    match opcode {
        0xd9 => 4,
        0xdd => 8,
        0xdb => 10,
        _ => 0,
    }
}

pub const fn fixed_poly_sign_reduce(value: i64, period: i64) -> i64 {
    if period == 0 {
        value
    } else {
        let half = period / 2;
        let mut reduced = value % period;
        if reduced > half {
            reduced -= period;
        }
        if reduced < -half {
            reduced += period;
        }
        reduced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_fpu_bypasses_math_emu() {
        assert_eq!(math_emulation_decision(true), MathEmuDecision::HardwareFpu);
        assert_eq!(
            math_emulation_errno(math_emulation_decision(false)),
            Some(ENODEV)
        );
    }

    #[test]
    fn x87_tags_pack_two_bits_per_register() {
        let word = x87_tag_word([
            X87Tag::Valid,
            X87Tag::Zero,
            X87Tag::Special,
            X87Tag::Empty,
            X87Tag::Valid,
            X87Tag::Valid,
            X87Tag::Valid,
            X87Tag::Valid,
        ]);
        assert_eq!(word & 0xff, 0b11_10_01_00);
        assert_eq!(decode_x87_tag((word >> 4) & 0x3), X87Tag::Special);
    }

    #[test]
    fn status_word_keeps_top_and_condition_codes() {
        assert_eq!(
            x87_status_word(X87Status {
                top: 3,
                c0: true,
                c1: false,
                c2: true,
                c3: true,
            }),
            (3 << 11) | (1 << 8) | (1 << 10) | (1 << 14)
        );
        assert_eq!(x87_top_after_push(0), 7);
        assert_eq!(x87_top_after_pop(7), 0);
    }

    #[test]
    fn fixed_arithmetic_reports_math_emu_faults() {
        assert_eq!(fixed_fpu_arith(6, 3, FixedFpuOp::Div), Ok(2));
        assert_eq!(
            fixed_fpu_arith(6, 0, FixedFpuOp::Div),
            Err(MathEmuFault::DivideByZero)
        );
        assert_eq!(math_emu_fault_errno(MathEmuFault::DivideByZero), EDOM);
    }

    #[test]
    fn addressing_and_load_store_classifiers_are_bounded() {
        assert_eq!(x87_effective_address(0xffff_ffff, 2), 1);
        assert_eq!(x87_load_store_len(0xdd), 8);
        assert_eq!(fixed_poly_sign_reduce(7, 10), -3);
    }
}
