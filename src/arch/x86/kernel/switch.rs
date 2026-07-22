//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel
//! linux-source: vendor/linux/arch/x86/kernel/process_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86_64 context switch — `__switch_to_asm` and `__switch_to`.
//!
//! Implements the two-phase switch: callee-saved push/pop in `__switch_to_asm`,
//! then `__switch_to` doing TSS RSP0 update, per-CPU current pointer,
//! CR3/active_mm, and FS+GS base save/restore (the user GS base is round-tripped
//! through `MSR_KERNEL_GS_BASE` so involuntary preemption preserves
//! `arch_prctl(ARCH_SET_GS)` state). The switch also implements Linux's
//! user/kernel `active_mm` borrowing rules and task-local x87/SSE state.
//! It loads the incoming task's three TLS descriptors into the per-CPU GDT
//! before restoring DS/ES/FS/GS, matching Linux `process_64.c`.
//!
//! Implements M21: the two-phase context switch that is the heart of every
//! scheduler invocation.
//!
//! # Switch protocol (matches Linux exactly)
//!
//! 1. **`__switch_to_asm`** (naked assembly, `jmp`-called from `schedule()`):
//!    - Pushes the six callee-saved registers (RBP, RBX, R12–R15) of `prev`
//!      onto `prev`'s kernel stack.
//!    - Saves `prev`'s RSP into `prev->thread.sp`.
//!    - Loads `next->thread.sp` into RSP, switching to `next`'s kernel stack.
//!    - Pops the six callee-saved registers of `next`.
//!    - Tail-calls `__switch_to` (C-side housekeeping).
//!
//! 2. **`__switch_to`** (regular `extern "C"` function):
//!    - Updates `TSS.RSP0` so that the next ring-3→ring-0 transition uses
//!      `next`'s kernel stack.
//!    - Updates the per-CPU `current_task` pointer.
//!    - Returns `prev` so that `schedule()` / `finish_task_switch()` can
//!      release it (not yet implemented — future milestones).
//!
//! # References
//!   vendor/linux/arch/x86/entry/entry_64.S — `__switch_to_asm`
//!   vendor/linux/arch/x86/kernel/process_64.c — `__switch_to`
//!   vendor/linux/arch/x86/kernel/asm-offsets.c — `TASK_threadsp`
//!   vendor/linux/arch/x86/kernel/asm-offsets_64.c — x86-64 offsets

use core::mem::offset_of;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

#[cfg(any(test, debug_assertions))]
use core::sync::atomic::{AtomicI32, AtomicU64};

use crate::kernel::task::TaskStruct;
use crate::kernel::thread::ThreadStruct;

// ── TASK_THREADSP — the key offset for assembly ──────────────────────────────

/// Byte offset of `thread.sp` within `TaskStruct`.
///
/// This is the value that Linux's `asm-offsets.c` exports as `TASK_threadsp`
/// and that the `__switch_to_asm` assembly references as:
///   `movq %rsp, TASK_threadsp(%rdi)`
///
/// Computed at compile time using `offset_of!` on the actual Rust struct, so
/// it automatically stays in sync with the struct layout.
pub const TASK_THREADSP: usize = offset_of!(TaskStruct, thread) + offset_of!(ThreadStruct, sp);
pub const TASK_STACK_CANARY: usize = offset_of!(TaskStruct, stack_canary);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SwitchAttempt {
    pub sequence: u64,
    pub prev: u64,
    pub next: u64,
    pub prev_pid: i32,
    pub next_pid: i32,
    pub prev_sp: u64,
    pub next_sp: u64,
    pub prev_stack: u64,
    pub next_stack: u64,
}

// Keep the last-switch snapshot as debug-only crash instrumentation. Linux
// does not unconditionally serialize a diagnostic record into shared atomics
// on every context switch, and doing so is especially costly on SMP.
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_PREV: AtomicU64 = AtomicU64::new(0);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_NEXT: AtomicU64 = AtomicU64::new(0);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_PREV_PID: AtomicI32 = AtomicI32::new(-1);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_NEXT_PID: AtomicI32 = AtomicI32::new(-1);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_PREV_SP: AtomicU64 = AtomicU64::new(0);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_NEXT_SP: AtomicU64 = AtomicU64::new(0);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_PREV_STACK: AtomicU64 = AtomicU64::new(0);
#[cfg(any(test, debug_assertions))]
static SWITCH_ATTEMPT_NEXT_STACK: AtomicU64 = AtomicU64::new(0);

/// Lazy active_mm references released after the physical stack/CR3 switch.
///
/// Linux carries this as `rq->prev_mm` from `context_switch()` to
/// `finish_task_switch()`. Scheduler core deliberately stays architecture
/// agnostic in Lupos, so the x86 switch path carries the identical ownership
/// through one per-CPU slot.
static PREV_MM_TO_DROP: [AtomicUsize; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicUsize::new(0) }; crate::kernel::sched::MAX_CPUS];

