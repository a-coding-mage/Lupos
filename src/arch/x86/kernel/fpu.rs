//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! Minimal x86-64 FPU/SSE enablement.
//!
//! Userland built by modern musl/Rust uses SSE instructions during C/Rust
//! runtime startup. Linux enables OSFXSR/OSXMMEXCPT before returning to user
//! mode; do the same before the login boot launches real PID1.
//!
//! References:
//! - `vendor/linux/arch/x86/kernel/fpu/core.c`
//! - `vendor/linux/arch/x86/kernel/fpu/init.c`
//! - `vendor/linux/arch/x86/kernel/fpu/xstate.c`

const CR0_MP: u64 = 1 << 1;
const CR0_EM: u64 = 1 << 2;
const CR0_TS: u64 = 1 << 3;
const CR4_OSFXSR: u64 = 1 << 9;
const CR4_OSXMMEXCPT: u64 = 1 << 10;
const CR4_OSXSAVE: u64 = 1 << 18;

pub const XFEATURE_MASK_FP: u64 = 1 << 0;
pub const XFEATURE_MASK_SSE: u64 = 1 << 1;
pub const XFEATURE_MASK_YMM: u64 = 1 << 2;
pub const XFEATURE_MASK_FPSSE: u64 = XFEATURE_MASK_FP | XFEATURE_MASK_SSE;
pub const XFEATURE_MASK_AVX: u64 = XFEATURE_MASK_FPSSE | XFEATURE_MASK_YMM;
pub const FXSAVE_AREA_SIZE: usize = 512;

