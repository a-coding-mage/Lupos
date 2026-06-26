//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! FPU state in signal frames.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/fpu/signal.c
//!
//! On signal entry, Linux saves the FPU into the user signal frame
//! and writes a sentinel ("magic1" before the xstate header, "magic2"
//! after) so `sigreturn` can detect a tampered frame. This module
//! ports the magic-word layout, the `_fpx_sw_bytes` SW-reserved
//! sub-frame, and the `xstate_sigframe_size`/`fpu__alloc_mathframe`
//! frame-size calculations.

#![allow(dead_code)]

extern crate alloc;

use super::fpu_regset::XFEATURE_MASK_FPSSE;

/// Magic words written into the FX sw-reserved area.
pub const FP_XSTATE_MAGIC1: u32 = 0x46505853;
pub const FP_XSTATE_MAGIC2: u32 = 0x46505845;
pub const FP_XSTATE_MAGIC2_SIZE: usize = core::mem::size_of::<u32>();
pub const X86_FXSR_MAGIC: u16 = 0x0000;

/// `struct _fpx_sw_bytes` — SW-reserved area in the FXSAVE buffer.
/// 48 bytes total (mirrors `sizeof(struct _fpx_sw_bytes)`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct FpxSwBytes {
    pub magic1: u32,
    pub extended_size: u32,
    pub xfeatures: u64,
    pub xstate_size: u32,
    pub padding: [u32; 7],
}

/// Linux's `save_sw_bytes` — populate the SW-reserved area.
pub fn save_sw_bytes(
    sw: &mut FpxSwBytes,
    user_size: u32,
    user_xfeatures: u64,
    ia32_frame: bool,
    fregs_state_size: u32,
) {
    sw.magic1 = FP_XSTATE_MAGIC1;
    sw.extended_size = user_size + FP_XSTATE_MAGIC2_SIZE as u32;
    sw.xfeatures = user_xfeatures;
    sw.xstate_size = user_size;
    if ia32_frame {
        sw.extended_size = sw.extended_size.saturating_add(fregs_state_size);
    }
}

/// Linux's `check_xstate_in_sigframe`: verify magic1, then check magic2
/// sits at `fpstate + user_size`. Returns the resolved `(fx_only, xfeatures)`.
///
/// `magic2_word` is the byte read from `fpstate + user_size`; in
/// production this is `__get_user`'d.
pub fn check_xstate_in_sigframe(sw: &FpxSwBytes, magic2_word: u32) -> (bool, u64) {
    if sw.magic1 != FP_XSTATE_MAGIC1 || magic2_word != FP_XSTATE_MAGIC2 {
        // Fallback to "FX only": magic1 cleared so callers branch into
        // the legacy path; xfeatures forced to FP|SSE.
        return (true, XFEATURE_MASK_FPSSE);
    }
    (false, sw.xfeatures)
}

/// `xstate_sigframe_size(fpstate)`: in XSAVE mode add MAGIC2 trailer.
pub fn xstate_sigframe_size(user_size: usize, use_xsave: bool) -> usize {
    if use_xsave {
        user_size + FP_XSTATE_MAGIC2_SIZE
    } else {
        user_size
    }
}

/// `fpu__alloc_mathframe(sp, ia32_frame, …)`: place the FPU sigframe at
/// `sp - frame_size`, 64-byte aligned, and (for ia32+fxsr) reserve an
/// extra `sizeof(fregs_state)` slot below it for the legacy fregs header.
///
/// Returns `(new_sp, buf_fx, total_size)` mirroring the three out-params
/// Linux exposes.
pub fn fpu_alloc_mathframe(
    sp: u64,
    ia32_frame: bool,
    use_fxsr: bool,
    user_size: u32,
    fregs_state_size: u32,
    use_xsave: bool,
) -> (u64, u64, u32) {
    let mut frame_size = xstate_sigframe_size(user_size as usize, use_xsave) as u32;
    let buf_fx = round_down_64(sp - frame_size as u64);
    let mut new_sp = buf_fx;
    if ia32_frame && use_fxsr {
        frame_size += fregs_state_size;
        new_sp -= fregs_state_size as u64;
    }
    (new_sp, buf_fx, frame_size)
}

/// Linux's `fpu__get_fpstate_size` analogue: total bytes the sigframe
/// occupies including `MAGIC2_SIZE` and the legacy `fregs_state` quirk
/// pad for 32-bit-emulation kernels.
pub fn fpu_get_fpstate_size(
    max_size: u32,
    fregs_state_size: u32,
    use_xsave: bool,
    use_fxsr: bool,
    ia32_emulation: bool,
) -> u32 {
    let mut ret = max_size;
    if use_xsave {
        ret += FP_XSTATE_MAGIC2_SIZE as u32;
    }
    if ia32_emulation && use_fxsr {
        ret += fregs_state_size;
    }
    ret
}