// Linux resolves X86_BUG_NULL_SEG during CPU identification and turns the
// context-switch check into a static CPU capability branch. Lupos' current
// partial CPU-bug model approximates that flag by vendor, but must still cache
// the result: executing the serializing CPUID instruction for both FS and GS
// on every context switch is not equivalent to static_cpu_has_bug().
const NULL_SEG_BUG_UNKNOWN: u8 = 0;
const NULL_SEG_BUG_ABSENT: u8 = 1;
const NULL_SEG_BUG_PRESENT: u8 = 2;
static NULL_SEG_BUG_STATE: AtomicU8 = AtomicU8::new(NULL_SEG_BUG_UNKNOWN);

const fn null_seg_bug_for_vendor(vendor: crate::arch::x86::kernel::cpu::CpuVendor) -> bool {
    matches!(
        vendor,
        crate::arch::x86::kernel::cpu::CpuVendor::Amd
            | crate::arch::x86::kernel::cpu::CpuVendor::Hygon
    )
}

#[cfg(not(test))]
#[inline]
fn cpu_has_null_seg_bug() -> bool {
    match NULL_SEG_BUG_STATE.load(Ordering::Relaxed) {
        NULL_SEG_BUG_ABSENT => false,
        NULL_SEG_BUG_PRESENT => true,
        _ => {
            let present =
                null_seg_bug_for_vendor(crate::arch::x86::kernel::cpu::CpuVendor::current());
            NULL_SEG_BUG_STATE.store(
                if present {
                    NULL_SEG_BUG_PRESENT
                } else {
                    NULL_SEG_BUG_ABSENT
                },
                Ordering::Relaxed,
            );
            present
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MmSwitchKind {
    UserToUser,
    UserToKernel,
    KernelToUser,
    KernelToKernel,
}

fn classify_mm_switch(
    prev_mm: *mut crate::mm::mm_types::MmStruct,
    next_mm: *mut crate::mm::mm_types::MmStruct,
) -> MmSwitchKind {
    match (prev_mm.is_null(), next_mm.is_null()) {
        (false, false) => MmSwitchKind::UserToUser,
        (false, true) => MmSwitchKind::UserToKernel,
        (true, false) => MmSwitchKind::KernelToUser,
        (true, true) => MmSwitchKind::KernelToKernel,
    }
}

#[inline]
#[cfg(any(test, debug_assertions))]
unsafe fn task_pid(task: *mut TaskStruct) -> i32 {
    if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    }
}

#[inline]
#[cfg(any(test, debug_assertions))]
unsafe fn task_sp(task: *mut TaskStruct) -> u64 {
    if task.is_null() {
        0
    } else {
        unsafe { (*task).thread.sp }
    }
}

#[inline]
#[cfg(any(test, debug_assertions))]
unsafe fn task_stack(task: *mut TaskStruct) -> u64 {
    if task.is_null() {
        0
    } else {
        unsafe { (*task).stack as u64 }
    }
}

#[cfg(any(test, debug_assertions))]
pub unsafe fn record_switch_attempt(prev: *mut TaskStruct, next: *mut TaskStruct) {
    SWITCH_ATTEMPT_PREV.store(prev as u64, Ordering::Relaxed);
    SWITCH_ATTEMPT_NEXT.store(next as u64, Ordering::Relaxed);
    SWITCH_ATTEMPT_PREV_PID.store(unsafe { task_pid(prev) }, Ordering::Relaxed);
    SWITCH_ATTEMPT_NEXT_PID.store(unsafe { task_pid(next) }, Ordering::Relaxed);
    SWITCH_ATTEMPT_PREV_SP.store(unsafe { task_sp(prev) }, Ordering::Relaxed);
    SWITCH_ATTEMPT_NEXT_SP.store(unsafe { task_sp(next) }, Ordering::Relaxed);
    SWITCH_ATTEMPT_PREV_STACK.store(unsafe { task_stack(prev) }, Ordering::Relaxed);
    SWITCH_ATTEMPT_NEXT_STACK.store(unsafe { task_stack(next) }, Ordering::Relaxed);
    SWITCH_ATTEMPT_SEQUENCE.fetch_add(1, Ordering::Release);
}

#[cfg(not(any(test, debug_assertions)))]
#[inline(always)]
pub unsafe fn record_switch_attempt(_prev: *mut TaskStruct, _next: *mut TaskStruct) {}

#[cfg(any(test, debug_assertions))]
pub fn last_switch_attempt() -> SwitchAttempt {
    SwitchAttempt {
        sequence: SWITCH_ATTEMPT_SEQUENCE.load(Ordering::Acquire),
        prev: SWITCH_ATTEMPT_PREV.load(Ordering::Relaxed),
        next: SWITCH_ATTEMPT_NEXT.load(Ordering::Relaxed),
        prev_pid: SWITCH_ATTEMPT_PREV_PID.load(Ordering::Relaxed),
        next_pid: SWITCH_ATTEMPT_NEXT_PID.load(Ordering::Relaxed),
        prev_sp: SWITCH_ATTEMPT_PREV_SP.load(Ordering::Relaxed),
        next_sp: SWITCH_ATTEMPT_NEXT_SP.load(Ordering::Relaxed),
        prev_stack: SWITCH_ATTEMPT_PREV_STACK.load(Ordering::Relaxed),
        next_stack: SWITCH_ATTEMPT_NEXT_STACK.load(Ordering::Relaxed),
    }
}

#[cfg(not(any(test, debug_assertions)))]
#[inline(always)]
pub fn last_switch_attempt() -> SwitchAttempt {
    SwitchAttempt {
        sequence: 0,
        prev: 0,
        next: 0,
        prev_pid: -1,
        next_pid: -1,
        prev_sp: 0,
        next_sp: 0,
        prev_stack: 0,
        next_stack: 0,
    }
}

/// Make the incoming task's kernel stack visible before `__switch_to_asm`
/// loads it into RSP.
///
/// Linux keeps kernel mappings globally shared. Lupos has transitional process
/// PGDs, so vmalloc-backed stacks must have their PGD slot copied into the
/// currently active mm before the assembly stub touches `next->thread.sp`.
#[cfg(not(test))]
pub unsafe fn prepare_switch_to_task(next: *mut TaskStruct) {
    if next.is_null() {
        return;
    }
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        return;
    }
    unsafe {
        prepare_switch_stack_canaries(current, next);
    }

    // The incoming stack must exist in the currently loaded PGD until the
    // assembly stub changes RSP. A subsequent user-mm switch synchronizes the
    // complete vmalloc slot into the incoming PGD before loading CR3.
    let stack_top = unsafe { (*next).stack as u64 };
    if let Some(stack_bottom) =
        stack_top.checked_sub(crate::kernel::sched::KTHREAD_STACK_SIZE as u64)
        && crate::mm::vmalloc::is_vmalloc_addr(stack_bottom as *const u8)
    {
        let mm = unsafe { task_active_mm(current) };
        if !mm.is_null() {
            unsafe {
                crate::mm::vmalloc::sync_vmalloc_pgd_slot_to_mm(
                    mm,
                    stack_bottom,
                    crate::kernel::sched::KTHREAD_STACK_SIZE,
                );
            }
        }
    }

    // Linux performs active_mm/CR3 switching in context_switch(), before
    // `switch_to()` changes the kernel stack.
    unsafe {
        prepare_switch_mm(current, next);
    }
}

#[cfg(test)]
pub unsafe fn prepare_switch_to_task(next: *mut TaskStruct) {
    if !next.is_null() {
        unsafe {
            prepare_switch_stack_canaries(core::ptr::null_mut(), next);
        }
    }
}

#[cfg(not(test))]
unsafe fn task_active_mm(task: *mut TaskStruct) -> *mut crate::mm::mm_types::MmStruct {
    if task.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        }
    }
}