#[inline]
unsafe fn read_cr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
unsafe fn write_cr0(value: u64) {
    unsafe {
        core::arch::asm!("mov cr0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
unsafe fn read_cr4() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
unsafe fn write_cr4(value: u64) {
    unsafe {
        core::arch::asm!("mov cr4, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

#[cfg(not(test))]
#[inline]
unsafe fn xsetbv0(value: u64) {
    let lo = value as u32;
    let hi = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "xsetbv",
            in("ecx") 0u32,
            in("eax") lo,
            in("edx") hi,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
unsafe fn xsetbv0(_value: u64) {}

pub fn xcr0_mask_for_features(features: crate::arch::x86::kernel::cpu::CpuFeatures) -> u64 {
    if !features.has_xsave() {
        return 0;
    }

    // Do not enable the AVX/YMM xfeature yet.  The context-switch path only
    // preserves GPRs (and, in later Rust-side bookkeeping, FS/GS), so allowing
    // user tasks to execute AVX would leave YMM state shared between tasks.
    // Keep XCR0 limited to the x87/SSE baseline until per-task XSAVE/XRSTOR
    // support is wired into __switch_to.
    XFEATURE_MASK_FPSSE
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FxSaveArea {
    pub bytes: [u8; FXSAVE_AREA_SIZE],
}

impl FxSaveArea {
    pub const fn zeroed() -> Self {
        Self {
            bytes: [0; FXSAVE_AREA_SIZE],
        }
    }
}

impl Default for FxSaveArea {
    fn default() -> Self {
        Self::zeroed()
    }
}

pub const fn xsave_enabled_size(leaf_d0: crate::arch::x86::kernel::cpuid::CpuidResult) -> u32 {
    leaf_d0.ebx
}

pub const fn xsave_supported_size(leaf_d0: crate::arch::x86::kernel::cpuid::CpuidResult) -> u32 {
    leaf_d0.ecx
}

/// Save x87/SSE state into a 16-byte-aligned FXSAVE area.
///
/// # Safety
/// The CPU must support FXSAVE/FXRSTOR and `state` must be writable memory.
#[cfg(not(test))]
pub unsafe fn save_fxstate(state: &mut FxSaveArea) {
    unsafe {
        core::arch::asm!(
            "fxsave64 [{ptr}]",
            ptr = in(reg) state.bytes.as_mut_ptr(),
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
pub unsafe fn save_fxstate(_state: &mut FxSaveArea) {}

/// Restore x87/SSE state from a 16-byte-aligned FXSAVE area.
///
/// # Safety
/// The state image must have been produced by FXSAVE on a compatible CPU.
#[cfg(not(test))]
pub unsafe fn restore_fxstate(state: &FxSaveArea) {
    unsafe {
        core::arch::asm!(
            "fxrstor64 [{ptr}]",
            ptr = in(reg) state.bytes.as_ptr(),
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
pub unsafe fn restore_fxstate(_state: &FxSaveArea) {}

/// Enable the architectural bits required for x87 and SSE instructions.
///
/// # Safety
/// Must run at CPL0 during early CPU bring-up, before user mode is entered.
pub unsafe fn init() {
    let mut cr0 = unsafe { read_cr0() };
    cr0 |= CR0_MP;
    cr0 &= !(CR0_EM | CR0_TS);
    unsafe { write_cr0(cr0) };

    let mut cr4 = unsafe { read_cr4() };
    cr4 |= CR4_OSFXSR | CR4_OSXMMEXCPT;
    let features = crate::arch::x86::kernel::cpu::CpuFeatures::current();
    let xcr0 = xcr0_mask_for_features(features);
    if xcr0 != 0 {
        cr4 |= CR4_OSXSAVE;
    }
    unsafe { write_cr4(cr4) };

    if xcr0 != 0 {
        unsafe { xsetbv0(xcr0) };
    }

    unsafe {
        core::arch::asm!("fninit", options(nomem, nostack));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fpu_control_bits_match_x86_64_linux_requirements() {
        assert_eq!(CR0_MP, 1 << 1);
        assert_eq!(CR0_EM, 1 << 2);
        assert_eq!(CR0_TS, 1 << 3);
        assert_eq!(CR4_OSFXSR, 1 << 9);
        assert_eq!(CR4_OSXMMEXCPT, 1 << 10);
        assert_eq!(CR4_OSXSAVE, 1 << 18);
    }

    #[test]
    fn xcr0_mask_stays_with_fpsse_until_xsave_switching_exists() {
        let no_xsave = crate::arch::x86::kernel::cpu::CpuFeatures {
            leaf1_ecx: 0,
            leaf1_edx: 0,
            leaf7_ebx: 0,
            leaf7_ecx: 0,
            ext_edx: 0,
        };
        assert_eq!(xcr0_mask_for_features(no_xsave), 0);
        let xsave = crate::arch::x86::kernel::cpu::CpuFeatures {
            leaf1_ecx: 1 << 26,
            leaf1_edx: 0,
            leaf7_ebx: 0,
            leaf7_ecx: 0,
            ext_edx: 0,
        };
        assert_eq!(xcr0_mask_for_features(xsave), XFEATURE_MASK_FPSSE);
        let avx = crate::arch::x86::kernel::cpu::CpuFeatures {
            leaf1_ecx: (1 << 26) | (1 << 28),
            leaf1_edx: 0,
            leaf7_ebx: 0,
            leaf7_ecx: 0,
            ext_edx: 0,
        };
        assert_eq!(xcr0_mask_for_features(avx), XFEATURE_MASK_FPSSE);
    }

    #[test]
    fn fxsave_area_matches_architectural_size_and_alignment() {
        assert_eq!(core::mem::size_of::<FxSaveArea>(), FXSAVE_AREA_SIZE);
        assert_eq!(core::mem::align_of::<FxSaveArea>(), 16);
        let area = FxSaveArea::zeroed();
        assert!(area.bytes.iter().all(|b| *b == 0));
    }

    #[test]
    fn xsave_size_helpers_use_leaf_d_subleaf0_registers() {
        let leaf = crate::arch::x86::kernel::cpuid::CpuidResult {
            eax: 0,
            ebx: 832,
            ecx: 2688,
            edx: 0,
        };
        assert_eq!(xsave_enabled_size(leaf), 832);
        assert_eq!(xsave_supported_size(leaf), 2688);
    }
}
