//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/setup_percpu.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/setup_percpu.c
//! x86 per-CPU area setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/setup_percpu.c

#![allow(dead_code)]

use core::mem::offset_of;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::sched::MAX_CPUS;

pub static __PER_CPU_OFFSET: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];

pub fn register_module_exports() {
    if find_symbol("__per_cpu_offset").is_none() {
        export_symbol(
            "__per_cpu_offset",
            __PER_CPU_OFFSET.as_ptr() as usize,
            false,
        );
    }
}

/// Configured `struct softnet_data` size/alignment, verified against the
/// vendor build (`CONFIG_NR_CPUS=64`, x86-64 generic configuration).
pub const LINUX_SOFTNET_DATA_SIZE: usize = 1024;

/// Static Linux per-CPU template used by C modules.
///
/// x86 module code addresses these symbols as `%gs:symbol`. The symbol table
/// therefore exposes fields in unit zero, while `MSR_GS_BASE` contains the
/// byte delta from unit zero to the current CPU's unit. This is the same
/// `symbol + __per_cpu_offset[cpu]` contract used by Linux.
#[repr(C, align(64))]
pub struct LinuxPerCpuArea {
    cpu_number: AtomicU32,
    _cpu_number_pad: u32,
    this_cpu_off: AtomicU64,
    /// Linux's cache-hot `current_task` per-CPU variable.  The module-visible
    /// `current_task` and `const_current_task` symbols alias this same slot.
    current_task: AtomicUsize,
    /// Scratch slot used by `entry_SYSCALL_64` while RSP still points into
    /// userspace.  Linux uses `cpu_tss_rw.sp2` for the same purpose.
    syscall_user_rsp: AtomicU64,
    /// Pointer to this CPU's hardware TSS.  The syscall entry stub follows the
    /// pointer and reads RSP0 directly, so `tss::set_rsp0()` remains the single
    /// source of truth across task switches.
    syscall_tss: AtomicUsize,
    preempt_count: AtomicU32,
    /// x86 `__stack_chk_guard`, addressed by C as
    /// `%gs:__ref_stack_chk_guard` when SMP stack protection is enabled.
    stack_chk_guard: AtomicU64,
    /// Software call-depth state used by the Skylake RSB-underflow
    /// mitigation.  Linux initializes an empty call stack to the high bit and
    /// shifts it right on every thunked call / left on every return.
    x86_call_depth: AtomicU64,
    /// Fast-SRCU counters used by tracepoint-generated module code.  Keeping
    /// the pair in every Linux per-CPU unit makes the vendor sequence
    /// `mov tracepoint_srcu(%rip), %reg; incq %gs:(%reg)` valid.
    tracepoint_srcu_locks: AtomicU64,
    tracepoint_srcu_unlocks: AtomicU64,
    _header_pad: [u8; 16],
    softnet_data: [u8; LINUX_SOFTNET_DATA_SIZE],
    cpu_info: [u8; crate::arch::x86::kernel::cpu::common::LINUX_CPUINFO_X86_SIZE],
    initialized: AtomicBool,
    _tail_pad: [u8; 63],
}

impl LinuxPerCpuArea {
    const fn new() -> Self {
        Self {
            cpu_number: AtomicU32::new(0),
            _cpu_number_pad: 0,
            this_cpu_off: AtomicU64::new(0),
            current_task: AtomicUsize::new(0),
            syscall_user_rsp: AtomicU64::new(0),
            syscall_tss: AtomicUsize::new(0),
            preempt_count: AtomicU32::new(0),
            stack_chk_guard: AtomicU64::new(0),
            x86_call_depth: AtomicU64::new(crate::arch::x86::kernel::callthunks::RET_DEPTH_INIT),
            tracepoint_srcu_locks: AtomicU64::new(0),
            tracepoint_srcu_unlocks: AtomicU64::new(0),
            _header_pad: [0; 16],
            softnet_data: [0; LINUX_SOFTNET_DATA_SIZE],
            cpu_info: [0; crate::arch::x86::kernel::cpu::common::LINUX_CPUINFO_X86_SIZE],
            initialized: AtomicBool::new(false),
            _tail_pad: [0; 63],
        }
    }
}

pub(crate) static LINUX_PER_CPU_AREAS: [LinuxPerCpuArea; MAX_CPUS] =
    [const { LinuxPerCpuArea::new() }; MAX_CPUS];