#[cfg(not(test))]
unsafe fn publish_mm_drop(cpu: usize, mm: *mut crate::mm::mm_types::MmStruct) {
    if mm.is_null() {
        return;
    }
    let installed =
        PREV_MM_TO_DROP[cpu].compare_exchange(0, mm as usize, Ordering::AcqRel, Ordering::Acquire);
    assert!(
        installed.is_ok(),
        "context switch already carries an undropped active_mm"
    );
}

#[cfg(not(test))]
unsafe fn load_user_mm(cpu: usize, mm: *mut crate::mm::mm_types::MmStruct) {
    assert!(!mm.is_null(), "load_user_mm requires an mm");

    // Linux keys the overwhelmingly common same-mm path off the per-CPU
    // loaded_mm tracker. It does not read CR3 in production: tlb.c explicitly
    // keeps that expensive check under CONFIG_DEBUG_VM. With no-op lazy TLB,
    // this also preserves a borrowed mm without a needless CR3 reload when a
    // user task resumes after a kernel thread.
    if crate::arch::x86::mm::tlb::loaded_mm_matches(cpu as u32, mm) {
        crate::arch::x86::mm::tlb::reactivate_lazy_tlb(cpu as u32, mm);
        return;
    }

    let pgd = unsafe { (*mm).pgd as u64 };
    let pgd_phys = crate::arch::x86::mm::paging::direct_map_virt_to_phys(pgd)
        .filter(|phys| phys & (crate::arch::x86::mm::paging::PAGE_SIZE - 1) == 0)
        .unwrap_or_else(|| panic!("load_user_mm: mm PGD is outside the aligned direct map"));
    unsafe {
        crate::mm::vmalloc::sync_vmalloc_to_mm(mm);
    }

    unsafe {
        // Linux publishes LOADED_MM_SWITCHING before changing CR3 so a
        // concurrent shootdown conservatively targets this CPU regardless
        // of whether the old or new address space is currently loaded.
        crate::arch::x86::mm::tlb::set_active_mm_switching(cpu as u32);
        core::sync::atomic::fence(Ordering::SeqCst);
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) pgd_phys,
            options(nostack, preserves_flags)
        );
        crate::arch::x86::mm::tlb::set_active_mm(cpu as u32, mm);
    }
}

