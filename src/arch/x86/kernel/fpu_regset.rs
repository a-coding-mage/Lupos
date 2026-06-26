//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! FPU register set conversions for ptrace / core-dump.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/fpu/regset.c
//!
//! `ptrace(PTRACE_GETFPREGS)` and `core_dump` expose two views of the
//! FPU state: the IA-32 user-space layout (`user_i387_ia32_struct`) and
//! the modern FXSAVE layout (`fxregs_state`). This module ports the
//! conversion algorithm (`twd_i387_to_fxsr`, `twd_fxsr_to_i387`,
//! `convert_to_fxsr`, `convert_from_fxsr`) and the `xfpregs_set` /
//! `xstateregs_set` error-path validation.

#![allow(dead_code)]

extern crate alloc;

use crate::include::uapi::errno::{EFAULT, EINVAL, ENODEV};

use super::fpu::FXSAVE_AREA_SIZE;

/// Size of the IA-32 user-space FPU layout.
pub const USER_I387_IA32_SIZE: usize = 28 /* env */ + 8 * 10 /* st_space */ ;

/// Linux's FP tag-word values.
pub const FP_EXP_TAG_VALID: u32 = 0;
pub const FP_EXP_TAG_ZERO: u32 = 1;
pub const FP_EXP_TAG_SPECIAL: u32 = 2;
pub const FP_EXP_TAG_EMPTY: u32 = 3;

/// XFEATURE bits used in the xsave header (subset).
pub const XFEATURE_MASK_FP: u64 = 1 << 0;
pub const XFEATURE_MASK_SSE: u64 = 1 << 1;
pub const XFEATURE_MASK_FPSSE: u64 = XFEATURE_MASK_FP | XFEATURE_MASK_SSE;

/// `twd_i387_to_fxsr` — collapse the 16-bit 387 tag word (2 bits per
/// register) into the 8-bit FXSAVE tag (1 bit per register, 0 = valid).
///
/// This is the exact bit-shuffle Linux performs:
/// `tmp = ~twd`, then a sequence of OR-shifts to gather the V bits.
pub const fn twd_i387_to_fxsr(twd: u16) -> u16 {
    let mut tmp: u32 = (!twd) as u32;
    tmp = (tmp | (tmp >> 1)) & 0x5555;
    tmp = (tmp | (tmp >> 1)) & 0x3333;
    tmp = (tmp | (tmp >> 2)) & 0x0f0f;
    tmp = (tmp | (tmp >> 4)) & 0x00ff;
    tmp as u16
}

/// One 80-bit (10-byte) ST register stored in the IA-32 user layout.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct St80 {
    pub significand: [u16; 4],
    pub exponent: u16,
}

/// One 16-byte ST slot in the FXSAVE area.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct StFxsr {
    pub significand: [u16; 4],
    pub exponent: u16,
    pub pad: [u16; 3],
}

/// FXSAVE area subset: the fields touched by regset conversions.
#[derive(Debug, Clone, Copy)]
pub struct FxRegsState {
    pub cwd: u16,
    pub swd: u16,
    pub twd: u16,
    pub fop: u16,
    pub rip: u64,
    pub rdp: u64,
    pub mxcsr: u32,
    pub mxcsr_mask: u32,
    pub st_space: [StFxsr; 8],
    pub xmm_space: [u8; 16 * 16],
    pub padding: [u8; 64],
}

impl Default for FxRegsState {
    fn default() -> Self {
        Self {
            cwd: 0,
            swd: 0,
            twd: 0,
            fop: 0,
            rip: 0,
            rdp: 0,
            mxcsr: 0,
            mxcsr_mask: 0,
            st_space: [StFxsr::default(); 8],
            xmm_space: [0; 16 * 16],
            padding: [0; 64],
        }
    }
}

/// IA-32 user-space FPU layout (`struct user_i387_ia32_struct`).
#[derive(Debug, Clone, Copy, Default)]
pub struct UserI387Ia32 {
    pub cwd: u32,
    pub swd: u32,
    pub twd: u32,
    pub fip: u32,
    pub fcs: u32,
    pub foo: u32,
    pub fos: u32,
    pub st_space: [St80; 8],
}