/// Displacement of Linux's per-CPU stack guard from the unit-zero template.
/// `__switch_to_asm` combines this with the template symbol under `%gs`,
/// exactly like `PER_CPU_VAR(__stack_chk_guard)` in entry_64.S.
pub const STACK_CHK_GUARD_OFFSET: usize = offset_of!(LinuxPerCpuArea, stack_chk_guard);
pub const X86_CALL_DEPTH_OFFSET: usize = offset_of!(LinuxPerCpuArea, x86_call_depth);
pub const CPU_NUMBER_OFFSET: usize = offset_of!(LinuxPerCpuArea, cpu_number);
pub const PREEMPT_COUNT_OFFSET: usize = offset_of!(LinuxPerCpuArea, preempt_count);
pub const SYSCALL_USER_RSP_OFFSET: usize = offset_of!(LinuxPerCpuArea, syscall_user_rsp);
pub const SYSCALL_TSS_OFFSET: usize = offset_of!(LinuxPerCpuArea, syscall_tss);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PerCpuLayout {
    pub unit_size: u64,
    pub nr_units: usize,
    pub base: u64,
}

pub const fn pcpu_need_numa(nodes: usize) -> bool {
    nodes > 1
}

pub const fn pcpu_cpu_distance(from_node: usize, to_node: usize) -> i32 {
    if from_node == to_node { 10 } else { 20 }
}

pub fn setup_per_cpu_areas(base: u64, unit_size: u64, cpus: usize) -> Result<PerCpuLayout, i32> {
    if unit_size == 0 || cpus == 0 {
        return Err(EINVAL);
    }
    let nr = cpus.min(MAX_CPUS);
    for cpu in 0..nr {
        __PER_CPU_OFFSET[cpu].store(base + unit_size * cpu as u64, Ordering::Release);
    }
    Ok(PerCpuLayout {
        unit_size,
        nr_units: nr,
        base,
    })
}

pub fn per_cpu_offset(cpu: usize) -> u64 {
    __PER_CPU_OFFSET[cpu.min(MAX_CPUS - 1)].load(Ordering::Acquire)
}

pub fn setup_percpu_segment(cpu: usize) {
    let cpu = cpu.min(MAX_CPUS - 1);
    let base = core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0]) as usize;
    let current = core::ptr::addr_of!(LINUX_PER_CPU_AREAS[cpu]) as usize;
    let offset = current.wrapping_sub(base) as u64;
    __PER_CPU_OFFSET[cpu].store(offset, Ordering::Release);
    LINUX_PER_CPU_AREAS[cpu]
        .cpu_number
        .store(cpu as u32, Ordering::Release);
    LINUX_PER_CPU_AREAS[cpu]
        .this_cpu_off
        .store(offset, Ordering::Release);
    LINUX_PER_CPU_AREAS[cpu].syscall_tss.store(
        unsafe { crate::arch::x86::kernel::tss::tss_for_cpu(cpu) as usize },
        Ordering::Release,
    );
    LINUX_PER_CPU_AREAS[cpu]
        .preempt_count
        .store(0, Ordering::Release);
    LINUX_PER_CPU_AREAS[cpu].x86_call_depth.store(
        crate::arch::x86::kernel::callthunks::RET_DEPTH_INIT,
        Ordering::Release,
    );
    if LINUX_PER_CPU_AREAS[cpu]
        .stack_chk_guard
        .load(Ordering::Acquire)
        == 0
    {
        // Linux masks the low byte so unterminated C-string overwrites hit a
        // zero before disclosing the rest of the canary.
        let canary = crate::kernel::syscalls::next_random_u64() & 0xffff_ffff_ffff_ff00;
        LINUX_PER_CPU_AREAS[cpu]
            .stack_chk_guard
            .store(canary, Ordering::Release);
    }
    crate::arch::x86::kernel::cpu::common::write_linux_cpuinfo_x86(core::ptr::addr_of!(
        LINUX_PER_CPU_AREAS[cpu].cpu_info
    ) as *mut u8);
    initialize_softnet_data(cpu);

    #[cfg(not(test))]
    unsafe {
        crate::arch::x86::kernel::msr::write(crate::arch::x86::kernel::msr::MSR_GS_BASE, offset);
    }
    #[cfg(not(test))]
    if let Err(error) = crate::arch::x86::kernel::cet::setup_cet_cpu(cpu) {
        panic!("CET/IBT setup failed on CPU {cpu}: errno {error}");
    }
}

fn initialize_softnet_data(cpu: usize) {
    let area = &LINUX_PER_CPU_AREAS[cpu];
    if area.initialized.swap(true, Ordering::AcqRel) {
        return;
    }
    let softnet = area.softnet_data.as_ptr() as *mut u8;
    // `softnet_data.poll_list` is an empty Linux list head. Remaining fields
    // begin zeroed, matching `net_dev_init()` before backlog NAPI setup.
    unsafe {
        (softnet as *mut usize).write(softnet as usize);
        (softnet.add(core::mem::size_of::<usize>()) as *mut usize).write(softnet as usize);
        // `softnet_data.cpu` at configured offset 232.
        (softnet.add(232) as *mut u32).write(cpu as u32);
    }
}

pub fn cpu_number_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].cpu_number) as usize
}