/// Linux `context_switch()` active_mm transitions.
///
/// - user -> kernel: borrow and take one lazy `mm_count` reference;
/// - kernel -> kernel: transfer that reference;
/// - kernel -> user: load user CR3 and defer the lazy drop until the new stack;
/// - user -> user: load the incoming user CR3 with no lazy refcount traffic.
#[cfg(not(test))]
unsafe fn prepare_switch_mm(prev: *mut TaskStruct, next: *mut TaskStruct) {
    let prev_mm = unsafe { (*prev).mm };
    let next_mm = unsafe { (*next).mm };
    let cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number();

    match classify_mm_switch(prev_mm, next_mm) {
        MmSwitchKind::UserToKernel => {
            let borrowed = unsafe { task_active_mm(prev) };
            debug_assert!(
                unsafe { (*next).active_mm.is_null() },
                "non-current kernel task retained active_mm"
            );
            unsafe {
                (*next).active_mm = borrowed;
                if !borrowed.is_null() {
                    (*borrowed).mmdrop_get();
                }
                crate::arch::x86::mm::tlb::enter_lazy_tlb(cpu as u32, borrowed);
            }
        }
        MmSwitchKind::KernelToKernel => {
            let borrowed = unsafe { task_active_mm(prev) };
            debug_assert!(
                unsafe { (*next).active_mm.is_null() },
                "non-current kernel task retained active_mm"
            );
            unsafe {
                (*next).active_mm = borrowed;
                (*prev).active_mm = core::ptr::null_mut();
                crate::arch::x86::mm::tlb::enter_lazy_tlb(cpu as u32, borrowed);
            }
        }
        MmSwitchKind::KernelToUser => {
            let borrowed = unsafe { task_active_mm(prev) };
            unsafe {
                (*next).active_mm = next_mm;
                load_user_mm(cpu, next_mm);
                (*prev).active_mm = core::ptr::null_mut();
                publish_mm_drop(cpu, borrowed);
            }
        }
        MmSwitchKind::UserToUser => unsafe {
            (*next).active_mm = next_mm;
            load_user_mm(cpu, next_mm);
        },
    }
}

#[cfg(not(test))]
unsafe fn finish_switch_mm_drop() {
    let cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number();
    let mm = PREV_MM_TO_DROP[cpu].swap(0, Ordering::AcqRel) as *mut crate::mm::mm_types::MmStruct;
    if !mm.is_null() {
        unsafe {
            crate::mm::fork::mmdrop(mm);
        }
    }
}

#[cfg(test)]
unsafe fn finish_switch_mm_drop() {}

fn fresh_stack_canary() -> u64 {
    let canary = crate::kernel::syscalls::next_random_u64() & 0xffff_ffff_ffff_ff00;
    if canary == 0 {
        0x6c75_706f_735f_7300
    } else {
        canary
    }
}

unsafe fn prepare_switch_stack_canaries(prev: *mut TaskStruct, next: *mut TaskStruct) {
    if !prev.is_null() && unsafe { (*prev).stack_canary } == 0 {
        let current_guard = crate::arch::x86::kernel::setup_percpu::stack_chk_guard(
            crate::kernel::sched::current_cpu() as usize,
        );
        unsafe {
            (*prev).stack_canary = if current_guard == 0 {
                fresh_stack_canary()
            } else {
                current_guard
            };
        }
    }
    if !next.is_null() && unsafe { (*next).stack_canary } == 0 {
        unsafe {
            (*next).stack_canary = fresh_stack_canary();
        }
    }
}

// ── Compile-time sanity check ────────────────────────────────────────────────

const _: () = {
    assert!(TASK_THREADSP > 0, "TASK_THREADSP must not be zero");
    // The offset must be expressible as a 32-bit displacement for AT&T memory
    // operands (mov [reg + disp32], ...); values ≥ 2^31 would overflow.
    assert!(
        TASK_THREADSP < (1 << 31),
        "TASK_THREADSP too large for 32-bit displacement"
    );
    assert!(TASK_STACK_CANARY < (1 << 31));
};

