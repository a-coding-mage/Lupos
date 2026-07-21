//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/tools/testing/selftests/x86/xstate.c
//! x86-64 x87/SSE enablement and task context switching.
//!
//! Userland built by modern musl/Rust uses SSE instructions during C/Rust
//! runtime startup. Linux enables OSFXSR/OSXMMEXCPT before returning to user
//! mode and preserves each task's fpstate across context switches; do the same
//! before the login boot launches real PID1.
//!
//! Lupos deliberately keeps XCR0 restricted to x87/SSE. FXSAVE therefore
//! preserves every xfeature userspace can enable. AVX/YMM must remain disabled
//! until this file grows Linux's dynamically-sized XSAVE-family fpstate.
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
const X86_FEATURE_XSAVE: u32 = 4 * 32 + 26;
pub(crate) const KFPU_387: u32 = 1 << 0;
pub(crate) const KFPU_MXCSR: u32 = 1 << 1;
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
static MXCSR_FEATURE_MASK: AtomicU32 = AtomicU32::new(u32::MAX);

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

    // Do not enable the AVX/YMM xfeature yet. The task context-switch path
    // preserves the architectural 512-byte FXSAVE image, which fully covers
    // x87/SSE but not the upper halves of YMM registers. Keep XCR0 limited to
    // that baseline until dynamically-sized XSAVE/XRSTOR state is available.
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

    /// Architectural initial state used by Linux's `init_fpstate`.
    ///
    /// An all-zero image is not a valid userspace reset image: x87 FCW resets
    /// to 0x037f and MXCSR resets to 0x1f80.
    pub const fn init_state() -> Self {
        let mut bytes = [0; FXSAVE_AREA_SIZE];
        bytes[0] = 0x7f;
        bytes[1] = 0x03;
        bytes[24] = 0x80;
        bytes[25] = 0x1f;
        Self { bytes }
    }
}

impl Default for FxSaveArea {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// Task-local x87/SSE state and the small amount of Linux `struct fpu`
/// metadata needed by Lupos's eager FXSAVE/FXRSTOR implementation.
///
/// `switches` is diagnostic state for the runtime xstate selftest. It is
/// incremented whenever this task's image is saved or restored.
const TASK_FPU_STORAGE_SIZE: usize = FXSAVE_AREA_SIZE + 15;

#[repr(C)]
pub struct TaskFpuState {
    storage: [u8; TASK_FPU_STORAGE_SIZE],
    pub switches: u64,
    pub initialized: u8,
    pub runtime_test: u8,
    _pad: [u8; 6],
}

impl TaskFpuState {
    pub const fn empty() -> Self {
        Self {
            storage: [0; TASK_FPU_STORAGE_SIZE],
            switches: 0,
            initialized: 0,
            runtime_test: 0,
            _pad: [0; 6],
        }
    }

    #[inline]
    fn regs_ptr(&self) -> *const FxSaveArea {
        let start = self.storage.as_ptr();
        let offset = (16 - (start as usize & 15)) & 15;
        start.wrapping_add(offset).cast::<FxSaveArea>()
    }

    #[inline]
    fn regs_mut_ptr(&mut self) -> *mut FxSaveArea {
        let start = self.storage.as_mut_ptr();
        let offset = (16 - (start as usize & 15)) & 15;
        start.wrapping_add(offset).cast::<FxSaveArea>()
    }