/// `twd_fxsr_to_i387` — expand the 8-bit FXSAVE tag into the 16-bit 387
/// tag word, classifying each ST register as Valid / Zero / Special /
/// Empty according to its exponent and significand.
pub fn twd_fxsr_to_i387(fxsave: &FxRegsState) -> u32 {
    let tos = ((fxsave.swd >> 11) & 7) as u32;
    let mut twd = fxsave.twd as u32;
    let mut ret = 0xffff_0000u32;

    for i in 0..8u32 {
        let tag = if twd & 0x1 == 1 {
            let phys_idx = ((i.wrapping_sub(tos)) & 7) as usize;
            let st = &fxsave.st_space[phys_idx];
            match st.exponent & 0x7fff {
                0x7fff => FP_EXP_TAG_SPECIAL,
                0x0000 => {
                    if st.significand.iter().all(|w| *w == 0) {
                        FP_EXP_TAG_ZERO
                    } else {
                        FP_EXP_TAG_SPECIAL
                    }
                }
                _ => {
                    if st.significand[3] & 0x8000 != 0 {
                        FP_EXP_TAG_VALID
                    } else {
                        FP_EXP_TAG_SPECIAL
                    }
                }
            }
        } else {
            FP_EXP_TAG_EMPTY
        };
        ret |= tag << (2 * i);
        twd >>= 1;
    }
    ret
}

/// `convert_to_fxsr`: import IA-32 user env into the FXSAVE area.
/// (Subset relevant to ABI parity — no 32-bit-only fields.)
pub fn convert_to_fxsr(fxsave: &mut FxRegsState, env: &UserI387Ia32) {
    fxsave.cwd = env.cwd as u16;
    fxsave.swd = env.swd as u16;
    fxsave.twd = twd_i387_to_fxsr(env.twd as u16);
    fxsave.fop = (env.fcs >> 16) as u16;
    fxsave.rip = env.fip as u64;
    fxsave.rdp = env.foo as u64;
    for i in 0..8 {
        fxsave.st_space[i].significand = env.st_space[i].significand;
        fxsave.st_space[i].exponent = env.st_space[i].exponent;
    }
}

/// `__convert_from_fxsr`: export the FXSAVE area into the IA-32 user
/// layout. `cs` is the value of `pt_regs->cs` (the 64-bit kernel can't
/// read the real ds/cs at FPU-exception time).
pub fn convert_from_fxsr(env: &mut UserI387Ia32, fxsave: &FxRegsState, cs: u16) {
    env.cwd = (fxsave.cwd as u32) | 0xffff_0000;
    env.swd = (fxsave.swd as u32) | 0xffff_0000;
    env.twd = twd_fxsr_to_i387(fxsave);
    env.fip = fxsave.rip as u32;
    env.foo = fxsave.rdp as u32;
    env.fcs = cs as u32;
    env.fos = 0xffff_0000;
    for i in 0..8 {
        env.st_space[i].significand = fxsave.st_space[i].significand;
        env.st_space[i].exponent = fxsave.st_space[i].exponent;
    }
}

/// Trait seam for `boot_cpu_has(X86_FEATURE_FXSR)` / `X86_FEATURE_XSAVE`
/// and `mxcsr_feature_mask`.
pub trait FpuCpuFeatures {
    fn has_fxsr(&self) -> bool;
    fn has_xsave(&self) -> bool;
    fn mxcsr_feature_mask(&self) -> u32;
}

pub struct ModernCpu;
impl FpuCpuFeatures for ModernCpu {
    fn has_fxsr(&self) -> bool {
        true
    }
    fn has_xsave(&self) -> bool {
        true
    }
    fn mxcsr_feature_mask(&self) -> u32 {
        0x0000_FFBF
    }
}