// ── __switch_to_asm ──────────────────────────────────────────────────────────

/// Assembly context-switch stub.
///
/// Saves the outgoing task's callee-saved registers, swaps the kernel stack
/// pointer, restores the incoming task's callee-saved registers, and
/// tail-calls `__switch_to` for C-side housekeeping.
///
/// # ABI (matches Linux `__switch_to_asm`)
///
/// - **RDI** = `prev: *mut TaskStruct`
/// - **RSI** = `next: *mut TaskStruct`
/// - **Return** (in RAX, after `__switch_to` returns) = `prev`
///
/// The function is `#[unsafe(naked)]` — the compiler generates no prologue,
/// epilogue, or register-save code of its own.
///
/// # Initial call for a brand-new thread
///
/// The first time `next` is scheduled, `next->thread.sp` points to an
/// artificially constructed stack frame set up by `kthread_create()`.
/// After `__switch_to_asm` pops the six callee-saved slots and `__switch_to`
/// returns, execution continues at the return address baked into that initial
/// frame — which is `kthread_entry_stub()`.
///
/// # Safety
/// Must be called with both `prev` and `next` pointing to valid, fully-
/// initialised `TaskStruct`s whose kernel stacks have the correct layout.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __switch_to_asm(
    _prev: *mut TaskStruct,
    _next: *mut TaskStruct,
) -> *mut TaskStruct {
    // Naked function: no Rust prologue. Arguments arrive in RDI (_prev) and
    // RSI (_next) per the System V AMD64 ABI — referenced directly in asm.
    // SAFETY: Naked function — entered directly by schedule(); no Rust prologue.
    //
    // Register state on entry:
    //   RDI = prev  (first arg)
    //   RSI = next  (second arg)
    //
    // Stack layout after the six pushes (grows toward lower addresses):
    //   [RSP+40]  old RBP  ← top of frame
    //   [RSP+32]  old RBX
    //   [RSP+24]  old R12
    //   [RSP+16]  old R13
    //   [RSP+8]   old R14
    //   [RSP+0]   old R15  ← RSP points here; saved into prev->thread.sp
    //
    // After switching RSP and popping, we jmp to __switch_to (not call,
    // because we want __switch_to's `ret` to act as our `ret`, keeping
    // the call-depth correct for stack unwinders).
    // SAFETY: naked_asm! only allows const and sym operands (no in/out regs).
    // RDI = prev and RSI = next are already set by the System V calling
    // convention; we reference them directly by name in the assembly text.
    core::arch::naked_asm!(
        // Save callee-saved registers of `prev` onto its kernel stack.
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // prev->thread.sp = rsp
        // {sp} is the compile-time constant TASK_THREADSP (byte offset of
        // thread.sp within task_struct), substituted by the assembler as an
        // integer literal — equivalent to Linux's `TASK_threadsp` from
        // `asm-offsets.h`.
        // RDI holds `prev` (first argument, per SysV ABI).
        "mov [rdi + {sp}], rsp",

        // rsp = next->thread.sp
        // RSI holds `next` (second argument, per SysV ABI).
        "mov rsp, [rsi + {sp}]",

        // Linux entry_64.S updates the per-CPU guard after switching stacks
        // and before entering compiled code. RBX is scratch here because the
        // incoming value is restored from the new stack immediately below;
        // RAX is caller-clobbered and becomes __switch_to's return value.
        "mov rbx, [rsi + {stack_canary}]",
        "lea rax, [rip + {percpu_base}]",
        "mov qword ptr gs:[rax + {guard_offset}], rbx",

        // Restore callee-saved registers of `next` from its kernel stack.
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",

        // Tail-call __switch_to(prev=RDI, next=RSI).
        // Using jmp preserves the call-depth: __switch_to's `ret` will
        // return directly to our caller (schedule()), with RAX = prev.
        // RDI and RSI are still live here (we only touched RSP and the
        // callee-saved GPRs above).
        "jmp {switch_to}",

        sp = const TASK_THREADSP,
        stack_canary = const TASK_STACK_CANARY,
        percpu_base = sym crate::arch::x86::kernel::setup_percpu::LINUX_PER_CPU_AREAS,
        guard_offset = const crate::arch::x86::kernel::setup_percpu::STACK_CHK_GUARD_OFFSET,
        switch_to = sym __switch_to,
    );
}

// ── __switch_to ──────────────────────────────────────────────────────────────