/// Return the logical CPU number from Linux's GS-relative per-CPU area.
///
/// This is the Rust equivalent of `raw_smp_processor_id()`.  Once the per-CPU
/// segment is installed it avoids a LAPIC MMIO read (and the corresponding
/// virtual-machine exit) on scheduler, timer, and softirq hot paths.
#[cfg(not(test))]
#[inline]
pub fn current_cpu_number() -> usize {
    let cpu: u32;
    unsafe {
        core::arch::asm!(
            "mov {cpu:e}, dword ptr gs:[rip + {percpu_base} + {cpu_offset}]",
            cpu = lateout(reg) cpu,
            percpu_base = sym LINUX_PER_CPU_AREAS,
            cpu_offset = const CPU_NUMBER_OFFSET,
            options(nostack, readonly, preserves_flags),
        );
    }
    (cpu as usize).min(MAX_CPUS - 1)
}

#[cfg(test)]
#[inline]
pub const fn current_cpu_number() -> usize {
    0
}

pub fn this_cpu_off_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].this_cpu_off) as usize
}

pub fn current_task_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].current_task) as usize
}

pub fn set_current_task(cpu: usize, task: usize) {
    LINUX_PER_CPU_AREAS[cpu.min(MAX_CPUS - 1)]
        .current_task
        .store(task, Ordering::Release);
}

pub fn current_task(cpu: usize) -> usize {
    LINUX_PER_CPU_AREAS[cpu.min(MAX_CPUS - 1)]
        .current_task
        .load(Ordering::Acquire)
}

pub fn preempt_count_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].preempt_count) as usize
}

pub fn stack_chk_guard_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].stack_chk_guard) as usize
}

pub fn x86_call_depth_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].x86_call_depth) as usize
}

pub fn x86_call_depth(cpu: usize) -> u64 {
    LINUX_PER_CPU_AREAS[cpu.min(MAX_CPUS - 1)]
        .x86_call_depth
        .load(Ordering::Acquire)
}

pub fn stack_chk_guard(cpu: usize) -> u64 {
    LINUX_PER_CPU_AREAS[cpu.min(MAX_CPUS - 1)]
        .stack_chk_guard
        .load(Ordering::Acquire)
}

pub fn tracepoint_srcu_counters_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].tracepoint_srcu_locks) as usize
}

/// Install the incoming task's canary during `__switch_to()`.
///
/// This is the Rust equivalent of x86 Linux's
/// `this_cpu_write(__stack_chk_guard, next->stack_canary)`.
pub fn set_stack_chk_guard(cpu: usize, canary: u64) {
    LINUX_PER_CPU_AREAS[cpu.min(MAX_CPUS - 1)]
        .stack_chk_guard
        .store(canary & 0xffff_ffff_ffff_ff00, Ordering::Release);
}

pub fn softnet_data_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].softnet_data) as usize
}

/// Rust-side equivalent of Linux's `this_cpu_ptr(&softnet_data)`.
///
/// Callers must already pin execution to the current CPU (for example through
/// local-BH or preemption exclusion) before retaining the returned pointer.
pub(crate) fn current_softnet_data() -> *mut u8 {
    let cpu = current_cpu_number();
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[cpu].softnet_data) as *mut u8
}

pub fn cpu_info_symbol() -> usize {
    core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].cpu_info) as usize
}

pub fn preempt_count_slot(cpu: usize) -> &'static AtomicU32 {
    &LINUX_PER_CPU_AREAS[cpu.min(MAX_CPUS - 1)].preempt_count
}

pub const fn pcpu_populate_pte(_addr: u64) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_per_cpu_areas_populates_offsets() {
        let layout = setup_per_cpu_areas(0x1000, 0x2000, 3).unwrap();
        assert_eq!(layout.nr_units, 3);
        assert_eq!(per_cpu_offset(0), 0x1000);
        assert_eq!(per_cpu_offset(2), 0x5000);
        setup_percpu_segment(2);
        assert_eq!(
            LINUX_PER_CPU_AREAS[2].this_cpu_off.load(Ordering::Acquire),
            (core::mem::size_of::<LinuxPerCpuArea>() * 2) as u64
        );
        assert_eq!(LINUX_PER_CPU_AREAS[2].cpu_number.load(Ordering::Acquire), 2);
    }

    #[test]
    fn current_task_slots_are_cpu_local() {
        set_current_task(1, 0x1111);
        set_current_task(2, 0x2222);

        assert_eq!(current_task(1), 0x1111);
        assert_eq!(current_task(2), 0x2222);
        assert_eq!(
            current_task_symbol(),
            core::ptr::addr_of!(LINUX_PER_CPU_AREAS[0].current_task) as usize
        );
    }

    #[test]
    fn exports_linux_per_cpu_offset_array_symbol() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__per_cpu_offset"),
            Some(__PER_CPU_OFFSET.as_ptr() as usize)
        );
    }
}
