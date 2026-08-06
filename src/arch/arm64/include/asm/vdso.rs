// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/arm64/include/asm/vdso.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S000191

/*
 * Copyright (C) 2012 ARM Limited
 */

/// Number of vDSO data pages reserved by the arm64 vDSO ABI.
pub const __VDSO_PAGES: usize = 4;

/*
 * These symbols delimit page-aligned ELF images embedded by vdso-wrap.S and
 * vdso32-wrap.S. C declares incomplete `char` arrays. A non-ZST byte is used
 * only as the Rust address anchor: it is not Rust-owned storage and does not
 * describe the image extent.
 */
// SAFETY: the preserved LINUX_ARCH_ASM objects S000292 and S000294 define
// these four global labels for the frozen CONFIG_COMPAT_VDSO=y image. They
// have kernel-image lifetime; this header forms raw addresses only and never
// creates references to or dereferences the linker-owned storage.
unsafe extern "C" {
    #[link_name = "vdso_start"]
    pub(crate) static mut vdso_start: u8;
    #[link_name = "vdso_end"]
    pub(crate) static mut vdso_end: u8;
    #[link_name = "vdso32_start"]
    pub(crate) static mut vdso32_start: u8;
    #[link_name = "vdso32_end"]
    pub(crate) static mut vdso32_end: u8;
}

/// `include/generated/vdso-offsets.h`, materialized in the frozen Phase 0
/// evidence, defines exactly `vdso_offset_sigtramp 0x08d0` for this target.
/// AArch64 `unsigned long` is 64 bits, so the generated value uses `u64`.
pub(crate) const vdso_offset_sigtramp: u64 = 0x08d0;

/// Rust spelling of the selected `VDSO_SYMBOL(base, sigtramp)` expansion.
/// The identifier arm is the frozen generated-header token set: matching it
/// performs no runtime evaluation, while the base expression is evaluated
/// exactly once as in the C statement expression.
macro_rules! VDSO_SYMBOL {
    ($base:expr, sigtramp) => {{
        let __vdso_base = $base;
        let __vdso_addr = ((__vdso_base as usize) as u64).wrapping_add(vdso_offset_sigtramp);
        __vdso_addr as usize as *mut core::ffi::c_void
    }};
}
pub(crate) use VDSO_SYMBOL;