/// C-side context-switch housekeeping.
///
/// Called via tail-jump from `__switch_to_asm` with RDI=prev, RSI=next.
/// Performs the operations that require C/Rust code (TSS update, per-CPU
/// current pointer).
///
/// # Returns
/// `prev` — returned to `schedule()` via RAX so that `finish_task_switch()`
/// (future milestone) can release reference counts.
///
/// # Safety
/// Must be called immediately after `__switch_to_asm` with the kernel stack
/// already switched to `next`'s stack.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __switch_to(
    prev: *mut TaskStruct,
    next: *mut TaskStruct,
) -> *mut TaskStruct {
    // Linux process_64.c starts __switch_to() with switch_fpu(). The outgoing
    // hardware image is still live even though __switch_to_asm already changed
    // the kernel stack.
    unsafe {
        crate::arch::x86::kernel::fpu::switch_fpu(prev, next);
    }

    // Linux saves FS/GS before loading the next task's TLS/segment state:
    // vendor/linux/arch/x86/kernel/process_64.c `save_fsgs()` and
    // `x86_fsgsbase_load()`. Lupos currently uses FS base for libc TLS, so
    // preserving it across involuntary context switches is mandatory.
    unsafe {
        save_fsgs(prev);
    }

    // Linux must install next->thread.tls_array before any segment selector is
    // restored, because selectors 0x63/0x6b/0x73 resolve through these slots.
    let cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number();
    unsafe {
        crate::arch::x86::kernel::gdt::load_tls(&(*next).thread, cpu);
        load_fsgs(prev, next);
    }

    // 1. Update TSS.RSP0 to the top of next's kernel stack so that the next
    //    ring-3 → ring-0 transition (syscall or interrupt from user) loads
    //    next's kernel stack pointer instead of prev's.
    //
    //    `next->stack` points to the top of the task's kernel stack page
    //    (one byte past the highest usable address, since x86 stacks grow down).
    //
    //    Ref: Linux process_64.c `update_task_stack()`
    let stack_top = unsafe { (*next).stack } as u64;
    assert!(
        stack_top != 0,
        "__switch_to: next task has null kernel stack"
    );
    unsafe {
        crate::arch::x86::kernel::tss::set_rsp0(stack_top);
    }

    // 2. Update the per-CPU current_task pointer so that `get_current()`
    //    returns `next` from this point forward on this CPU.
    //
    //    Ref: Linux process_64.c `raw_cpu_write(current_task, next_p)`
    unsafe {
        crate::kernel::sched::set_current(next);
        finish_switch_mm_drop();
    }

    // Return `prev` so the caller knows which task just stopped running.
    prev
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(not(test))]
unsafe fn save_fsgs(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }

    unsafe {
        let fsindex = crate::arch::x86::kernel::gdt::read_fs();
        let gsindex = crate::arch::x86::kernel::gdt::read_gs();
        (*task).thread.fsindex = fsindex;
        (*task).thread.gsindex = gsindex;

        // Lupos does not enable CR4.FSGSBASE. Match Linux save_base_legacy():
        // selector zero is the overwhelmingly common 64-bit TLS case and its
        // task-thread base is already authoritative, so avoid two RDMSRs on
        // every switch. For every nonzero selector Linux's save_base_legacy()
        // records zero: selectors 1..=3 have base zero and selectors >3 get
        // their base from the descriptor that load_TLS() installs.
        if fsindex != 0 {
            (*task).thread.fsbase = 0;
        }
        if gsindex != 0 {
            (*task).thread.gsbase = 0;
        }
    }
}

