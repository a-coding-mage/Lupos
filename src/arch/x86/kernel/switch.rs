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
//! `arch_prctl(ARCH_SET_GS)` state). Remaining work vs Linux `process_64.c` for
//! `complete`: `load_TLS(next)` (needs three TLS GDT slots added to the GDT),
//! and FPU/xstate switch (needs a per-task xstate save area); both are
//! boot-ABI-sensitive and tracked separately.
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
use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

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

static SWITCH_ATTEMPT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static SWITCH_ATTEMPT_PREV: AtomicU64 = AtomicU64::new(0);
static SWITCH_ATTEMPT_NEXT: AtomicU64 = AtomicU64::new(0);
static SWITCH_ATTEMPT_PREV_PID: AtomicI32 = AtomicI32::new(-1);
static SWITCH_ATTEMPT_NEXT_PID: AtomicI32 = AtomicI32::new(-1);
static SWITCH_ATTEMPT_PREV_SP: AtomicU64 = AtomicU64::new(0);
static SWITCH_ATTEMPT_NEXT_SP: AtomicU64 = AtomicU64::new(0);
static SWITCH_ATTEMPT_PREV_STACK: AtomicU64 = AtomicU64::new(0);
static SWITCH_ATTEMPT_NEXT_STACK: AtomicU64 = AtomicU64::new(0);

#[inline]
unsafe fn task_pid(task: *mut TaskStruct) -> i32 {
    if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    }
}

#[inline]
unsafe fn task_sp(task: *mut TaskStruct) -> u64 {
    if task.is_null() {
        0
    } else {
        unsafe { (*task).thread.sp }
    }
}

#[inline]
unsafe fn task_stack(task: *mut TaskStruct) -> u64 {
    if task.is_null() {
        0
    } else {
        unsafe { (*task).stack as u64 }
    }
}

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
    unsafe {
        prepare_switch_stack_canaries(crate::kernel::sched::get_current(), next);
    }
    let stack_top = unsafe { (*next).stack as u64 };
    if stack_top == 0 {
        return;
    }
    let Some(stack_bottom) = stack_top.checked_sub(crate::kernel::sched::KTHREAD_STACK_SIZE as u64)
    else {
        return;
    };
    if !crate::mm::vmalloc::is_vmalloc_addr(stack_bottom as *const u8) {
        return;
    }

    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        return;
    }
    let mm = unsafe {
        if !(*current).mm.is_null() {
            (*current).mm
        } else {
            (*current).active_mm
        }
    };
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

#[cfg(test)]
pub unsafe fn prepare_switch_to_task(next: *mut TaskStruct) {
    if !next.is_null() {
        unsafe {
            prepare_switch_stack_canaries(core::ptr::null_mut(), next);
        }
    }
}

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
/// # TODO (M22)
/// - `load_TLS(next)`: write next's three TLS GDT entries into the GDT.
/// - GS base and selector switching for nontrivial GS users.
/// - Stack-protector canary update.
/// - FPU state switch.
///
/// # Safety
/// Must be called immediately after `__switch_to_asm` with the kernel stack
/// already switched to `next`'s stack.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __switch_to(
    prev: *mut TaskStruct,
    next: *mut TaskStruct,
) -> *mut TaskStruct {
    // Linux saves FS/GS before loading the next task's TLS/segment state:
    // vendor/linux/arch/x86/kernel/process_64.c `save_fsgs()` and
    // `x86_fsgsbase_load()`. Lupos currently uses FS base for libc TLS, so
    // preserving it across involuntary context switches is mandatory.
    unsafe {
        save_fsgs(prev);
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
    unsafe {
        load_task_cr3(next);
    }
    unsafe {
        load_fsgs(next);
    }
    unsafe {
        let mm = if !(*next).mm.is_null() {
            (*next).mm
        } else {
            (*next).active_mm
        };
        crate::arch::x86::mm::tlb::set_active_mm(crate::kernel::sched::current_cpu(), mm);
    }

    // 2. Update the per-CPU current_task pointer so that `get_current()`
    //    returns `next` from this point forward on this CPU.
    //
    //    Ref: Linux process_64.c `raw_cpu_write(current_task, next_p)`
    unsafe {
        crate::kernel::sched::set_current(next);
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
        (*task).thread.fsbase =
            crate::arch::x86::kernel::msr::read(crate::arch::x86::kernel::msr::MSR_FS_BASE);
        // While we run in kernel context the user's GS base lives in
        // KERNEL_GS_BASE (swapgs on entry stashed it there). Save it so an
        // involuntary switch does not clobber an ARCH_SET_GS user's GS base.
        // Ref: Linux process_64.c `save_fsgs()` / `save_base_legacy()`.
        (*task).thread.gsbase =
            crate::arch::x86::kernel::msr::read(crate::arch::x86::kernel::msr::MSR_KERNEL_GS_BASE);
    }
}

#[cfg(test)]
unsafe fn save_fsgs(_task: *mut TaskStruct) {}

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
    unsafe {
        save_fsgs(task);
    }
}

#[cfg(not(test))]
unsafe fn load_fsgs(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }

    unsafe {
        crate::arch::x86::kernel::msr::write(
            crate::arch::x86::kernel::msr::MSR_FS_BASE,
            (*task).thread.fsbase,
        );
        // Restore the incoming task's user GS base into KERNEL_GS_BASE; the
        // matching swapgs on the next ring-0 → ring-3 transition makes it the
        // active GS base in user mode.
        crate::arch::x86::kernel::msr::write(
            crate::arch::x86::kernel::msr::MSR_KERNEL_GS_BASE,
            (*task).thread.gsbase,
        );
    }
}

#[cfg(test)]
unsafe fn load_fsgs(_task: *mut TaskStruct) {}

#[cfg(not(test))]
unsafe fn load_task_cr3(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    let mm = unsafe {
        if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        }
    };
    if mm.is_null() {
        return;
    }
    unsafe {
        crate::mm::vmalloc::sync_vmalloc_to_mm(mm);
    }
    let pgd = unsafe { (*mm).pgd as u64 };
    if let Some(pgd_phys) = crate::arch::x86::mm::paging::virt_to_phys(pgd) {
        let current = crate::arch::x86::mm::paging::read_cr3();
        if current != pgd_phys {
            unsafe {
                core::arch::asm!(
                    "mov cr3, {0}",
                    in(reg) pgd_phys,
                    options(nostack, preserves_flags)
                );
            }
        }
    }
}

#[cfg(test)]
unsafe fn load_task_cr3(_task: *mut TaskStruct) {}

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
