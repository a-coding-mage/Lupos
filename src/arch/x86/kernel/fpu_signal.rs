//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/tools/testing/selftests/x86/xstate.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/x86/corrupt_xstate_header.c
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
//!
//! Lupos does not yet have Linux's exception-table wrappers for executing
//! FXSAVE/FXRSTOR directly on user memory. The native state therefore passes
//! through one fixed-size aligned kernel buffer and fault-aware uaccess. This
//! preserves the ABI and failure contract but adds one bounded copy per
//! signal entry/return.

#![allow(dead_code)]

extern crate alloc;

use super::fpu::{self, FXSAVE_AREA_SIZE, FxSaveArea};
use super::fpu_regset::{XFEATURE_MASK_FP, XFEATURE_MASK_FPSSE, XFEATURE_MASK_SSE};

/// Magic words written into the FX sw-reserved area.
pub const FP_XSTATE_MAGIC1: u32 = 0x46505853;
pub const FP_XSTATE_MAGIC2: u32 = 0x46505845;
pub const FP_XSTATE_MAGIC2_SIZE: usize = core::mem::size_of::<u32>();
pub const X86_FXSR_MAGIC: u16 = 0x0000;
pub const FXSAVE_SW_RESERVED_OFFSET: usize = 464;
pub const XSAVE_HEADER_OFFSET: usize = FXSAVE_AREA_SIZE;
pub const XSAVE_HEADER_SIZE: usize = 64;
pub const USER_XSTATE_SIZE: usize = XSAVE_HEADER_OFFSET + XSAVE_HEADER_SIZE;
pub const MAX_SIGNAL_FPSTATE_SIZE: usize = USER_XSTATE_SIZE + FP_XSTATE_MAGIC2_SIZE;

const FP_ENV_END: usize = 24;
const MXCSR_END: usize = 32;
const FP_REGS_START: usize = 32;
const FP_REGS_END: usize = 160;
const XMM_REGS_START: usize = 160;
const XMM_REGS_END: usize = 416;

const _: () = {
    assert!(core::mem::size_of::<FpxSwBytes>() == 48);
    assert!(FXSAVE_SW_RESERVED_OFFSET + core::mem::size_of::<FpxSwBytes>() == FXSAVE_AREA_SIZE);
    assert!(USER_XSTATE_SIZE == 576);
};

#[repr(C, align(64))]
struct SignalFpstateImage {
    bytes: [u8; MAX_SIGNAL_FPSTATE_SIZE],
}

impl SignalFpstateImage {
    const fn zeroed() -> Self {
        Self {
            bytes: [0; MAX_SIGNAL_FPSTATE_SIZE],
        }
    }
}

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

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn write_sw_bytes(bytes: &mut [u8], sw: &FpxSwBytes) {
    let base = FXSAVE_SW_RESERVED_OFFSET;
    write_u32(bytes, base, sw.magic1);
    write_u32(bytes, base + 4, sw.extended_size);
    write_u64(bytes, base + 8, sw.xfeatures);
    write_u32(bytes, base + 16, sw.xstate_size);
    for (index, value) in sw.padding.iter().enumerate() {
        write_u32(bytes, base + 20 + index * 4, *value);
    }
}

fn read_sw_bytes(bytes: &[u8]) -> FpxSwBytes {
    let base = FXSAVE_SW_RESERVED_OFFSET;
    let mut padding = [0; 7];
    for (index, value) in padding.iter_mut().enumerate() {
        *value = read_u32(bytes, base + 20 + index * 4);
    }
    FpxSwBytes {
        magic1: read_u32(bytes, base),
        extended_size: read_u32(bytes, base + 4),
        xfeatures: read_u64(bytes, base + 8),
        xstate_size: read_u32(bytes, base + 16),
        padding,
    }
}

/// Number of bytes occupied by the native x86-64 signal fpstate.
pub const fn signal_fpstate_size_for(use_xsave: bool) -> usize {
    if use_xsave {
        MAX_SIGNAL_FPSTATE_SIZE
    } else {
        FXSAVE_AREA_SIZE
    }
}