#[cfg(test)]
unsafe fn save_fsgs(_task: *mut TaskStruct) {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LegacyLoadPlan {
    None,
    Selector,
    Base,
    SelectorAndBase,
}

const fn selector_after_tls_load(
    selector: u16,
    tls: [crate::kernel::thread::DescStruct; 3],
) -> u16 {
    // A GDT selector has TI clear. Linux's exception-table segment loaders
    // turn a selector whose newly installed descriptor is empty into zero.
    if selector & 0x4 == 0 {
        let index = (selector >> 3) as usize;
        if index >= crate::arch::x86::kernel::gdt::GDT_ENTRY_TLS_MIN
            && index <= crate::arch::x86::kernel::gdt::GDT_ENTRY_TLS_MAX
            && tls[index - crate::arch::x86::kernel::gdt::GDT_ENTRY_TLS_MIN].0 == 0
        {
            return 0;
        }
    }
    selector
}

const fn legacy_load_plan(
    prev_index: u16,
    prev_base: u64,
    next_index: u16,
    next_base: u64,
) -> LegacyLoadPlan {
    if next_index <= 3 {
        if next_base == 0 {
            if prev_index != 0 || next_index != 0 || prev_base != 0 {
                LegacyLoadPlan::Selector
            } else {
                LegacyLoadPlan::None
            }
        } else if prev_index != next_index {
            LegacyLoadPlan::SelectorAndBase
        } else {
            LegacyLoadPlan::Base
        }
    } else {
        LegacyLoadPlan::Selector
    }
}

const fn null_seg_bug_requires_two_loads(
    next_index: u16,
    next_base: u64,
    null_seg_bug: bool,
) -> bool {
    null_seg_bug && next_index <= 3 && next_base == 0
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
enum FsgsSelector {
    Fs,
    Gs,
}

#[cfg(not(test))]
unsafe fn load_user_selector(which: FsgsSelector, selector: u16) {
    unsafe {
        match which {
            FsgsSelector::Fs => crate::arch::x86::kernel::gdt::load_fs(selector),
            FsgsSelector::Gs => crate::arch::x86::kernel::gdt::load_gs_index(selector),
        }
    }
}

#[cfg(not(test))]
unsafe fn load_seg_legacy(
    prev_index: u16,
    prev_base: u64,
    next_index: u16,
    next_base: u64,
    null_seg_bug: bool,
    which: FsgsSelector,
) {
    // Linux uses X86_BUG_NULL_SEG to decide whether a null selector needs a
    // USER_DS intermediate load. Lupos does not yet carry bug bits, so apply
    // the architectural workaround to AMD/Hygon CPUs. Linux performs both
    // loads even when all saved values are zero; that is the case where the
    // workaround is most important because a stale hidden base may remain.
    if null_seg_bug_requires_two_loads(next_index, next_base, null_seg_bug) {
        unsafe {
            load_user_selector(which, crate::arch::x86::kernel::gdt::sel::USER_DS);
            load_user_selector(which, next_index);
        }
        return;
    }

    let plan = legacy_load_plan(prev_index, prev_base, next_index, next_base);
    if matches!(
        plan,
        LegacyLoadPlan::Selector | LegacyLoadPlan::SelectorAndBase
    ) {
        unsafe {
            load_user_selector(which, next_index);
        }
    }
    if matches!(plan, LegacyLoadPlan::Base | LegacyLoadPlan::SelectorAndBase) {
        let msr = match which {
            FsgsSelector::Fs => crate::arch::x86::kernel::msr::MSR_FS_BASE,
            FsgsSelector::Gs => crate::arch::x86::kernel::msr::MSR_KERNEL_GS_BASE,
        };
        unsafe {
            crate::arch::x86::kernel::msr::write(msr, next_base);
        }
    }
}

/// Snapshot the current task's live user FS/GS bases before copying thread
/// state.  With FSGSBASE enabled userspace may change either base directly, so
/// `task.thread` is not necessarily current until Linux's `current_save_fsgs()`
/// equivalent runs.
///
/// Ref: Linux `arch/x86/kernel/process_64.c::current_save_fsgs()`.
pub(crate) unsafe fn current_save_fsgs() {
    let task = crate::kernel::sched::get_current();
    if task.is_null() {
        return;
    }
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    unsafe {
        save_fsgs(task);
    }
    crate::kernel::locking::irqflags::local_irq_restore(flags);
}

#[cfg(not(test))]
unsafe fn load_fsgs(prev: *mut TaskStruct, next: *mut TaskStruct) {
    if prev.is_null() || next.is_null() {
        return;
    }

    unsafe {
        let next_tls = (*next).thread.tls_array;
        let prev_es = crate::arch::x86::kernel::gdt::read_es();
        (*prev).thread.es = prev_es;
        let next_es = selector_after_tls_load((*next).thread.es, next_tls);
        if next_es != 0 || prev_es != 0 {
            crate::arch::x86::kernel::gdt::load_es(next_es);
        }
        let prev_ds = crate::arch::x86::kernel::gdt::read_ds();
        (*prev).thread.ds = prev_ds;
        let next_ds = selector_after_tls_load((*next).thread.ds, next_tls);
        if next_ds != 0 || prev_ds != 0 {
            crate::arch::x86::kernel::gdt::load_ds(next_ds);
        }

        let saved_fsindex = (*next).thread.fsindex;
        let next_fsindex = selector_after_tls_load(saved_fsindex, next_tls);
        let next_fsbase = if next_fsindex == saved_fsindex {
            (*next).thread.fsbase
        } else {
            0
        };
        let null_seg_bug = cpu_has_null_seg_bug();
        load_seg_legacy(
            (*prev).thread.fsindex,
            (*prev).thread.fsbase,
            next_fsindex,
            next_fsbase,
            null_seg_bug,
            FsgsSelector::Fs,
        );
        let saved_gsindex = (*next).thread.gsindex;
        let next_gsindex = selector_after_tls_load(saved_gsindex, next_tls);
        let next_gsbase = if next_gsindex == saved_gsindex {
            (*next).thread.gsbase
        } else {
            0
        };
        load_seg_legacy(
            (*prev).thread.gsindex,
            (*prev).thread.gsbase,
            next_gsindex,
            next_gsbase,
            null_seg_bug,
            FsgsSelector::Gs,
        );
    }
}

#[cfg(test)]
unsafe fn load_fsgs(_prev: *mut TaskStruct, _next: *mut TaskStruct) {}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn task_threadsp_is_nonzero() {
        assert!(TASK_THREADSP > 0);
    }

    #[test]
    fn task_threadsp_fits_in_32_bit_displacement() {
        // AT&T syntax memory operands use 32-bit sign-extended displacements.
        assert!(TASK_THREADSP < (1u64 << 31) as usize);
    }

    #[test]
    fn task_threadsp_matches_expected_offset() {
        // thread starts at LINUX_OFFSET_THREAD, sp is at +24 within ThreadStruct.
        let expected = crate::kernel::task::LINUX_OFFSET_THREAD + offset_of!(ThreadStruct, sp);
        assert_eq!(TASK_THREADSP, expected);
    }

    #[test]
    fn kthread_stack_frame_size_is_56() {
        // __switch_to_asm pushes 6 callee-saved regs (48 bytes).
        // kthread_create prepends 1 return-address slot (8 bytes) above those.
        // Total initial frame: 56 bytes.
        let callee_saved_regs = 6usize;
        let return_addr_slot = 1usize;
        let frame_size = (callee_saved_regs + return_addr_slot) * 8;
        assert_eq!(frame_size, 56);
    }

    #[test]
    fn legacy_fsgs_load_plan_matches_linux_selector_base_ordering() {
        // Origin: vendor/linux/arch/x86/kernel/process_64.c
        // `load_seg_legacy`.
        assert_eq!(legacy_load_plan(0, 0, 0, 0), LegacyLoadPlan::None);
        assert_eq!(legacy_load_plan(0x63, 0, 0, 0), LegacyLoadPlan::Selector);
        assert_eq!(legacy_load_plan(0, 0, 0, 0x1234), LegacyLoadPlan::Base);
        assert_eq!(
            legacy_load_plan(0x63, 0, 0, 0x1234),
            LegacyLoadPlan::SelectorAndBase
        );
        assert_eq!(legacy_load_plan(0, 0x1234, 0, 0), LegacyLoadPlan::Selector);
        assert_eq!(legacy_load_plan(0, 0, 0x6b, 0), LegacyLoadPlan::Selector);
        assert!(
            null_seg_bug_requires_two_loads(0, 0, true),
            "Linux's AMD null-segment workaround must not skip the all-zero case"
        );
        assert!(!null_seg_bug_requires_two_loads(0, 0, false));
        assert!(!null_seg_bug_for_vendor(
            crate::arch::x86::kernel::cpu::CpuVendor::Intel
        ));
        assert!(null_seg_bug_for_vendor(
            crate::arch::x86::kernel::cpu::CpuVendor::Amd
        ));
        assert!(null_seg_bug_for_vendor(
            crate::arch::x86::kernel::cpu::CpuVendor::Hygon
        ));

        let empty = [crate::kernel::thread::DescStruct(0); 3];
        let populated = [
            crate::kernel::thread::DescStruct(1),
            crate::kernel::thread::DescStruct(2),
            crate::kernel::thread::DescStruct(3),
        ];
        assert_eq!(selector_after_tls_load(0x63, empty), 0);
        assert_eq!(selector_after_tls_load(0x63, populated), 0x63);
        assert_eq!(selector_after_tls_load(0x7, empty), 0x7);
    }

    #[test]
    fn active_mm_transition_classifies_all_linux_context_switch_cases() {
        let user_a = 0x1000usize as *mut crate::mm::mm_types::MmStruct;
        let user_b = 0x2000usize as *mut crate::mm::mm_types::MmStruct;
        let kernel = core::ptr::null_mut();

        assert_eq!(classify_mm_switch(user_a, user_b), MmSwitchKind::UserToUser);
        assert_eq!(
            classify_mm_switch(user_a, kernel),
            MmSwitchKind::UserToKernel
        );
        assert_eq!(
            classify_mm_switch(kernel, user_b),
            MmSwitchKind::KernelToUser
        );
        assert_eq!(
            classify_mm_switch(kernel, kernel),
            MmSwitchKind::KernelToKernel
        );
    }

    #[test]
    fn incoming_task_gets_a_stable_masked_stack_canary() {
        let mut task: Box<TaskStruct> = Box::new(unsafe { core::mem::zeroed() });
        unsafe { prepare_switch_to_task(&mut *task) };
        let canary = task.stack_canary;
        assert_ne!(canary, 0);
        assert_eq!(canary & 0xff, 0);
        unsafe { prepare_switch_to_task(&mut *task) };
        assert_eq!(task.stack_canary, canary);
    }
}