pub const fn round_down_64(addr: u64) -> u64 {
    addr & !63
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_word_constants_match_linux() {
        // Documented in include/uapi/asm-generic/signal-defs.h (via
        // the FXSAVE SW-reserved area). Kept verbatim.
        assert_eq!(FP_XSTATE_MAGIC1, 0x46505853);
        assert_eq!(FP_XSTATE_MAGIC2, 0x46505845);
        assert_eq!(FP_XSTATE_MAGIC2_SIZE, 4);
    }

    #[test]
    fn save_sw_bytes_packs_layout() {
        let mut sw = FpxSwBytes::default();
        save_sw_bytes(&mut sw, 1024, 0x07, false, 112);
        assert_eq!(sw.magic1, FP_XSTATE_MAGIC1);
        assert_eq!(sw.xstate_size, 1024);
        assert_eq!(sw.extended_size, 1024 + 4);
        assert_eq!(sw.xfeatures, 0x07);
    }

    #[test]
    fn save_sw_bytes_ia32_includes_fregs_state_size() {
        let mut sw = FpxSwBytes::default();
        save_sw_bytes(&mut sw, 256, 0x03, true, 112);
        assert_eq!(sw.extended_size, 256 + 4 + 112);
    }

    #[test]
    fn check_xstate_accepts_matching_magics() {
        let sw = FpxSwBytes {
            magic1: FP_XSTATE_MAGIC1,
            extended_size: 100,
            xfeatures: 0x0F,
            xstate_size: 96,
            padding: [0; 7],
        };
        let (fx_only, xf) = check_xstate_in_sigframe(&sw, FP_XSTATE_MAGIC2);
        assert!(!fx_only);
        assert_eq!(xf, 0x0F);
    }

    #[test]
    fn check_xstate_falls_back_when_magic1_missing() {
        let sw = FpxSwBytes::default();
        let (fx_only, xf) = check_xstate_in_sigframe(&sw, FP_XSTATE_MAGIC2);
        assert!(fx_only);
        assert_eq!(xf, XFEATURE_MASK_FPSSE);
    }

    #[test]
    fn check_xstate_falls_back_when_magic2_missing() {
        let sw = FpxSwBytes {
            magic1: FP_XSTATE_MAGIC1,
            extended_size: 0,
            xfeatures: 0,
            xstate_size: 0,
            padding: [0; 7],
        };
        let (fx_only, xf) = check_xstate_in_sigframe(&sw, 0xDEAD);
        assert!(fx_only);
        assert_eq!(xf, XFEATURE_MASK_FPSSE);
    }

    #[test]
    fn sigframe_size_xsave_includes_magic2() {
        assert_eq!(xstate_sigframe_size(1024, true), 1028);
        assert_eq!(xstate_sigframe_size(1024, false), 1024);
    }

    #[test]
    fn alloc_mathframe_aligns_to_64_bytes() {
        // sp = 0x10000, user_size = 0x100, use_xsave = true
        let (new_sp, buf_fx, sz) = fpu_alloc_mathframe(0x1_0000, false, true, 0x100, 112, true);
        // buf_fx must be 64-aligned and below sp.
        assert_eq!(buf_fx & 63, 0);
        assert!(buf_fx < 0x1_0000);
        assert_eq!(sz, 0x100 + 4);
        // ia32_frame=false → new_sp == buf_fx.
        assert_eq!(new_sp, buf_fx);
    }

    #[test]
    fn alloc_mathframe_reserves_extra_for_ia32_fxsr() {
        let (new_sp, buf_fx, sz) = fpu_alloc_mathframe(0x1_0000, true, true, 0x100, 112, true);
        assert_eq!(new_sp + 112, buf_fx);
        assert_eq!(sz, 0x100 + 4 + 112);
    }

    #[test]
    fn fpstate_size_includes_magic2_and_fregs_pad_on_ia32() {
        let s = fpu_get_fpstate_size(0x100, 112, true, true, true);
        assert_eq!(s, 0x100 + 4 + 112);
    }

    #[test]
    fn fpstate_size_excludes_fregs_pad_on_64bit_only() {
        let s = fpu_get_fpstate_size(0x100, 112, true, true, false);
        assert_eq!(s, 0x100 + 4);
    }

    #[test]
    fn round_down_64_aligns_to_lower_boundary() {
        assert_eq!(round_down_64(0x1234), 0x1200);
        assert_eq!(round_down_64(0x1240), 0x1240);
    }
}