    unsafe fn reset(&mut self) {
        unsafe {
            self.regs_mut_ptr().write(FxSaveArea::init_state());
        }
        self.switches = 0;
        self.initialized = 1;
        self.runtime_test = 0;
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

/// Whether the boot CPU uses Linux's XSAVE signal-frame ABI.
///
/// Lupos currently enables only x87 and SSE in XCR0, but Linux still exposes
/// those components through the 512-byte legacy area followed by the 64-byte
/// XSAVE header whenever XSAVE is enabled.
#[inline]
pub fn signal_uses_xsave() -> bool {
    crate::arch::x86::kernel::cpu::common::boot_cpu_has(X86_FEATURE_XSAVE)
}

/// MXCSR bits accepted from userspace.
///
/// Linux intersects the `mxcsr_mask` reported by every initialized CPU and
/// uses `0x0000ffbf` when the architectural FXSAVE field is zero.
#[inline]
pub fn mxcsr_feature_mask() -> u32 {
    #[cfg(test)]
    {
        0x0000_ffbf
    }
    #[cfg(not(test))]
    {
        MXCSR_FEATURE_MASK.load(Ordering::Acquire)
    }
}

#[inline]
unsafe fn task_uses_fpu(task: *mut crate::kernel::task::TaskStruct) -> bool {
    !task.is_null() && unsafe { !(*task).mm.is_null() || (*task).x86_fpu.runtime_test != 0 }
}

/// Snapshot the current task's live x87/SSE register image for a signal frame.
///
/// Linux's `copy_fpstate_to_sigframe()` saves the live register state rather
/// than trusting the task's potentially stale memory image.  Keep the task
/// image coherent as well so a later context switch cannot expose older state.
///
/// # Safety
/// The caller must be the current task and must not migrate during the save.
pub unsafe fn save_current_user_fxstate(state: &mut FxSaveArea) {
    *state = FxSaveArea::init_state();
    let task = unsafe { crate::kernel::sched::get_current() };
    if !unsafe { task_uses_fpu(task) } {
        return;
    }

    #[cfg(not(test))]
    unsafe {
        save_fxstate_raw(state as *mut FxSaveArea);
    }
    #[cfg(test)]
    unsafe {
        if (*task).x86_fpu.initialized != 0 {
            core::ptr::copy_nonoverlapping(
                (*task).x86_fpu.regs_ptr().cast::<u8>(),
                state.bytes.as_mut_ptr(),
                FXSAVE_AREA_SIZE,
            );
        }
    }

    unsafe {
        core::ptr::copy_nonoverlapping(
            state.bytes.as_ptr(),
            (*task).x86_fpu.regs_mut_ptr().cast::<u8>(),
            FXSAVE_AREA_SIZE,
        );
        (*task).x86_fpu.initialized = 1;
    }
}

/// Install a validated signal-frame x87/SSE image into the current task.
///
/// # Safety
/// The caller must be the current task and must not migrate during the
/// restore. `state` must have passed the signal-frame validation in
/// `fpu_signal`.
pub unsafe fn restore_current_user_fxstate(state: &FxSaveArea) {
    let task = unsafe { crate::kernel::sched::get_current() };
    if !unsafe { task_uses_fpu(task) } {
        return;
    }

    unsafe {
        core::ptr::copy_nonoverlapping(
            state.bytes.as_ptr(),
            (*task).x86_fpu.regs_mut_ptr().cast::<u8>(),
            FXSAVE_AREA_SIZE,
        );
        (*task).x86_fpu.initialized = 1;
        restore_fxstate_raw((*task).x86_fpu.regs_ptr());
    }
}

/// Reset the current task's user-visible FPU state after a null or invalid
/// sigreturn fpstate pointer, matching `fpu__clear_user_states()`.
///
/// # Safety
/// The caller must be the current task and must not migrate during the reset.
pub unsafe fn clear_current_user_fxstate() {
    let initial = FxSaveArea::init_state();
    unsafe { restore_current_user_fxstate(&initial) };
}

/// Save the outgoing task's live user fpstate and restore the incoming task.
///
/// This is the eager FXSAVE equivalent of Linux
/// `switch_fpu_prepare()`/`switch_fpu_finish()`. Kernel threads neither own
/// nor perturb user fpstate, matching Linux's PF_KTHREAD exclusion. Because
/// Lupos advertises only x87/SSE in XCR0, the 512-byte image covers all
/// userspace-visible state.
///
/// # Safety
/// `prev` and `next` must be the valid tasks participating in the current
/// IRQ-disabled context switch.
pub unsafe fn switch_fpu(
    prev: *mut crate::kernel::task::TaskStruct,
    next: *mut crate::kernel::task::TaskStruct,
) {
    if prev == next {
        return;
    }

    if unsafe { task_uses_fpu(prev) } {
        unsafe {
            save_fxstate_raw((*prev).x86_fpu.regs_mut_ptr());
            (*prev).x86_fpu.initialized = 1;
            if (*prev).x86_fpu.runtime_test != 0 {
                (*prev).x86_fpu.switches = (*prev).x86_fpu.switches.wrapping_add(1);
            }
        }
    }

    if unsafe { task_uses_fpu(next) } {
        unsafe {
            if (*next).x86_fpu.initialized == 0 {
                (*next).x86_fpu.reset();
            }
            restore_fxstate_raw((*next).x86_fpu.regs_ptr());
            if (*next).x86_fpu.runtime_test != 0 {
                (*next).x86_fpu.switches = (*next).x86_fpu.switches.wrapping_add(1);
            }
        }
    }
}

/// Initialize a fork child from the parent's live fpstate.
///
/// Linux's `fpu_clone()` writes the current CPU's register image directly
/// into the child because the parent's buffered image may be stale. The same
/// rule matters when `clone(2)` is entered from userspace in Lupos.
///
/// # Safety
/// `parent` and `child` must be valid; `parent` must be current for a user
/// clone, and the child must not yet be runnable.
pub unsafe fn clone_task_fpu(
    parent: *mut crate::kernel::task::TaskStruct,
    child: *mut crate::kernel::task::TaskStruct,
    kernel_thread: bool,
) {
    if child.is_null() {
        return;
    }

    unsafe {
        (*child).x86_fpu = TaskFpuState::empty();
        (*child).x86_fpu.reset();
    }
    if kernel_thread || unsafe { (*child).mm.is_null() } {
        return;
    }

    #[cfg(not(test))]
    unsafe {
        debug_assert_eq!(crate::kernel::sched::get_current(), parent);
        save_fxstate_raw((*child).x86_fpu.regs_mut_ptr());
    }
    #[cfg(test)]
    if !parent.is_null() && unsafe { (*parent).x86_fpu.initialized != 0 } {
        unsafe {
            core::ptr::copy_nonoverlapping(
                (*parent).x86_fpu.regs_ptr().cast::<u8>(),
                (*child).x86_fpu.regs_mut_ptr().cast::<u8>(),
                FXSAVE_AREA_SIZE,
            );
        }
    }
}

/// Reset the current task and live hardware to the architectural exec state.
///
/// Linux calls `fpu_flush_thread()` from `flush_thread()` while committing a
/// new image. Kernel code is compiled with the soft-float ABI, so syscall and
/// interrupt entry do not save a second FPU image which could overwrite this
/// reset on return.
///
/// # Safety
/// `task` must be the current task and must not migrate during the reset.
pub unsafe fn reset_task_fpu_for_exec(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }
    debug_assert_eq!(unsafe { crate::kernel::sched::get_current() }, task);
    unsafe {
        (*task).x86_fpu.reset();
        restore_fxstate_raw((*task).x86_fpu.regs_ptr());
    }
}

/// Result of one runtime xstate context-switch probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XstateSwitchProbe {
    pub expected: [u64; 2],
    pub observed: [u64; 2],
    pub switches_before: u64,
    pub switches_after: u64,
    pub cpu_before: u32,
    pub cpu_after: u32,
}

impl XstateSwitchProbe {
    pub fn preserved(self) -> bool {
        self.expected == self.observed && self.switches_after > self.switches_before
    }
}

#[cfg(not(test))]
unsafe extern "C" fn xstate_probe_yield() {
    unsafe {
        crate::kernel::sched::reschedule_runnable();
    }
}

/// Assembly envelope for the runtime probe.
///
/// Keeping the set/yield/read sequence in one assembly function prevents the
/// Rust ABI's caller-clobbered XMM rules from obscuring what the test is
/// checking. R12-R15 carry the pointers and pre-yield switch count across the
/// scheduler;
/// Lupos's normal switch stub already preserves those callee-saved GPRs.
#[cfg(not(test))]
#[unsafe(naked)]
unsafe extern "C" fn xstate_probe_asm(
    _expected: *const [u64; 2],
    _observed: *mut [u64; 2],
    _switches: *const u64,
) -> u64 {
    core::arch::naked_asm!(
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov r12, rdi",
        "mov r13, rsi",
        "mov r14, rdx",
        "movdqu xmm15, xmmword ptr [r12]",
        "mov r15, [r14]",
        "call {yield_fn}",
        "movdqu xmmword ptr [r13], xmm15",
        "mov rax, r15",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "ret",
        yield_fn = sym xstate_probe_yield,
    );
}

/// Exercise x87/SSE preservation across one real scheduler yield.
///
/// This adapts the core technique from Linux selftests `xstate.c`: install a
/// per-thread randomized register value, block/yield to force context
/// switches, then validate the register after resumption. A runtime harness
/// should run at least two probe kthreads on the same CPU and retry until
/// `switches_after > switches_before`; running probes on multiple CPUs also
/// covers migration.
///
/// The current task is temporarily treated as an fpstate-owning test task so
/// the helper is usable from Lupos's existing SMP kernel test mode without a
/// userspace test binary.
#[cfg(not(test))]
pub unsafe fn run_xstate_switch_probe(expected: [u64; 2]) -> XstateSwitchProbe {
    let task = unsafe { crate::kernel::sched::get_current() };
    assert!(!task.is_null(), "xstate probe requires a current task");

    let mut observed = [0u64; 2];
    let cpu_before = crate::arch::x86::kernel::setup_percpu::current_cpu_number() as u32;
    let switches_before = unsafe {
        (*task).x86_fpu.runtime_test = 1;
        let before = xstate_probe_asm(
            &expected,
            &mut observed,
            core::ptr::addr_of!((*task).x86_fpu.switches),
        );
        (*task).x86_fpu.runtime_test = 0;
        before
    };
    let switches_after = unsafe { (*task).x86_fpu.switches };
    let cpu_after = crate::arch::x86::kernel::setup_percpu::current_cpu_number() as u32;

    XstateSwitchProbe {
        expected,
        observed,
        switches_before,
        switches_after,
        cpu_before,
        cpu_after,
    }
}

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
        ldmxcsr_default();
    }

    let mut initial = FxSaveArea::zeroed();
    unsafe { save_fxstate_raw(&mut initial) };
    let mut mask = u32::from_le_bytes([
        initial.bytes[28],
        initial.bytes[29],
        initial.bytes[30],
        initial.bytes[31],
    ]);
    if mask == 0 {
        mask = 0x0000_ffbf;
    }
    MXCSR_FEATURE_MASK.fetch_and(mask, Ordering::AcqRel);
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
        let initial = FxSaveArea::init_state();
        assert_eq!(
            u16::from_le_bytes([initial.bytes[0], initial.bytes[1]]),
            0x037f
        );
        assert_eq!(
            u32::from_le_bytes([
                initial.bytes[24],
                initial.bytes[25],
                initial.bytes[26],
                initial.bytes[27],
            ]),
            MXCSR_DEFAULT
        );
        assert_eq!(core::mem::size_of::<TaskFpuState>(), 544);
        assert_eq!(core::mem::align_of::<TaskFpuState>(), 8);
        let mut task_state = TaskFpuState::empty();
        assert_eq!(task_state.regs_ptr() as usize % 16, 0);
        assert_eq!(task_state.regs_mut_ptr() as usize % 16, 0);
    }

    #[test]
    fn exec_reset_installs_architectural_initial_fpstate() {
        struct RestoreCurrent(*mut crate::kernel::task::TaskStruct);
        impl Drop for RestoreCurrent {
            fn drop(&mut self) {
                unsafe {
                    crate::kernel::sched::set_current(self.0);
                }
            }
        }

        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut task: alloc::boxed::Box<crate::kernel::task::TaskStruct> =
            alloc::boxed::Box::new(unsafe { core::mem::zeroed() });
        task.x86_fpu = TaskFpuState::empty();
        let task_ptr = &mut *task as *mut crate::kernel::task::TaskStruct;
        unsafe {
            crate::kernel::sched::set_current(task_ptr);
            reset_task_fpu_for_exec(task_ptr);
        }
        let _restore = RestoreCurrent(previous);

        let scratch = unsafe { &*task.x86_fpu.regs_ptr() };
        assert_eq!(
            u16::from_le_bytes([scratch.bytes[0], scratch.bytes[1]]),
            0x037f
        );
        assert_eq!(
            u32::from_le_bytes([
                scratch.bytes[24],
                scratch.bytes[25],
                scratch.bytes[26],
                scratch.bytes[27],
            ]),
            MXCSR_DEFAULT
        );
        assert_eq!(task.x86_fpu.initialized, 1);
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
