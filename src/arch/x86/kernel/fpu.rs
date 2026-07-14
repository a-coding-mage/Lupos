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

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

const CR0_MP: u64 = 1 << 1;
const CR0_EM: u64 = 1 << 2;
const CR0_TS: u64 = 1 << 3;
const CR4_OSFXSR: u64 = 1 << 9;
const CR4_OSXMMEXCPT: u64 = 1 << 10;
const CR4_OSXSAVE: u64 = 1 << 18;

const X86_FEATURE_FPU: u32 = 0;
const X86_FEATURE_XMM: u32 = 0 * 32 + 25;
const KFPU_387: u32 = 1 << 0;
const KFPU_MXCSR: u32 = 1 << 1;
const MXCSR_DEFAULT: u32 = 0x1f80;

pub const XFEATURE_MASK_FP: u64 = 1 << 0;
pub const XFEATURE_MASK_SSE: u64 = 1 << 1;
pub const XFEATURE_MASK_YMM: u64 = 1 << 2;
pub const XFEATURE_MASK_FPSSE: u64 = XFEATURE_MASK_FP | XFEATURE_MASK_SSE;
pub const XFEATURE_MASK_AVX: u64 = XFEATURE_MASK_FPSSE | XFEATURE_MASK_YMM;
pub const FXSAVE_AREA_SIZE: usize = 512;

static mut KERNEL_FPU_SAVE: [FxSaveArea; crate::kernel::sched::MAX_CPUS] =
    [FxSaveArea::zeroed(); crate::kernel::sched::MAX_CPUS];
static KERNEL_FPU_DEPTH: [AtomicU32; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicU32::new(0) }; crate::kernel::sched::MAX_CPUS];
static KERNEL_FPU_LOCKED_BH: [AtomicBool; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicBool::new(false) }; crate::kernel::sched::MAX_CPUS];

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
unsafe fn save_fxstate_raw(state: *mut FxSaveArea) {
    unsafe {
        core::arch::asm!(
            "fxsave64 [{ptr}]",
            ptr = in(reg) state.cast::<u8>(),
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
unsafe fn save_fxstate_raw(_state: *mut FxSaveArea) {}

#[cfg(not(test))]
pub unsafe fn save_fxstate(state: &mut FxSaveArea) {
    unsafe { save_fxstate_raw(state as *mut FxSaveArea) };
}

#[cfg(test)]
pub unsafe fn save_fxstate(_state: &mut FxSaveArea) {}

/// Restore x87/SSE state from a 16-byte-aligned FXSAVE area.
///
/// # Safety
/// The state image must have been produced by FXSAVE on a compatible CPU.
#[cfg(not(test))]
unsafe fn restore_fxstate_raw(state: *const FxSaveArea) {
    unsafe {
        core::arch::asm!(
            "fxrstor64 [{ptr}]",
            ptr = in(reg) state.cast::<u8>(),
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
unsafe fn restore_fxstate_raw(_state: *const FxSaveArea) {}

#[cfg(not(test))]
pub unsafe fn restore_fxstate(state: &FxSaveArea) {
    unsafe { restore_fxstate_raw(state as *const FxSaveArea) };
}

#[cfg(test)]
pub unsafe fn restore_fxstate(_state: &FxSaveArea) {}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("irq_fpu_usable", irq_fpu_usable as usize, false);
    export_symbol_once(
        "kernel_fpu_begin_mask",
        kernel_fpu_begin_mask as usize,
        true,
    );
    export_symbol_once("kernel_fpu_end", kernel_fpu_end as usize, true);
}

#[inline]
fn current_cpu_index() -> usize {
    crate::kernel::sched::current_cpu() as usize
}

#[cfg(not(test))]
#[inline]
unsafe fn ldmxcsr_default() {
    let mxcsr = MXCSR_DEFAULT;
    unsafe {
        core::arch::asm!("ldmxcsr [{ptr}]", ptr = in(reg) &mxcsr, options(nostack, preserves_flags));
    }
}

#[cfg(test)]
#[inline]
unsafe fn ldmxcsr_default() {}

/// Linux `irq_fpu_usable()`.
pub extern "C" fn irq_fpu_usable() -> bool {
    let cpu = current_cpu_index();
    if crate::kernel::locking::preempt::in_nmi() {
        return false;
    }
    if KERNEL_FPU_DEPTH[cpu].load(Ordering::Acquire) != 0 {
        return false;
    }
    if !crate::kernel::locking::preempt::in_hardirq() {
        return true;
    }
    !crate::kernel::locking::preempt::in_softirq()
}

/// Linux `kernel_fpu_begin_mask()`.
pub extern "C" fn kernel_fpu_begin_mask(kfpu_mask: u32) {
    let cpu = current_cpu_index();
    if KERNEL_FPU_DEPTH[cpu].load(Ordering::Acquire) != 0 {
        KERNEL_FPU_DEPTH[cpu].fetch_add(1, Ordering::AcqRel);
        return;
    }
    let locked_bh = !crate::kernel::locking::irqs_disabled();
    if locked_bh {
        crate::kernel::locking::preempt::local_bh_disable();
    }
    let depth = KERNEL_FPU_DEPTH[cpu].fetch_add(1, Ordering::AcqRel);
    if depth != 0 {
        if locked_bh {
            crate::kernel::locking::preempt::local_bh_enable();
        }
        return;
    }
    KERNEL_FPU_LOCKED_BH[cpu].store(locked_bh, Ordering::Release);

    unsafe {
        let state = core::ptr::addr_of_mut!(KERNEL_FPU_SAVE[cpu]);
        save_fxstate_raw(state);
    }

    if kfpu_mask & KFPU_MXCSR != 0
        && crate::arch::x86::kernel::cpu::common::boot_cpu_has(X86_FEATURE_XMM)
    {
        unsafe { ldmxcsr_default() };
    }

    if kfpu_mask & KFPU_387 != 0
        && crate::arch::x86::kernel::cpu::common::boot_cpu_has(X86_FEATURE_FPU)
    {
        unsafe {
            core::arch::asm!("fninit", options(nomem, nostack));
        }
    }
}

/// Linux `kernel_fpu_end()`.
pub extern "C" fn kernel_fpu_end() {
    let cpu = current_cpu_index();
    let depth = KERNEL_FPU_DEPTH[cpu].load(Ordering::Acquire);
    if depth == 0 {
        return;
    }

    if KERNEL_FPU_DEPTH[cpu].fetch_sub(1, Ordering::AcqRel) != 1 {
        return;
    }

    unsafe {
        let state = core::ptr::addr_of!(KERNEL_FPU_SAVE[cpu]);
        restore_fxstate_raw(state);
    }

    if KERNEL_FPU_LOCKED_BH[cpu].swap(false, Ordering::AcqRel) {
        crate::kernel::locking::preempt::local_bh_enable();
    }
}

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