/// Number of bytes occupied by the boot CPU's native signal fpstate.
pub fn signal_fpstate_size() -> usize {
    signal_fpstate_size_for(fpu::signal_uses_xsave())
}

fn build_signal_fpstate(raw: &FxSaveArea, use_xsave: bool) -> SignalFpstateImage {
    let mut image = SignalFpstateImage::zeroed();
    image.bytes[..FXSAVE_AREA_SIZE].copy_from_slice(&raw.bytes);

    let user_size = if use_xsave {
        USER_XSTATE_SIZE
    } else {
        FXSAVE_AREA_SIZE
    };
    let mut sw = FpxSwBytes::default();
    save_sw_bytes(&mut sw, user_size as u32, XFEATURE_MASK_FPSSE, false, 0);
    write_sw_bytes(&mut image.bytes, &sw);

    if use_xsave {
        // Linux's save_xstate_epilog() forces FP/SSE present in the signal
        // header even when either component currently has architectural init
        // state, then writes magic2 immediately after the user xstate image.
        write_u64(&mut image.bytes, XSAVE_HEADER_OFFSET, XFEATURE_MASK_FPSSE);
        write_u32(&mut image.bytes, USER_XSTATE_SIZE, FP_XSTATE_MAGIC2);
    }
    image
}

/// Linux `copy_fpstate_to_sigframe()` for Lupos's current FP/SSE-only XCR0.
///
/// The destination is the separately allocated 64-byte-aligned fpstate area,
/// not part of `struct rt_sigframe`.
///
/// # Safety
/// `dst` is a user pointer belonging to the current task.
pub unsafe fn copy_fpstate_to_sigframe(dst: u64) -> bool {
    let use_xsave = fpu::signal_uses_xsave();
    let size = signal_fpstate_size_for(use_xsave);
    if dst == 0 || dst & 63 != 0 || !crate::arch::x86::kernel::uaccess::access_ok(dst, size as u64)
    {
        return false;
    }

    let mut raw = FxSaveArea::init_state();
    unsafe { fpu::save_current_user_fxstate(&mut raw) };
    let image = build_signal_fpstate(&raw, use_xsave);
    unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(dst as *mut u8, image.bytes.as_ptr(), size)
            == 0
    }
}

fn select_user_components(
    image: &SignalFpstateImage,
    active_features: u64,
) -> Result<FxSaveArea, ()> {
    if active_features & XFEATURE_MASK_SSE != 0 {
        let mxcsr = read_u32(&image.bytes, FP_ENV_END);
        if mxcsr & !fpu::mxcsr_feature_mask() != 0 {
            return Err(());
        }
    }

    let mut state = FxSaveArea::zeroed();
    state
        .bytes
        .copy_from_slice(&image.bytes[..FXSAVE_AREA_SIZE]);
    let initial = FxSaveArea::init_state();

    if active_features & XFEATURE_MASK_FP == 0 {
        state.bytes[..FP_ENV_END].copy_from_slice(&initial.bytes[..FP_ENV_END]);
        state.bytes[FP_REGS_START..FP_REGS_END]
            .copy_from_slice(&initial.bytes[FP_REGS_START..FP_REGS_END]);
    }
    if active_features & XFEATURE_MASK_SSE == 0 {
        state.bytes[FP_ENV_END..MXCSR_END].copy_from_slice(&initial.bytes[FP_ENV_END..MXCSR_END]);
    }
    if active_features & XFEATURE_MASK_SSE == 0 {
        state.bytes[XMM_REGS_START..XMM_REGS_END]
            .copy_from_slice(&initial.bytes[XMM_REGS_START..XMM_REGS_END]);
    }

    Ok(state)
}