/// Linux's `xfpregs_set`: validate `pos == 0 && count == sizeof(fxregs)`
/// and a non-bogus `MXCSR`. Mirrors only the validation; the actual
/// `memcpy` runs over a trait seam.
pub fn xfpregs_set<C: FpuCpuFeatures>(
    cpu: &C,
    pos: u32,
    count: u32,
    new_mxcsr: u32,
) -> Result<(), i32> {
    if !cpu.has_fxsr() {
        return Err(ENODEV);
    }
    if pos != 0 || count as usize != FXSAVE_AREA_SIZE {
        return Err(EINVAL);
    }
    if new_mxcsr & !cpu.mxcsr_feature_mask() != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

/// Linux's `xstateregs_set`: validate `pos == 0 && count == max_size`.
/// `EFAULT` is the kernel's choice for "wrong-sized buffer" in this path.
pub fn xstateregs_set<C: FpuCpuFeatures>(
    cpu: &C,
    pos: u32,
    count: u32,
    max_size: u32,
) -> Result<(), i32> {
    if !cpu.has_xsave() {
        return Err(ENODEV);
    }
    if pos != 0 || count != max_size {
        return Err(EFAULT);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twd_i387_all_empty_collapses_to_zero() {
        // All 2-bit slots = 11 (empty) → 0xFFFF → ~ = 0 → result 0.
        assert_eq!(twd_i387_to_fxsr(0xFFFF), 0x00);
    }

    #[test]
    fn twd_i387_all_valid_expands_to_0xff() {
        // All 2-bit slots = 00 (valid) → 0x0000 → ~ = 0xFFFF → 0xFF.
        assert_eq!(twd_i387_to_fxsr(0x0000), 0xFF);
    }

    #[test]
    fn twd_i387_mixed_slots() {
        // Slot 0 valid, slots 1-7 empty: 0xFFFC → ~ = 0x0003 → 0x01.
        assert_eq!(twd_i387_to_fxsr(0xFFFC), 0x01);
    }

    #[test]
    fn twd_fxsr_all_empty_marks_each_slot_empty() {
        let fx = FxRegsState::default();
        let twd = twd_fxsr_to_i387(&fx);
        // All slots empty → 0xFFFF for the low 16 bits.
        assert_eq!(twd & 0xFFFF, 0xFFFF);
        assert_eq!(twd & 0xFFFF_0000, 0xFFFF_0000);
    }

    #[test]
    fn twd_fxsr_valid_slot_with_normalised_st() {
        let mut fx = FxRegsState::default();
        // Tag bit 0 = 1 (this slot is in use).
        fx.twd = 0x01;
        // Exponent in normal range and bit 15 of significand[3] set
        // → FP_EXP_TAG_VALID = 0.
        fx.st_space[0].exponent = 0x4000;
        fx.st_space[0].significand[3] = 0x8000;
        let twd = twd_fxsr_to_i387(&fx);
        // Slot 0 → tag bits 0-1 = 00 (valid). Other slots empty.
        assert_eq!(twd & 0x3, 0x0);
        assert_eq!(twd & 0xFFFC, 0xFFFC);
    }

    #[test]
    fn twd_fxsr_zero_slot_returns_zero_tag() {
        let mut fx = FxRegsState::default();
        fx.twd = 0x01;
        fx.st_space[0].exponent = 0;
        // significand all zero → FP_EXP_TAG_ZERO = 1.
        let twd = twd_fxsr_to_i387(&fx);
        assert_eq!(twd & 0x3, FP_EXP_TAG_ZERO);
    }

    #[test]
    fn convert_to_fxsr_round_trips_with_convert_from_fxsr() {
        let mut env = UserI387Ia32::default();
        env.cwd = 0x37f;
        env.swd = 0x4000;
        // Slot 0 valid: exponent in normal range, significand[3] bit 15 set.
        env.twd = 0xFFFC; // slot 0 valid only
        env.st_space[0].exponent = 0x4000;
        env.st_space[0].significand = [0, 0, 0, 0x8000];
        let mut fx = FxRegsState::default();
        convert_to_fxsr(&mut fx, &env);

        let mut env2 = UserI387Ia32::default();
        convert_from_fxsr(&mut env2, &fx, 0x33);
        // cwd survives, twd is normalised (slot 0 valid = bit pattern 00, others empty)
        assert_eq!(env2.cwd & 0xFFFF, env.cwd);
        assert_eq!(env2.twd & 0xFFFF, 0xFFFC);
        assert_eq!(env2.fcs, 0x33);
    }

    #[test]
    fn xfpregs_set_rejects_partial_writes() {
        let cpu = ModernCpu;
        let r = xfpregs_set(&cpu, 8, FXSAVE_AREA_SIZE as u32, 0x1F80);
        assert_eq!(r, Err(EINVAL));
    }

    #[test]
    fn xfpregs_set_rejects_oversized_writes() {
        let cpu = ModernCpu;
        let r = xfpregs_set(&cpu, 0, (FXSAVE_AREA_SIZE + 1) as u32, 0x1F80);
        assert_eq!(r, Err(EINVAL));
    }

    #[test]
    fn xfpregs_set_rejects_invalid_mxcsr() {
        let cpu = ModernCpu;
        let r = xfpregs_set(&cpu, 0, FXSAVE_AREA_SIZE as u32, 0xFFFF_FFFF);
        assert_eq!(r, Err(EINVAL));
    }

    #[test]
    fn xfpregs_set_accepts_valid_inputs() {
        let cpu = ModernCpu;
        let r = xfpregs_set(&cpu, 0, FXSAVE_AREA_SIZE as u32, 0x1F80);
        assert!(r.is_ok());
    }

    #[test]
    fn xstateregs_set_rejects_wrong_size() {
        let cpu = ModernCpu;
        assert_eq!(xstateregs_set(&cpu, 0, 100, 4096), Err(EFAULT));
    }

    #[test]
    fn xfpregs_set_returns_enodev_without_fxsr() {
        struct NoFxsr;
        impl FpuCpuFeatures for NoFxsr {
            fn has_fxsr(&self) -> bool {
                false
            }
            fn has_xsave(&self) -> bool {
                false
            }
            fn mxcsr_feature_mask(&self) -> u32 {
                0
            }
        }
        assert_eq!(
            xfpregs_set(&NoFxsr, 0, FXSAVE_AREA_SIZE as u32, 0),
            Err(ENODEV)
        );
    }
}