fn decode_signal_fpstate(
    image: &SignalFpstateImage,
    user_addr: u64,
    use_xsave: bool,
) -> Result<FxSaveArea, ()> {
    let sw = read_sw_bytes(&image.bytes);
    let mut extended = false;

    if use_xsave
        && sw.magic1 == FP_XSTATE_MAGIC1
        && sw.xstate_size as usize >= USER_XSTATE_SIZE
        && sw.xstate_size as usize <= USER_XSTATE_SIZE
        && sw.xstate_size <= sw.extended_size
    {
        let magic2 = read_u32(&image.bytes, sw.xstate_size as usize);
        extended = magic2 == FP_XSTATE_MAGIC2;
    }

    if !extended {
        if user_addr & 15 != 0 {
            return Err(());
        }
        return select_user_components(image, XFEATURE_MASK_FPSSE);
    }

    if user_addr & 63 != 0 {
        return Err(());
    }

    let header_features = read_u64(&image.bytes, XSAVE_HEADER_OFFSET);
    if header_features & !XFEATURE_MASK_FPSSE != 0
        || image.bytes[XSAVE_HEADER_OFFSET + 8..USER_XSTATE_SIZE]
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(());
    }

    // XRSTOR applies the requested feature mask from sw_reserved and treats
    // components absent from XSTATE_BV as architectural init state.
    let active_features = sw.xfeatures & header_features & XFEATURE_MASK_FPSSE;
    select_user_components(image, active_features)
}

/// Linux `fpu__restore_sig()` for Lupos's current FP/SSE-only XCR0.
///
/// A null pointer resets both components. Invalid user state also resets the
/// current task before returning failure, as Linux does in its common `out`
/// path.
///
/// # Safety
/// `src` is the fpstate pointer supplied by the current task's sigcontext.
pub unsafe fn restore_fpstate_from_sigframe(src: u64) -> bool {
    if src == 0 {
        unsafe { fpu::clear_current_user_fxstate() };
        return true;
    }

    let use_xsave = fpu::signal_uses_xsave();
    // Linux first reads the 512-byte FX image and only accesses the XSAVE
    // header/magic2 when sw_reserved describes an extended frame.  Requiring
    // all 580 bytes up front would reject a valid legacy FX-only image whose
    // final byte sits at the end of a mapped page.
    if !crate::arch::x86::kernel::uaccess::access_ok(src, FXSAVE_AREA_SIZE as u64) {
        unsafe { fpu::clear_current_user_fxstate() };
        return false;
    }

    let mut image = SignalFpstateImage::zeroed();
    if unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            image.bytes.as_mut_ptr(),
            src as *const u8,
            FXSAVE_AREA_SIZE,
        )
    } != 0
    {
        unsafe { fpu::clear_current_user_fxstate() };
        return false;
    }

    let sw = read_sw_bytes(&image.bytes);
    let extended_candidate = use_xsave
        && sw.magic1 == FP_XSTATE_MAGIC1
        && sw.xstate_size as usize == USER_XSTATE_SIZE
        && sw.xstate_size <= sw.extended_size;
    if extended_candidate {
        let tail_size = MAX_SIGNAL_FPSTATE_SIZE - FXSAVE_AREA_SIZE;
        let tail_addr = src + FXSAVE_AREA_SIZE as u64;
        if !crate::arch::x86::kernel::uaccess::access_ok(tail_addr, tail_size as u64)
            || unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    image.bytes[FXSAVE_AREA_SIZE..].as_mut_ptr(),
                    tail_addr as *const u8,
                    tail_size,
                )
            } != 0
        {
            unsafe { fpu::clear_current_user_fxstate() };
            return false;
        }
    }

    match decode_signal_fpstate(&image, src, use_xsave) {
        Ok(state) => {
            unsafe { fpu::restore_current_user_fxstate(&state) };
            true
        }
        Err(()) => {
            unsafe { fpu::clear_current_user_fxstate() };
            false
        }
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

    #[test]
    fn native_xstate_signal_image_matches_linux_uapi_layout() {
        // Adapted from selftests/x86/xstate.c::validate_sigfpstate().
        let mut raw = FxSaveArea::init_state();
        for (index, byte) in raw.bytes[XMM_REGS_START..XMM_REGS_START + 16]
            .iter_mut()
            .enumerate()
        {
            *byte = 0x80 + index as u8;
        }

        let image = build_signal_fpstate(&raw, true);
        let sw = read_sw_bytes(&image.bytes);
        assert_eq!(sw.magic1, FP_XSTATE_MAGIC1);
        assert_eq!(sw.extended_size as usize, MAX_SIGNAL_FPSTATE_SIZE);
        assert_eq!(sw.xfeatures, XFEATURE_MASK_FPSSE);
        assert_eq!(sw.xstate_size as usize, USER_XSTATE_SIZE);
        assert_eq!(
            read_u64(&image.bytes, XSAVE_HEADER_OFFSET),
            XFEATURE_MASK_FPSSE
        );
        assert!(
            image.bytes[XSAVE_HEADER_OFFSET + 8..USER_XSTATE_SIZE]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(read_u32(&image.bytes, USER_XSTATE_SIZE), FP_XSTATE_MAGIC2);
        assert_eq!(
            &image.bytes[XMM_REGS_START..XMM_REGS_START + 16],
            &raw.bytes[XMM_REGS_START..XMM_REGS_START + 16]
        );
    }

    #[test]
    fn sigreturn_imports_xmm_edits_from_signal_handler() {
        // xstate.c writes new random state through uc_mcontext.fpregs and
        // verifies that sigreturn makes it live.
        let raw = FxSaveArea::init_state();
        let mut image = build_signal_fpstate(&raw, true);
        let replacement = [
            0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45,
            0x23, 0x01,
        ];
        image.bytes[XMM_REGS_START..XMM_REGS_START + replacement.len()]
            .copy_from_slice(&replacement);

        let restored = decode_signal_fpstate(&image, 0x10_000, true).unwrap();
        assert_eq!(
            &restored.bytes[XMM_REGS_START..XMM_REGS_START + replacement.len()],
            &replacement
        );
    }

    #[test]
    fn sigreturn_rejects_corrupt_xstate_header_reserved_bits() {
        // Adapted from selftests/x86/corrupt_xstate_header.c, which writes
        // the first reserved u64 in the XSAVE header.
        let raw = FxSaveArea::init_state();
        let mut image = build_signal_fpstate(&raw, true);
        write_u64(&mut image.bytes, XSAVE_HEADER_OFFSET + 16, 0x0fff_ffff);

        assert!(decode_signal_fpstate(&image, 0x10_000, true).is_err());
    }

    #[test]
    fn sigreturn_initializes_components_cleared_from_xstate_bv() {
        let mut raw = FxSaveArea::init_state();
        raw.bytes[XMM_REGS_START..XMM_REGS_START + 16].fill(0xa5);
        write_u32(&mut raw.bytes, FP_ENV_END, 0);
        let mut image = build_signal_fpstate(&raw, true);
        write_u64(&mut image.bytes, XSAVE_HEADER_OFFSET, XFEATURE_MASK_FP);

        let restored = decode_signal_fpstate(&image, 0x10_000, true).unwrap();
        assert_eq!(
            &restored.bytes[XMM_REGS_START..XMM_REGS_START + 16],
            &[0; 16]
        );
        assert_eq!(read_u32(&restored.bytes, FP_ENV_END), 0x1f80);
    }

    #[test]
    fn sigreturn_rejects_invalid_mxcsr_when_sse_is_restored() {
        let raw = FxSaveArea::init_state();
        let mut image = build_signal_fpstate(&raw, true);
        write_u32(&mut image.bytes, FP_ENV_END, u32::MAX);

        assert!(decode_signal_fpstate(&image, 0x10_000, true).is_err());
    }

    #[test]
    fn bad_magic2_falls_back_to_fx_only_like_linux() {
        let mut raw = FxSaveArea::init_state();
        raw.bytes[XMM_REGS_START] = 0x5a;
        let mut image = build_signal_fpstate(&raw, true);
        write_u32(&mut image.bytes, USER_XSTATE_SIZE, 0);

        // FXRSTOR requires 16-byte, not 64-byte, alignment in this fallback.
        let restored = decode_signal_fpstate(&image, 0x10_010, true).unwrap();
        assert_eq!(restored.bytes[XMM_REGS_START], 0x5a);
    }
}
