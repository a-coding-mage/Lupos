//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry
//! linux-source: vendor/linux/arch/x86/entry/syscall_64.c
//! linux-source: vendor/linux/arch/x86/entry/entry_64.S
//! linux-source: vendor/linux/arch/x86/kernel/cpu/common.c
//! test-origin: linux:vendor/linux/arch/x86/entry
//! SYSCALL/SYSRET MSR setup — fast system-call entry point for 64-bit mode.
//!
//! Implements `do_syscall_64`-equivalent table dispatch over `pt_regs`, the
//! `syscall_init()` MSR setup (EFER.SCE/STAR/LSTAR/FMASK), the naked SYSCALL
//! entry stub, and the SYSRET-vs-IRET decision with Linux's exact predicates
//! (`rcx==rip`, `r11==eflags`, `cs==USER_CS`, `ss==USER_DS`, RF/TF clear).
//!
//! Remaining work vs Linux for genuine `complete`:
//!   * x32 ABI dispatch (`do_syscall_x32`) is not implemented (x86-64 only).
//!   * `check_pending_signals()` is a legacy no-op kept as an exported symbol;
//!     the live exit path is `syscall_exit_slowpath()`, which delivers frames.
//!
//! `SYSCALL` is the preferred system-call instruction for 64-bit Linux ABI.
//! It is faster than `INT 0x80` because it does not do a full interrupt-gate
//! transition (no privilege-level check through the TSS, no EFLAGS save via
//! the IDT path — the CPU loads CS/SS from MSRs directly).
//!
//! # How SYSCALL works (per Intel SDM Vol. 2B "SYSCALL")
//!
//! On `SYSCALL`:
//!   - RCX ← RIP (user return address — saved by CPU)
//!   - R11 ← RFLAGS (saved by CPU)
//!   - RIP ← IA32_LSTAR (our syscall entry stub)
//!   - CS  ← IA32_STAR[47:32] + 0           = KERNEL_CS (ring-0 code)
//!   - SS  ← IA32_STAR[47:32] + 8           = KERNEL_DS (ring-0 data)
//!   - RFLAGS &= ~IA32_FMASK               (clear IF, DF, TF on entry)
//!
//! On `SYSRET` (64-bit):
//!   - RIP    ← RCX (restored user RIP)
//!   - RFLAGS ← R11 (restored user RFLAGS)
//!   - CS     ← IA32_STAR[63:48] + 16 | RPL=3  = USER_CS
//!   - SS     ← IA32_STAR[63:48] + 8  | RPL=3  = USER_DS
//!
//! # GDT layout requirement
//!
//! With STAR[63:48] = USER32_CS = 0x23 (see `gdt::sel`):
//!   SYSRET SS = 0x23 + 8  = 0x2b → gdt[USER_DS] ✓
//!   SYSRET CS = 0x23 + 16 = 0x33 → gdt[USER_CS] ✓
//!
//! # References
//!   AMD64 APM Vol. 2 §2.5 "SYSCALL and SYSRET Instructions"
//!   Intel SDM Vol. 2B "SYSCALL — Fast System Call"
//!   Intel SDM Vol. 4 §2.1 "Architectural MSRs"
//!   Linux: arch/x86/kernel/cpu/common.c `syscall_init()`
//!   Linux: arch/x86/entry/entry_64.S `SYM_CODE_START(entry_SYSCALL_64)`

use crate::arch::x86::kernel::gdt::sel;
use crate::kernel::exec::UserStartContext;
use crate::kernel::seccomp::{
    SECCOMP_MODE_FILTER, SECCOMP_MODE_STRICT, SECCOMP_RET_ACTION_FULL, SECCOMP_RET_ALLOW,
    SECCOMP_RET_DATA, SECCOMP_RET_ERRNO, SECCOMP_RET_KILL_PROCESS, SECCOMP_RET_KILL_THREAD,
    SECCOMP_RET_LOG, SECCOMP_RET_TRACE, SECCOMP_RET_TRAP, SECCOMP_RET_USER_NOTIF, Seccomp,
    SeccompData, seccomp_run_filters,
};
use crate::kernel::signal;
use crate::kernel::task::{TIF_NEED_RESCHED, TIF_SIGPENDING};
use crate::kernel::trace::ring_buffer::{
    TRACE_RB, TRACE_SYSCALL_ENTER, TRACE_SYSCALL_EXIT, TraceEvent,
};
use crate::kernel::{audit, ptrace, sched};
use crate::log_error;

const ENOSYS: i64 = 38;
const EPERM: i64 = 1;
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;

fn enosys_log_suppressed(nr: usize) -> bool {
    matches!(
        nr,
        40   // sendfile: userland falls back to read/write
            | 275 // splice: userland falls back to buffered I/O
            | 326 // copy_file_range: coreutils/systemd fall back to userspace copy
            | 334 // rseq: glibc probes and disables rseq on ENOSYS
    )
}

// ── MSR addresses ────────────────────────────────────────────────────────────
//
// These are "Model-Specific Registers" — not I/O ports.  They are read and
// written with the `rdmsr`/`wrmsr` instructions (CPL=0 required).
//
// Reference: Intel SDM Vol. 4 §2.1, Table 2-1 "Architectural MSRs"

const MSR_EFER: u32 = 0xC000_0080; // Extended Feature Enable Register
const MSR_STAR: u32 = 0xC000_0081; // Segment selectors for SYSCALL/SYSRET
const MSR_LSTAR: u32 = 0xC000_0082; // SYSCALL target RIP (Long mode, 64-bit)
const MSR_FMASK: u32 = 0xC000_0084; // RFLAGS bits to clear on SYSCALL

// ── EFER flags ───────────────────────────────────────────────────────────────

/// EFER.SCE — System Call Extensions.  Must be set to enable SYSCALL/SYSRET.
/// arch/x86/boot/header.S already set EFER.LME and EFER.LMA for long mode; we OR in SCE.
const EFER_SCE: u64 = 1 << 0;

// ── RFLAGS mask ──────────────────────────────────────────────────────────────
//
// Bits cleared in RFLAGS when SYSCALL is executed (IA32_FMASK):
//   IF  (bit 9)  — Disable hardware interrupts during syscall prologue.
//                  The kernel re-enables them explicitly after swapping stacks.
//   DF  (bit 10) — Clear direction flag; C ABI assumes DF=0 at function entry.
//   TF  (bit 8)  — Clear single-step trap; prevents GDB from single-stepping
//                  into kernel code unless the kernel explicitly handles it.
//
// Linux masks these same flags (and a few more) in IA32_FMASK.
const RFLAGS_IF: u64 = 1 << 9;
const RFLAGS_DF: u64 = 1 << 10;
const RFLAGS_TF: u64 = 1 << 8;
const RFLAGS_FIXED: u64 = 1 << 1;
const SYSCALL_RFLAGS_MASK: u64 = RFLAGS_IF | RFLAGS_DF | RFLAGS_TF;

// ── Syscall entry stub ───────────────────────────────────────────────────────
//
// This stub is the first kernel code that runs when a user-space program
// executes `syscall`.  At entry:
//   - RCX = user RIP (return address, saved by CPU)
//   - R11 = user RFLAGS (saved by CPU)
//   - RAX = syscall number (Linux ABI)
//   - RDI, RSI, RDX, R10, R8, R9 = syscall arguments (Linux ABI)
//   - RSP = user stack pointer (NOT yet switched to kernel stack!)
//   - Interrupts are OFF (IF cleared by FMASK)
//
// DANGER: We are still running on the user stack.  The very first thing a
// real syscall handler must do is load RSP0 from the TSS (via SWAPGS + GS:0
// or directly from the TSS) to switch to a kernel stack.
//
// The entry path switches to the current task's kernel stack, builds a
// Linux-shaped `pt_regs`, dispatches through the x86-64 syscall table, runs the
// exit slow path, and returns with SYSRET.
//
// Reference: Linux entry_SYSCALL_64 in arch/x86/entry/entry_64.S
// Reference: https://wiki.osdev.org/SYSENTER_and_SYSEXIT#Differences_between_Intel_and_AMD

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syscall_entry() {
    // SAFETY: Naked stub — entered directly by the CPU; no Rust prologue.
    //
    // Entry state (set by CPU on SYSCALL):
    //   RCX  = user RIP (for SYSRET)
    //   R11  = user RFLAGS (for SYSRET)
    //   RAX  = syscall number
    //   RDI, RSI, RDX, R10, R8, R9 = syscall arguments (per Linux ABI)
    //   RSP  = user stack (not yet switched)
    //   Interrupts disabled (by FMASK)
    //
    // The goal: call Rust's `syscall_dispatch` with the arguments already in the
    // correct registers, check for pending signals, then return the result via SYSRET.
    //
    // Reference: Linux `arch/x86/entry/entry_64.S` `entry_SYSCALL_64`
    // Reference: AMD64 APM Vol. 2 §2.5 "SYSCALL and SYSRET Instructions"

    core::arch::naked_asm!(
        // Save the user entry context before we touch any registers.
        "swapgs",
        // Linux uses the per-CPU TSS sp2 slot as scratch here, then loads
        // cpu_current_top_of_stack rather than reading TSS.RSP0.  The TSS
        // remains the hardware privilege-transition source; this software
        // entry must follow Linux's current-task stack publication exactly.
        "mov qword ptr gs:[rip + {percpu_base} + {user_rsp_offset}], rsp",
        "mov rsp, qword ptr gs:[rip + {percpu_base} + {current_top_of_stack_offset}]",

        // Construct a Linux-shaped `struct pt_regs` on the kernel stack.
        "push {user_ds}", // ss
        "push qword ptr gs:[rip + {percpu_base} + {user_rsp_offset}]", // rsp
        "push r11", // eflags
        "push {user_cs}", // cs
        "push rcx", // rip
        "push rax", // orig_rax

        // General purpose registers (reverse order so RSP points at r15).
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx", // rcx (syscall clobbers rcx to RIP)
        "mov rax, -38", // -ENOSYS default
        "push rax", // rax
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Preserve the pt_regs pointer in a minimal aligned call frame.
        // The kernel target uses the soft-float ABI, exactly like Linux's
        // -mno-sse kernel build, so entry must not pay an FXSAVE/FXRSTOR pair
        // on every syscall. Task state is saved only by the context switch.
        "mov rdi, rsp",
        "sub rsp, 16",
        "and rsp, -16",
        "mov [rsp], rdi",

        // rdi = &pt_regs, do_syscall_64-like table dispatch.
        // Linux runs normal syscall bodies with IRQs enabled after the entry
        // frame is complete; otherwise blocking syscalls can schedule with IF
        // masked and starve timer-driven wakeups.
        "sti",
        "mov rdi, [rsp]",
        "call {dispatch_ptregs}",

        // Store return value into pt_regs.rax so the restore path reloads it.
        "mov rdi, [rsp]",
        "mov [rdi + 80], rax",

        // Run exit slowpath work before choosing the user return instruction.
        "mov rdi, [rsp]",
        "call {exit_slowpath}",

        // Linux only uses SYSRET for a clean syscall frame. Signal delivery
        // and rt_sigreturn can make user-visible RCX/R11 differ from the
        // SYSRET target/flags pair, so those paths must return via IRET.
        "mov rdi, [rsp]",
        "call {should_use_sysret}",
        // Keep the branch decision in the second scratch word across the
        // interrupt-disable and stack-pointer reload below.
        "mov [rsp + 8], al",

        // Keep interrupts closed while restoring userspace state and doing
        // SWAPGS/SYSRET or IRET, matching Linux's exit-to-user discipline.
        "cli",

        // Restore the branch decision and pt_regs stack pointer.
        "mov al, [rsp + 8]",
        "mov rsp, [rsp]",

        "test al, al",
        "jz 3f",

        // Restore registers from pt_regs and return to userspace via SYSRET.
        "mov r15, [rsp + 0]",
        "mov r14, [rsp + 8]",
        "mov r13, [rsp + 16]",
        "mov r12, [rsp + 24]",
        "mov rbp, [rsp + 32]",
        "mov rbx, [rsp + 40]",
        "mov r11, [rsp + 48]",
        "mov r10, [rsp + 56]",
        "mov r9,  [rsp + 64]",
        "mov r8,  [rsp + 72]",
        "mov rax, [rsp + 80]",
        "mov rcx, [rsp + 128]", // user RIP for SYSRET
        "mov r11, [rsp + 144]", // user RFLAGS for SYSRET
        "mov rdx, [rsp + 96]",
        "mov rsi, [rsp + 104]",
        "mov rdi, [rsp + 112]",
        "mov rsp, [rsp + 152]", // user RSP
        "swapgs",
        "sysretq",

        // IRET fallback for signal/ptrace-like frames. Unlike SYSRET, IRET
        // restores the user-visible RCX/R11 slots independently from RIP/RFLAGS.
        "3:",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rax",
        "pop rcx",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "add rsp, 8", // skip orig_rax; RIP/CS/RFLAGS/RSP/SS form the IRET frame.
        "swapgs",
        "iretq",

        percpu_base = sym crate::arch::x86::kernel::setup_percpu::LINUX_PER_CPU_AREAS,
        user_rsp_offset =
            const crate::arch::x86::kernel::setup_percpu::SYSCALL_USER_RSP_OFFSET,
        current_top_of_stack_offset = const crate::arch::x86::kernel::setup_percpu::CURRENT_TOP_OF_STACK_OFFSET,
        dispatch_ptregs = sym syscall_dispatch_ptregs,
        exit_slowpath = sym syscall_exit_slowpath,
        should_use_sysret = sym syscall_should_use_sysret,
        user_cs = const sel::USER_CS as u64,
        user_ds = const sel::USER_DS as u64,
    );
}

pub unsafe extern "C" fn syscall_exit_slowpath(
    regs: *mut crate::arch::x86::kernel::ptrace::PtRegs,
) {
    let task = sched::get_current();
    if task.is_null() {
        return;
    }
    let restart_pending = if regs.is_null() {
        false
    } else {
        matches!(unsafe { (*regs).rax as i64 }, -512 | -513 | -514 | -516)
    };
    if unsafe {
        (*task)
            .thread_info
            .flags
            .load(core::sync::atomic::Ordering::Acquire)
            & TIF_SIGPENDING
            != 0
    } || restart_pending
    {
        if regs.is_null() {
            while unsafe { crate::kernel::signal::do_signal_stop_only() } {}
        } else {
            // `arch::x86::kernel::ptrace::PtRegs` and `kernel::task::PtRegs` both mirror
            // Linux `struct pt_regs` with the same `repr(C)` layout; the latter
            // keeps Linux's short field names used by the signal frame builder.
            // A process-directed signal can be consumed by a sibling after a
            // blocking syscall has selected an internal restart result.  Still
            // run arch_do_signal_or_restart-equivalent processing so no
            // ERESTART* value can escape to userspace.
            let _ = unsafe { crate::kernel::signal::do_signal(regs.cast()) };
            // get_signal() consumes default stop/continue actions internally
            // before arch_do_signal_or_restart() applies restart semantics.
            // Lupos' combined do_signal() can return after those actions, so
            // finish any still-pending internal restart here.  This also makes
            // the exit boundary robust against a process-directed signal being
            // consumed by a sibling between the wait wakeup and this path.
            unsafe {
                crate::kernel::signal::apply_syscall_restart_without_handler(&mut *regs.cast());
            }
        }
    }
    sanitize_syscall_user_rflags(regs);
    let need_resched = unsafe {
        (*task)
            .thread_info
            .flags
            .load(core::sync::atomic::Ordering::Acquire)
            & TIF_NEED_RESCHED
            != 0
    };
    if need_resched && crate::kernel::locking::preempt::preempt_count() == 0 {
        unsafe {
            // The returning task is runnable; yield for fairness but never halt
            // it (that would waste a tick per syscall under a slow per-syscall
            // transport — see `reschedule_runnable`).
            sched::reschedule_runnable();
        }
    }
}

fn sanitize_user_rflags(flags: u64) -> u64 {
    flags | RFLAGS_FIXED | RFLAGS_IF
}

fn sanitize_syscall_user_rflags(regs: *mut crate::arch::x86::kernel::ptrace::PtRegs) {
    if regs.is_null() {
        return;
    }
    unsafe {
        let old_flags = (*regs).eflags;
        let new_flags = sanitize_user_rflags(old_flags);
        (*regs).eflags = new_flags;
        if (*regs).r11 == old_flags {
            (*regs).r11 = new_flags;
        }
    }
}

pub(crate) unsafe extern "C" fn syscall_should_use_sysret(
    regs: *const crate::arch::x86::kernel::ptrace::PtRegs,
) -> bool {
    if regs.is_null() {
        return false;
    }
    let regs = unsafe { &*regs };
    // Focused return-path probe for the four-CPU graphics failure.  A normal
    // x86-64 executable mapping is far below this top-of-user-stack window;
    // reaching it at the SYSRET/IRET decision point means the frame was
    // already corrupt before the assembly restore sequence runs.  Keep this
    // conditional diagnostic until the first divergence is proven.
    if regs.rip >= 0x0000_7fff_f000_0000 {
        let task = unsafe { sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        log_error!(
            "syscall",
            "syscall: suspicious-user-return pid={} nr={} rip={:#018x} rsp={:#018x} rcx={:#018x} r11={:#018x} flags={:#018x}",
            pid,
            regs.orig_rax,
            regs.rip,
            regs.rsp,
            regs.rcx,
            regs.r11,
            regs.eflags,
        );
    }
    syscall_sysret_fast_path_enabled() && syscall_frame_allows_sysret(regs)
}

fn syscall_frame_allows_sysret(regs: &crate::arch::x86::kernel::ptrace::PtRegs) -> bool {
    // Mirrors Linux arch/x86/entry/syscall_64.c::do_syscall_64(): SYSRET is
    // only safe when RCX/R11 still match the architectural RIP/RFLAGS return
    // pair and the frame is a normal 64-bit user frame.
    regs.rcx == regs.rip
        && regs.r11 == regs.eflags
        && regs.cs == sel::USER_CS as u64
        && regs.ss == sel::USER_DS as u64
        && regs.rip < crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX
        && regs.eflags
            & (crate::arch::x86::kernel::ptrace::X86_EFLAGS_RF
                | crate::arch::x86::kernel::ptrace::X86_EFLAGS_TF)
            == 0
}

fn syscall_sysret_fast_path_enabled() -> bool {
    true
}

#[cfg(not(test))]
unsafe fn load_current_user_cr3() {
    let task = sched::get_current();
    if task.is_null() {
        return;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return;
    }
    unsafe {
        crate::mm::vmalloc::sync_vmalloc_to_mm(mm);
    }
    let pgd_virt = unsafe { (*mm).pgd as u64 };
    if let Some(pgd_phys) = crate::arch::x86::mm::paging::virt_to_phys(pgd_virt) {
        unsafe {
            // Match switch_mm_irqs_off()'s conservative transition state.
            // This exec path can still have IRQs enabled, so a shootdown that
            // lands on either side of MOV-to-CR3 must flush rather than infer
            // which address space is currently loaded.
            let cpu = sched::current_cpu();
            crate::arch::x86::mm::tlb::set_active_mm_switching(cpu);
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            core::arch::asm!(
                "mov cr3, {0}",
                in(reg) pgd_phys,
                options(nostack, preserves_flags)
            );
            crate::arch::x86::mm::tlb::set_active_mm(cpu, mm);
        }
    }
}

#[cfg(test)]
unsafe fn load_current_user_cr3() {}

/// Syscall dispatcher for the syscall entry stub.
///
/// Mirrors Linux `do_syscall_64()` in the sense that it is table-driven and
/// operates over a `pt_regs` frame.
///
/// # Safety
/// `regs` must point to a valid kernel-stack `PtRegs`.
pub unsafe extern "C" fn syscall_dispatch_ptregs(
    regs: *mut crate::arch::x86::kernel::ptrace::PtRegs,
) -> i64 {
    if regs.is_null() {
        return -ENOSYS;
    }
    unsafe { syscall_dispatch_ptregs_inner(regs) }
}

/// Last jiffy on which the per-syscall console drain ran (throttle state).
///
/// The common case is a read that observes the current jiffy. Only syscalls
/// racing at a jiffy transition attempt the locked compare/exchange, so
/// syscall-heavy SMP workloads do not serialize on an unconditional exchange.
static SYSCALL_DRAIN_LAST_JIFFY: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(u64::MAX);

#[inline]
fn syscall_console_drain_due(last_jiffy: &core::sync::atomic::AtomicU64, now: u64) -> bool {
    use core::sync::atomic::Ordering;

    let previous = last_jiffy.load(Ordering::Relaxed);
    previous != now
        && last_jiffy
            .compare_exchange(previous, now, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
}

unsafe fn syscall_dispatch_ptregs_inner(
    regs: *mut crate::arch::x86::kernel::ptrace::PtRegs,
) -> i64 {
    use super::syscall_table::{NR_syscalls, SYS_CALL_TABLE};

    let nr = unsafe { (*regs).orig_rax } as usize;
    let task = current_task_for_syscall();
    let hook_state = syscall_enter(unsafe { &*regs }, task);
    trace_udev_syscall_enter(unsafe { &*regs }, task);
    trace_stall_syscall_enter(unsafe { &*regs }, task);
    // Focused Firefox startup evidence: keep the existing opt-in probe on the
    // clone3 boundary while the browser process-creation path is compared
    // with Linux.  The probe itself filters to nr=435 below.
    trace_firefox_syscall_enter(unsafe { &*regs }, task);
    // Draining the console here delivers terminal signals (Ctrl-C) promptly, but
    // `try_console_input` probes the i8042 status port (`inb 0x64`) when its
    // queue is empty — a port-I/O access that is a VM-exit under VirtualBox/KVM
    // and slow emulation under TCG. Running it on *every* syscall made it the
    // dominant boot cost (the syscall-heavy systemd generators). Throttle to at
    // most once per tick: Ctrl-C latency stays ≤1 jiffy, but the per-syscall
    // port I/O is gone. The console wait loops (console_read, epoll, …) still
    // drain unthrottled, so interactive input is unaffected.
    #[cfg(not(test))]
    {
        let now = crate::kernel::time::jiffies::jiffies();
        if syscall_console_drain_due(&SYSCALL_DRAIN_LAST_JIFFY, now) {
            crate::init::rootfs::drain_console_control_bytes();
        }
    }

    let ret = match syscall_seccomp_check(unsafe { &*regs }, task) {
        SeccompCheck::Allow if nr < NR_syscalls => unsafe { SYS_CALL_TABLE[nr](regs) },
        SeccompCheck::Allow => {
            #[cfg(not(test))]
            {
                let pid = if task.is_null() {
                    -1
                } else {
                    unsafe { (*task).pid }
                };
                if pid != 1 && pid > 0 {
                    crate::linux_driver_abi::tty::serial_println!(
                        "enosys-oor pid={} nr={}",
                        pid,
                        nr
                    );
                }
            }
            -ENOSYS
        }
        SeccompCheck::Errno(errno) => errno,
        SeccompCheck::Trap(data) => {
            // Linux `SECCOMP_RET_TRAP` rolls the syscall registers back,
            // skips dispatch, and forces SIGSYS/SYS_SECCOMP.  The userspace
            // handler sees the original syscall number in both RAX and
            // siginfo, and may replace RAX in its ucontext to emulate it.
            unsafe { queue_seccomp_trap(&mut *regs, task, data) };
            unsafe { (*regs).orig_rax as i64 }
        }
    };

    #[cfg(not(test))]
    trace_seccomp_control(unsafe { &*regs }, task, ret);

    #[cfg(not(test))]
    if ret == -ENOSYS {
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        if pid != 1 && pid > 0 && !enosys_log_suppressed(nr) {
            crate::linux_driver_abi::tty::serial_println!("enosys pid={} nr={}", pid, nr);
        }
    }
    unsafe {
        (*regs).rax = ret as u64;
    }
    if ret == 0 {
        if let Some(ctx) = crate::kernel::exec::take_exec_start_for_current() {
            unsafe {
                load_current_user_cr3();
                if ctx.old_mm != 0 {
                    crate::mm::fork::mmput(ctx.old_mm as *mut crate::mm::mm_types::MmStruct);
                }
                reset_successful_exec_user_tls_bases(task);
                initialize_exec_registers(&mut *regs, &ctx);
            }
        }
    }
    syscall_exit(unsafe { &*regs }, ret, task, hook_state);
    trace_systemd_service_syscall(unsafe { &*regs }, ret, task);
    trace_udev_syscall_exit(unsafe { &*regs }, ret, task);
    trace_firefox_syscall_exit(unsafe { &*regs }, ret, task);
    ret
}

/// Install the new image's live FS/GS bases on successful `execve`.
///
/// Ordinary syscalls leave the hardware bases untouched. `arch_prctl()` writes
/// a current task's requested base immediately, and `__switch_to()` restores
/// bases for a task switch. Exec is the exception: it replaces the image
/// without switching tasks, after `exec.rs` has reset the saved bases. Linux's
/// `start_thread_common()` clears the live FS/GS state at this same transition.
unsafe fn reset_successful_exec_user_tls_bases(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }

    let fsbase = unsafe { (*task).thread.fsbase };
    let gsbase = unsafe { (*task).thread.gsbase };
    unsafe {
        // Linux start_thread_common() clears the selectors before installing
        // the new image's bases.
        crate::arch::x86::kernel::gdt::load_ds(0);
        crate::arch::x86::kernel::gdt::load_es(0);
        crate::arch::x86::kernel::gdt::load_fs(0);
        crate::arch::x86::kernel::gdt::load_gs_index(0);
        crate::arch::x86::kernel::msr::write(crate::arch::x86::kernel::msr::MSR_FS_BASE, fsbase);
        // SYSCALL entry has swapped to the kernel GS base. The new image's
        // inactive user GS base therefore lives in IA32_KERNEL_GS_BASE until
        // the matching return-to-user SWAPGS.
        crate::arch::x86::kernel::msr::write(
            crate::arch::x86::kernel::msr::MSR_KERNEL_GS_BASE,
            gsbase,
        );
    }
}

/// Initialize the x86-64 user register image for a newly executed ELF.
///
/// Linux's `ELF_PLAT_INIT()` calls `elf_common_init()` before
/// `start_thread()`: all general-purpose registers except `%rax` are cleared,
/// `%rax` carries execve's zero return value, and the new instruction/stack
/// pointers and user segments are installed. In particular, `%rdx` must be
/// zero. The System V startup ABI treats a nonzero `%rdx` as the dynamic
/// loader's finalizer callback; leaking execve's old `envp` there makes static
/// PIE startup register a data pointer with `atexit()`.
///
/// Ref: Linux `arch/x86/include/asm/elf.h::elf_common_init()` and
/// `arch/x86/kernel/process_64.c::start_thread_common()`.
fn initialize_exec_registers(
    regs: &mut crate::arch::x86::kernel::ptrace::PtRegs,
    ctx: &UserStartContext,
) {
    regs.r15 = 0;
    regs.r14 = 0;
    regs.r13 = 0;
    regs.r12 = 0;
    regs.rbp = 0;
    regs.rbx = 0;
    regs.r11 = 0;
    regs.r10 = 0;
    regs.r9 = 0;
    regs.r8 = 0;
    regs.rax = 0;
    regs.rcx = 0;
    regs.rdx = 0;
    regs.rsi = 0;
    regs.rdi = 0;
    regs.rip = ctx.ip;
    regs.cs = sel::USER_CS as u64;
    regs.eflags = ctx.rflags;
    regs.rsp = ctx.sp;
    regs.ss = sel::USER_DS as u64;
}

#[cfg(not(test))]
fn trace_udev_syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null()
        || !crate::kernel::debug_trace::udev_enabled()
        || !crate::kernel::debug_trace::syscall_enabled()
    {
        return;
    }
    let comm = unsafe { &(*task).comm };
    if !comm_starts_with(comm, b"systemd-udevd") {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-udev-enter pid={} nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x}",
        unsafe { (*task).pid },
        regs.orig_rax,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3(),
        regs.arg4(),
        regs.arg5()
    );
}

#[cfg(test)]
fn trace_udev_syscall_enter(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

/// Syscalls that can block indefinitely, so an *enter* record with no matching
/// exit identifies where a task went to sleep.
///
/// `trace_systemd_service_syscall()` only records syscall exits, which is
/// blind to exactly the case the boot stall needs: a task that enters a
/// syscall and never returns emits nothing at all.  Pairing this enter record
/// with that exit record turns the stall into a readable "last call in, no
/// call out" for PID 1 and the `systemd-*` helpers.
///
/// Restricted to blocking-capable numbers so enabling `lupos.trace=syscall`
/// does not flood the serial console with the read/write/openat storm that
/// systemd's generators produce.
fn trace_stall_syscall_is_blocking(nr: u64) -> bool {
    matches!(
        nr,
        0     // read
        | 7   // poll
        | 16  // ioctl
        | 23  // select
        | 34  // pause
        | 35  // nanosleep
        | 42  // connect
        | 43  // accept
        | 45  // recvfrom
        | 47  // recvmsg
        | 61  // wait4
        | 165 // mount
        | 202 // futex
        | 230 // clock_nanosleep
        | 232 // epoll_wait
        | 247 // waitid
        | 257 // openat
        | 270 // pselect6
        | 271 // ppoll
        | 281 // epoll_pwait
        | 441 // epoll_pwait2
    )
}

#[cfg(not(test))]
fn trace_stall_syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null() || !crate::kernel::debug_trace::syscall_enabled() {
        return;
    }
    let nr = regs.orig_rax;
    if !trace_stall_syscall_is_blocking(nr) {
        return;
    }
    let pid = unsafe { (*task).pid };
    let comm = unsafe { &(*task).comm };
    // Same selection the exit trace uses: PID 1 plus the `systemd-*` helpers.
    if pid != 1 && !comm_starts_with(comm, b"systemd-") {
        return;
    }
    // jiffies anchors the record against the stall window in the serial log.
    crate::linux_driver_abi::tty::serial_println!(
        "trace-stall-enter j={} pid={} nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x}",
        crate::kernel::time::jiffies::jiffies(),
        pid,
        nr,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3()
    );
}

#[cfg(test)]
fn trace_stall_syscall_enter(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
fn trace_udev_syscall_exit(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    ret: i64,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null()
        || !crate::kernel::debug_trace::udev_enabled()
        || !crate::kernel::debug_trace::syscall_enabled()
    {
        return;
    }
    let comm = unsafe { &(*task).comm };
    if !comm_starts_with(comm, b"systemd-udevd") {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-udev-exit pid={} nr={} ret={}",
        unsafe { (*task).pid },
        regs.orig_rax,
        ret
    );
}

#[cfg(test)]
fn trace_udev_syscall_exit(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _ret: i64,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

/// Helper function to get the current CPU's ID.
/// Returns the dense logical CPU number from Linux's GS-relative per-CPU area.
///
/// The assembly entry path no longer needs this out-of-line helper, but the
/// exported symbol remains for compatibility with existing probes.
#[unsafe(no_mangle)]
pub extern "C" fn current_cpu_id() -> usize {
    crate::arch::x86::kernel::setup_percpu::current_cpu_number()
}

/// Check for pending signals on the current task and deliver them if present.
///
/// Called from the syscall exit path after syscall_dispatch returns.
/// This is a simplified version that checks TIF_SIGPENDING but does not yet
/// construct signal frames (which requires access to the full PtRegs structure).
///
/// TODO (M25 full impl): Build PtRegs on kernel stack during syscall entry,
/// then pass it to do_signal for proper frame construction.
#[unsafe(no_mangle)]
pub extern "C" fn check_pending_signals() {
    unsafe {
        let task = unsafe { sched::get_current() };
        if task.is_null() {
            return;
        }

        // Check if TIF_SIGPENDING is set.
        let has_pending = {
            let thread_info = &(*task).thread_info;
            (thread_info
                .flags
                .load(core::sync::atomic::Ordering::Acquire)
                & TIF_SIGPENDING)
                != 0
        };

        if !has_pending {
            return;
        }

        // TODO: Call do_signal(regs) once PtRegs is available on the kernel stack.
        // For now, just clear the flag to prevent spin-waiting.
        signal::clear_tif_sigpending(task);
    }
}

// ── Ring-3 entry point ───────────────────────────────────────────────────────
//
// After `execve` succeeds and stores a `UserStartContext`, the task needs to
// enter ring 3 (user mode) and start executing the binary. This function
// synthesises a SYSRET to jump to ring 3 with the given context.
//
// This is typically called from the syscall exit path (after `sys_execve` returns
// success) or from a dedicated userspace entry scheduler.

/// Enter ring 3 (user mode) with the given context.
///
/// This function never returns — it transfers control to the user binary
/// at `ctx.ip` with the stack pointer set to `ctx.sp` and RFLAGS to `ctx.rflags`.
///
/// # Safety
/// - The context must be valid: `ctx.ip` must be a valid user-space code address,
///   `ctx.sp` must be a valid user-space stack address.
/// - Must be called from ring-0 (kernel context).
/// - Interrupts are disabled for the final kernel-side restore window; SYSRET
///   restores the user-visible IF bit from `ctx.rflags`.
///
/// # How it works
///
/// The x86-64 `SYSRET` instruction is the fast return path from syscalls:
///   RIP ← RCX (restored from kernel stack frame)
///   RFLAGS ← R11 (restored from kernel stack frame)
///   CS ← USER_CS (from IA32_STAR[63:48] + 16 with RPL=3)
///   SS ← USER_DS (from IA32_STAR[63:48] + 8 with RPL=3)
///
/// We set up the registers and execute `sysretq`, which acts as a "fake" syscall
/// return, thereby transferring to ring 3.
#[unsafe(naked)]
pub unsafe extern "C" fn enter_userspace(ctx: &UserStartContext) -> ! {
    core::arch::naked_asm!(
        // Close the interrupt window before switching RSP to the user stack.
        "cli",
        // Load the user context into the registers used by SYSRET.
        //   RCX = user RIP (SYSRET loads this into RIP)
        //   R11 = user RFLAGS (SYSRET loads this into RFLAGS)
        //   RSP = user stack pointer

        // The context is passed in RDI (first argument, per x86-64 System V ABI).
        // We need to extract the fields and load them into the appropriate registers.
        "mov rcx, [rdi + 0]",  // RCX = ctx.ip (field offset 0, 8 bytes)
        "mov r11, [rdi + 16]", // R11 = ctx.rflags (field offset 16, 8 bytes)
        "mov rsp, [rdi + 8]",  // RSP = ctx.sp (field offset 8, 8 bytes)
        // Switch back to user GS base (if we're running in kernel GS context).
        "swapgs",
        // Execute SYSRET: transfer to ring 3 with the loaded context.
        // This instruction does not return to the calling kernel code; it transfers
        // control to user-space at RCX with RFLAGS from R11.
        "sysretq",
        // Unreachable: SYSRET is a non-returning instruction, but Rust doesn't
        // know that. We use a loop to satisfy the compiler's return type checking.
        "2: jmp 2b",
    );
}

// ── MSR access helpers ───────────────────────────────────────────────────────

/// Write a 64-bit value into a Model-Specific Register.
///
/// `wrmsr` takes the MSR address in ECX and the value split across EDX:EAX.
///
/// # Safety
/// - Privileged instruction (CPL=0).
/// - Writing incorrect values to MSRs can hang, reset, or corrupt the CPU.
#[inline]
unsafe fn wrmsr(msr: u32, value: u64) {
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("rcx") msr,
            in("rax") (value & 0xFFFF_FFFF) as u32,
            in("rdx") (value >> 32) as u32,
            options(nostack, nomem, preserves_flags),
        );
    }
}

/// Read a 64-bit value from a Model-Specific Register.
///
/// # Safety
/// Privileged instruction (CPL=0).
#[inline]
unsafe fn rdmsr(msr: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("rcx") msr,
            out("rax") lo,
            out("rdx") hi,
            options(nostack, nomem, preserves_flags),
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

// ── Initialization ───────────────────────────────────────────────────────────

/// Enable SYSCALL/SYSRET and configure the four required MSRs.
///
/// Must be called after `gdt::init()` so that the segment selectors stored in
/// STAR are valid and the GDT backing them is live.
///
/// # Safety
/// - Must run at CPL=0 (`wrmsr` is privileged).
/// - Must run on the target physical CPU (MSRs are per-CPU registers).
/// - Not re-entrant; call once from `kernel_main`.
unsafe fn init_cpu_msrs() {
    unsafe {
        // 1. Enable SCE in EFER.
        //    arch/x86/boot/header.S set LME (bit 8) and the CPU set LMA (bit 10) automatically.
        //    We preserve those bits and add SCE (bit 0).
        let efer = rdmsr(MSR_EFER);
        wrmsr(MSR_EFER, efer | EFER_SCE);

        // 2. Configure IA32_STAR — segment selectors for SYSCALL/SYSRET.
        //
        //    STAR[47:32] = kernel CS selector (SYSCALL loads CS and CS+8 as SS)
        //      → KERNEL_CS = 0x10 → CS = 0x10, SS = 0x18 = KERNEL_DS ✓
        //
        //    STAR[63:48] = base for user selectors (SYSRET adds 8 for SS, 16 for CS)
        //      → USER32_CS = 0x23 → SS = 0x2b = USER_DS, CS = 0x33 = USER_CS ✓
        //
        //    CPU automatically forces RPL=3 on the CS/SS selectors loaded by SYSRET.
        let star: u64 = ((sel::USER32_CS as u64) << 48) | ((sel::KERNEL_CS as u64) << 32);
        wrmsr(MSR_STAR, star);

        // 3. Set LSTAR — the RIP the CPU jumps to on SYSCALL.
        wrmsr(MSR_LSTAR, syscall_entry as *const () as u64);

        // 4. Set FMASK — RFLAGS bits cleared on SYSCALL entry.
        //    Clearing IF disables hardware interrupts until the kernel explicitly
        //    re-enables them after switching to a safe kernel stack.
        wrmsr(MSR_FMASK, SYSCALL_RFLAGS_MASK);
    }
}

/// Initialize CPU0's per-CPU entry area and SYSCALL MSRs.
///
/// # Safety
/// Same constraints as `init_cpu_msrs`; CPU0's GDT and TSS must already be live.
pub unsafe fn init() {
    crate::arch::x86::kernel::setup_percpu::setup_percpu_segment(0);
    unsafe {
        init_cpu_msrs();
    }
}

/// Program SYSCALL/SYSRET MSRs on the current application processor.
///
/// `setup_percpu_segment(cpu)` must already have installed this AP's GS base.
/// Unlike `init()`, this deliberately does not touch GS, so it cannot reset the
/// BSP's per-CPU state and it preserves the AP-local area selected by bring-up.
///
/// # Safety
/// Must run once at CPL0 on the target AP after its GDT, TSS, and per-CPU GS
/// area are initialized.
pub unsafe fn init_ap() {
    unsafe {
        init_cpu_msrs();
    }
}

// ── Syscall numbers — x86-64 Linux ABI ──────────────────────────────────────
//
// These match the x86-64 Linux syscall table.
// Ref: Linux `arch/x86/entry/syscalls/syscall_64.tbl`

/// `mmap` (anonymous + file-backed) — Linux syscall 9.
pub const SYS_MMAP: u64 = 9;
/// `mprotect` — Linux syscall 10.
pub const SYS_MPROTECT: u64 = 10;
/// `munmap` — Linux syscall 11.
pub const SYS_MUNMAP: u64 = 11;
/// `brk` — Linux syscall 12.
pub const SYS_BRK: u64 = 12;
/// `mremap` — Linux syscall 25.
pub const SYS_MREMAP: u64 = 25;
/// `madvise` — Linux syscall 28.
pub const SYS_MADVISE: u64 = 28;
/// `rt_sigaction` — Linux syscall 13.
pub const SYS_RT_SIGACTION: u64 = 13;
/// `rt_sigprocmask` — Linux syscall 14.
pub const SYS_RT_SIGPROCMASK: u64 = 14;
/// `rt_sigreturn` — Linux syscall 15.
pub const SYS_RT_SIGRETURN: u64 = 15;
/// `clone` — Linux syscall 56.
pub const SYS_CLONE: u64 = 56;
/// `fork` — Linux syscall 57.
pub const SYS_FORK: u64 = 57;
/// `execve` — Linux syscall 59.
pub const SYS_EXECVE: u64 = 59;
/// `rt_sigpending` — Linux syscall 127.
pub const SYS_RT_SIGPENDING: u64 = 127;
/// `rt_sigtimedwait` — Linux syscall 128.
pub const SYS_RT_SIGTIMEDWAIT: u64 = 128;
/// `rt_sigqueueinfo` — Linux syscall 129.
pub const SYS_RT_SIGQUEUEINFO: u64 = 129;
/// `sigaltstack` — Linux syscall 131.
pub const SYS_SIGALTSTACK: u64 = 131;
/// `tkill` — Linux syscall 200.
pub const SYS_TKILL: u64 = 200;
/// `tgkill` — Linux syscall 234.
pub const SYS_TGKILL: u64 = 234;
/// `execveat` — Linux syscall 322.
pub const SYS_EXECVEAT: u64 = 322;

// ── M26 — exit / wait / ptrace ───────────────────────────────────────────────

/// `exit` — Linux syscall 60.  Terminate calling thread.
pub const SYS_EXIT: u64 = 60;
/// `wait4` — Linux syscall 61.  Wait for and reap a zombie child.
pub const SYS_WAIT4: u64 = 61;
/// `ptrace` — Linux syscall 101.
pub const SYS_PTRACE: u64 = 101;
/// `exit_group` — Linux syscall 231.  Terminate all threads in the tgid.
pub const SYS_EXIT_GROUP: u64 = 231;
/// `waitid` — Linux syscall 247.  Wait with siginfo output.
pub const SYS_WAITID: u64 = 247;

// ── Rust-level syscall dispatcher ────────────────────────────────────────────
//
// `syscall_dispatch` is the Rust entry point used by tests and helper paths.
// The hot assembly entry path calls `syscall_dispatch_ptregs` directly after
// constructing a Linux-shaped `pt_regs` frame.
//
// Argument mapping follows the Linux x86-64 syscall ABI:
//   rax = nr, rdi = a0, rsi = a1, rdx = a2, r10 = a3, r8 = a4, r9 = a5
//
// Ref: Linux `arch/x86/entry/entry_64.S` — `entry_SYSCALL_64`

/// Rust syscall dispatcher.
///
/// Called with the six syscall arguments extracted from registers.
/// Returns the syscall return value (negative errno on error, Linux convention).
///
/// # Safety
/// Must be called with a valid `mm` pointer in a process context.
pub unsafe fn syscall_dispatch(
    nr: u64,
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
) -> i64 {
    // Table-driven dispatch (M59).  We build a synthetic PtRegs on the stack
    // populated only with the syscall-ABI fields (orig_rax + rdi/rsi/rdx/r10/r8/r9)
    // and hand it to the wrapper from SYS_CALL_TABLE. The hot entry path uses
    // the full `pt_regs` frame built by assembly; this helper preserves the
    // older positional calling convention for Rust tests and internal callers.
    //
    // Ref: vendor/linux/arch/x86/entry/syscall_64.c::do_syscall_64
    use crate::arch::x86::kernel::ptrace::PtRegs;
    let mut regs = PtRegs {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        rbp: 0,
        rbx: 0,
        r11: 0,
        r10: a3,
        r9: a5,
        r8: a4,
        rax: nr,
        rcx: 0,
        rdx: a2,
        rsi: a1,
        rdi: a0,
        orig_rax: nr,
        rip: 0,
        cs: 0,
        eflags: 0,
        rsp: 0,
        ss: 0,
    };

    unsafe { syscall_dispatch_ptregs_inner(&mut regs as *mut PtRegs) }
}

#[cfg(not(test))]
fn current_task_for_syscall() -> *mut crate::kernel::task::TaskStruct {
    unsafe { sched::get_current() }
}

#[cfg(test)]
fn current_task_for_syscall() -> *mut crate::kernel::task::TaskStruct {
    core::ptr::null_mut()
}

fn current_pid(task: *mut crate::kernel::task::TaskStruct) -> i32 {
    if task.is_null() {
        0
    } else {
        unsafe { (*task).pid }
    }
}

#[derive(Clone, Copy)]
struct SyscallHookState {
    audit_matched: bool,
}

fn syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) -> SyscallHookState {
    let nr = regs.orig_rax as i32;
    let pid = current_pid(task);
    let audit_matched = audit::audit_filter_syscall(nr, pid);

    if audit_matched {
        audit::audit_log(&alloc::format!(
            "type=SYSCALL syscall={} pid={} phase=enter",
            nr,
            pid
        ));
    }

    unsafe {
        ptrace::syscall_trace_enter(task, regs);
    }
    trace_ping_syscall_enter(regs, task);
    trace_executor_syscall_enter(regs, task);
    trace_bwrap_syscall_enter(regs, task);
    trace_xfce_syscall_enter(regs, task);
    trace_syscall(TRACE_SYSCALL_ENTER, regs, 0, pid);
    SyscallHookState { audit_matched }
}

fn syscall_exit(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    ret: i64,
    task: *mut crate::kernel::task::TaskStruct,
    hook_state: SyscallHookState,
) {
    let pid = current_pid(task);
    if hook_state.audit_matched {
        audit::audit_log(&alloc::format!(
            "type=SYSCALL syscall={} pid={} phase=exit ret={}",
            regs.orig_rax,
            pid,
            ret
        ));
    }
    unsafe {
        ptrace::syscall_trace_exit(task, regs, ret);
    }
    trace_syscall(TRACE_SYSCALL_EXIT, regs, ret, pid);
}

fn trace_syscall(
    ev_type: u32,
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    ret: i64,
    pid: i32,
) {
    TRACE_RB.push(TraceEvent {
        ts_nsec: crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000,
        ev_type,
        cpu: 0,
        pid: pid.clamp(0, u16::MAX as i32) as u16,
        arg0: regs.orig_rax,
        arg1: ret as u64,
    });
}

#[cfg(not(test))]
fn trace_ping_syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null() {
        return;
    }
    let comm = unsafe { &(*task).comm };
    let pid = unsafe { (*task).pid };
    if !crate::kernel::debug_trace::ping_enabled() {
        return;
    }
    if !comm_starts_with(comm, b"ping") && !crate::kernel::debug_trace::ping_pid_matches(pid) {
        return;
    }
    let nr = regs.orig_rax;
    if !trace_service_syscall_is_interesting(nr, 0, false, true, false) {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-ping-sys-enter pid={} comm={} nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x}",
        pid,
        comm_for_trace(comm),
        nr,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3(),
        regs.arg4(),
        regs.arg5()
    );
}

#[cfg(test)]
fn trace_ping_syscall_enter(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
static TRACE_EXECUTOR_SYSCALL_ENTER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(not(test))]
fn trace_executor_syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null() || !crate::kernel::debug_trace::proc_enabled() {
        return;
    }
    let comm = unsafe { &(*task).comm };
    if !comm_starts_with(comm, b"systemd-executo") {
        return;
    }
    let count =
        TRACE_EXECUTOR_SYSCALL_ENTER_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    if count >= 400 {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-executor-sys-enter pid={} comm={} nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x}",
        unsafe { (*task).pid },
        comm_for_trace(comm),
        regs.orig_rax,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3(),
        regs.arg4(),
        regs.arg5()
    );
}

#[cfg(test)]
fn trace_executor_syscall_enter(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
static TRACE_BWRAP_SYSCALL_ENTER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(not(test))]
fn trace_bwrap_syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null() || !crate::kernel::debug_trace::glycin_enabled() {
        return;
    }
    let comm = unsafe { &(*task).comm };
    if !comm_starts_with(comm, b"bwrap") {
        return;
    }
    let count = TRACE_BWRAP_SYSCALL_ENTER_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    if count >= 800 {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-bwrap-sys-enter seq={} pid={} nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x}",
        count,
        unsafe { (*task).pid },
        regs.orig_rax,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3(),
        regs.arg4(),
        regs.arg5()
    );
}

#[cfg(not(test))]
static TRACE_FIREFOX_SYSCALL_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(not(test))]
static TRACE_FIREFOX_POLL_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(not(test))]
static TRACE_FIREFOX_RECVMSG_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(not(test))]
static TRACE_FIREFOX_FUTEX_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(not(test))]
static TRACE_FIREFOX_CLONE_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(not(test))]
static TRACE_FIREFOX_NAMESPACE_CLONE_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(not(test))]
static TRACE_FIREFOX_TGID: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(-1);
#[cfg(not(test))]
static TRACE_FIREFOX_SPAWN_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

/// Comms a spawn child carries between `fork()` and `execve()`: it inherits
/// the forking thread's name, so glycin's loader spawn shows up as
/// `gly-hdl-loader` or `blocking-N` until the exec renames it.
#[cfg(not(test))]
fn firefox_trace_spawn_child_comm(comm: &[u8; 16]) -> bool {
    comm_starts_with(comm, b"gly-hdl-loader") || comm_starts_with(comm, b"blocking-")
}

/// Trace the small set of process, IPC, blocking, and sandbox syscalls that
/// determines whether Firefox can create its content process and publish its
/// first X11 window.  The flag is intentionally separate from TRACE_SYSCALL:
/// a full syscall stream changes the desktop schedule enough to hide the
/// liveness bug this probe is meant to locate.
#[cfg(not(test))]
fn trace_firefox_syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if !firefox_trace_task(task) {
        return;
    }
    let comm = unsafe { &(*task).comm };
    let nr = regs.orig_rax;
    // A forked-but-not-yet-exec'd spawn child gets *every* syscall traced.
    // Which calls it makes between fork() and the futex it dies on is the
    // whole question, and the ordinary filters below hide most of them: with
    // them applied the stuck child produced exactly one line. It is only a
    // handful of syscalls per child, on its own budget, so the volume stays
    // negligible.
    let spawn_child =
        unsafe { (*task).pid == (*task).tgid } && firefox_trace_spawn_child_comm(comm);
    if !spawn_child && !firefox_trace_syscall_is_interesting(nr, 0) {
        return;
    }
    // Firefox's sandbox can use a process clone with namespace flags after
    // its large thread-clone burst.  Keep that boundary visible even after
    // the ordinary clone3 budget is exhausted.
    let namespace_clone_flags = if nr == 435 && regs.arg0() != 0 && regs.arg1() <= 4096 {
        let clone_args = unsafe { *(regs.arg0() as *const crate::kernel::clone::CloneArgs) };
        clone_args.flags
            & (crate::kernel::clone::CLONE_NEWUSER
                | crate::kernel::clone::CLONE_NEWPID
                | crate::kernel::clone::CLONE_NEWNS
                | crate::kernel::clone::CLONE_NEWNET
                | crate::kernel::clone::CLONE_NEWIPC
                | crate::kernel::clone::CLONE_NEWUTS
                | crate::kernel::clone::CLONE_NEWCGROUP)
    } else if nr == 56 {
        regs.arg0()
            & (crate::kernel::clone::CLONE_NEWUSER
                | crate::kernel::clone::CLONE_NEWPID
                | crate::kernel::clone::CLONE_NEWNS
                | crate::kernel::clone::CLONE_NEWNET
                | crate::kernel::clone::CLONE_NEWIPC
                | crate::kernel::clone::CLONE_NEWUTS
                | crate::kernel::clone::CLONE_NEWCGROUP)
    } else {
        0
    };
    if namespace_clone_flags != 0 {
        let namespace_seq =
            TRACE_FIREFOX_NAMESPACE_CLONE_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
        if namespace_seq < 64 {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-firefox-namespace-clone seq={} pid={} nr={} flags={:#x} a0={:#x} a1={:#x}",
                namespace_seq,
                unsafe { (*task).pid },
                nr,
                namespace_clone_flags,
                regs.arg0(),
                regs.arg1()
            );
        }
    }
    // Memory-management probes are useful only when they fail.  Do not print
    // the successful mmap/mprotect/munmap stream: Firefox performs thousands
    // of those during startup and the serial traffic changes the scheduler
    // boundary being investigated.
    if !spawn_child && matches!(nr, 9 | 10 | 11 | 12 | 28) {
        return;
    }
    // Successful pathname probes are not relevant to the process-boundary
    // question and can dominate the serial stream during Firefox startup.
    if !spawn_child && nr == 257 {
        return;
    }
    // A forked-but-not-yet-exec'd spawn child inherits the forking thread's
    // comm and becomes its own thread-group leader. Those few syscalls are the
    // entire question when a loader spawn hangs before execve(), and the
    // shared per-category budgets are exhausted by the parent's ordinary futex
    // traffic long before the interesting child ever runs -- measured: the one
    // child that never exec'd produced *no* trace lines at all. Give it its
    // own budget so it cannot be starved by unrelated events.
    // Everything that is not a spawn child is restricted to process-lifecycle
    // syscalls. Tracing the Firefox thread group's full futex/poll/recvmsg
    // stream produced ~10 000 serial lines and slowed the guest enough that
    // the graphics gate timed out before the probe finished -- the same
    // perturbation trap this file documents for `lupos.trace=syscall`. The
    // spawn child's own syscalls are the question; its parent's matter only at
    // the clone/exec boundary.
    if !spawn_child && !matches!(nr, 56 | 57 | 58 | 59 | 231 | 322 | 435) {
        return;
    }
    if spawn_child {
        if TRACE_FIREFOX_SPAWN_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel) >= 4000 {
            return;
        }
    } else if !firefox_trace_event_allowed(nr) {
        return;
    }
    // The global cap must not silence spawn children. Firefox starts late in a
    // graphics run, so by the time *its* glycin loaders fork, 6000 unrelated
    // events have already been spent and the interesting children print
    // nothing — which is why earlier captures only ever showed loader forks
    // from other processes (their reads of the per-process sample address were
    // plain text, e.g. 0x6361623a646c6968 == "hild:cac"). Spawn children have
    // their own bounded budget already.
    let seq = TRACE_FIREFOX_SYSCALL_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    if seq >= 6_000 && !spawn_child {
        return;
    }
    let pid = unsafe { (*task).pid };
    let tgid = unsafe { (*task).tgid };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-firefox-enter seq={} pid={} tgid={} comm={} nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x}",
        seq,
        pid,
        tgid,
        comm_for_trace(comm),
        nr,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3(),
        regs.arg4(),
        regs.arg5()
    );
    trace_firefox_scheduler_state(task, seq, "enter");
    if nr == 435 && regs.arg1() <= 4096 && regs.arg0() != 0 {
        let clone_args = unsafe { *(regs.arg0() as *const crate::kernel::clone::CloneArgs) };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-firefox-clone3-args pid={} flags={:#x} stack={:#x} stack-size={:#x} tls={:#x} pidfd={:#x} child-tid={:#x} parent-tid={:#x} size={}",
            pid,
            clone_args.flags,
            clone_args.stack,
            clone_args.stack_size,
            clone_args.tls,
            clone_args.pidfd,
            clone_args.child_tid,
            clone_args.parent_tid,
            regs.arg1()
        );
    }
    if matches!(nr, 59 | 322) {
        let path_ptr = if nr == 322 { regs.arg1() } else { regs.arg0() };
        trace_firefox_user_path(pid, nr, path_ptr);
    }
    // A glycin loader spawn is `fork()` from a `blocking-N`/`gly-*` thread, and
    // whether the child survives is decided entirely by whether some *other*
    // thread of this process held an allocator lock at this instant. Dump the
    // siblings' blocked syscalls right here: a holder parked in a kernel path
    // is a kernel-side contribution we can fix, whereas holders that are merely
    // running mean the window is pure userspace.
    if nr == 56 && firefox_trace_spawn_child_comm(comm) {
        trace_firefox_sibling_threads(task, pid);
    }
    // The stuck spawn child blocks on a futex immediately after one
    // `openat(AT_FDCWD, ..., O_CLOEXEC)`, and the healthy child makes the same
    // call and proceeds. Naming that file names the code path holding the
    // lock, so capture it for spawn children only (their `openat` traffic is a
    // handful of calls, unlike the parent's).
    // Decisive discriminator for the glycin loader deadlock. The child blocks
    // in FUTEX_WAIT on an inherited lock word that reads 2. Two explanations
    // remain and they have opposite fixes:
    //
    //   * glibc's fork() child-side reset never ran, so the child inherited a
    //     held lock -- a userspace fork-safety problem; or
    //   * the reset *did* run and the child's store was lost -- a kernel COW
    //     defect.
    //
    // The page tables tell them apart without any userspace cooperation: a
    // store by the child would have taken a write fault and left this page as
    // a private, writable copy. Still-write-protected with the page shared
    // means the child never wrote to it at all.
    if spawn_child && nr == 202 {
        trace_firefox_user_pte(pid, regs.arg0());
        trace_firefox_user_word(pid, "futex", regs.arg0());
    }
    // Two samples of the contended lock word bracket glibc's child-side reset:
    //
    //   nr=273 set_robust_list -- inside `_Fork()`, BEFORE
    //          `__run_fork_handlers(atfork_run_child)`. A non-zero reading here
    //          is expected either way and proves nothing on its own.
    //   nr=14  rt_sigprocmask  -- the first syscall *after* those handlers ran.
    //
    // Still non-zero at nr=14 means the reset did not clear it, so the lock is
    // not one glibc owns (userspace fork-safety bug). Zero at nr=14 but 2 at
    // the futex would mean the reset landed and the value was lost afterwards,
    // which in a single-threaded child could only be a kernel defect.
    //
    // Diagnostic constant: this address has been byte-identical across every
    // captured run. Removed once the question is settled.
    // Only Firefox's own children: the sampled address is a per-process
    // location, and another app's glycin loader reads plain text there
    // (observed: 0x6361623a646c6968 == "hild:cac"), which is meaningless noise.
    let firefox_spawn_child = spawn_child
        && unsafe {
            let parent = (*task).m26.real_parent;
            !parent.is_null()
                && (*parent).tgid == TRACE_FIREFOX_TGID.load(core::sync::atomic::Ordering::Acquire)
        };
    if firefox_spawn_child && matches!(nr, 273 | 14 | 257) {
        let tag = match nr {
            273 => "pre-atfork",
            14 => "post-atfork",
            _ => "pre-openat",
        };
        trace_firefox_user_word(pid, tag, 0x10000700038);
    }
    if spawn_child && matches!(nr, 257 | 2) {
        let path_ptr = if nr == 257 { regs.arg1() } else { regs.arg0() };
        trace_firefox_user_path(pid, nr, path_ptr);
    }
}

#[cfg(test)]
fn trace_firefox_syscall_enter(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
fn trace_firefox_syscall_exit(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    ret: i64,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if !firefox_trace_task(task) {
        return;
    }
    let comm = unsafe { &(*task).comm };
    let nr = regs.orig_rax;
    if !firefox_trace_syscall_is_interesting(nr, ret) {
        return;
    }
    if matches!(nr, 9 | 10 | 11 | 12 | 28) && ret >= 0 {
        return;
    }
    if nr == 257 && ret >= 0 {
        return;
    }
    if !firefox_trace_event_allowed(nr) {
        return;
    }
    let seq = TRACE_FIREFOX_SYSCALL_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    if seq >= 6_000 {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-firefox-exit seq={} pid={} tgid={} comm={} nr={} ret={}",
        seq,
        unsafe { (*task).pid },
        unsafe { (*task).tgid },
        comm_for_trace(comm),
        nr,
        ret
    );
    trace_firefox_scheduler_state(task, seq, "exit");
}

#[cfg(test)]
fn trace_firefox_syscall_exit(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _ret: i64,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
fn firefox_trace_comm(comm: &[u8; 16]) -> bool {
    comm_starts_with(comm, b"firefox")
        || comm_starts_with(comm, b"glxtest")
        || comm_starts_with(comm, b"Socket Process")
        || comm_starts_with(comm, b"forkserver")
        || comm_starts_with(comm, b"Web Content")
        || comm_starts_with(comm, b"RDD Process")
        || comm_starts_with(comm, b"GPU Process")
        || comm_starts_with(comm, b"Utility")
        || comm_starts_with(comm, b"crashhelper")
        // glycin spawns its sandboxed image loader from these threads, and
        // that spawn is the one that never reaches execve(). They are threads
        // of the Firefox tgid, but keep them here too so a loader that already
        // left the group is still traced.
        || comm_starts_with(comm, b"blocking-")
        || comm_starts_with(comm, b"gly-")
        || comm_starts_with(comm, b"glycin")
        || comm_starts_with(comm, b"bwrap")
}

/// Trace the whole Firefox thread group once its leader has been observed.
/// X11 requests commonly run on `IPC I/O Parent`, `Compositor`, or `gmain`
/// rather than the leader named `firefox`; filtering only by `comm` therefore
/// made a missing-window replay look like a syscall-free user-mode stall.
#[cfg(not(test))]
fn firefox_trace_task(task: *mut crate::kernel::task::TaskStruct) -> bool {
    if task.is_null() || !crate::kernel::debug_trace::firefox_enabled() {
        return false;
    }
    let comm = unsafe { &(*task).comm };
    let tgid = unsafe { (*task).tgid };
    if comm_starts_with(comm, b"firefox") {
        let _ = TRACE_FIREFOX_TGID.compare_exchange(
            -1,
            tgid,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        );
        return true;
    }
    firefox_trace_comm(comm)
        || TRACE_FIREFOX_TGID.load(core::sync::atomic::Ordering::Acquire) == tgid
}

fn firefox_trace_syscall_is_interesting(nr: u64, ret: i64) -> bool {
    let _ = ret;
    // Keep this replay bounded enough that serial output does not change the
    // scheduler boundary being investigated.  Process creation plus the
    // blocking wait boundaries identify the first unmatched Firefox wait.
    matches!(
        nr,
        7     // poll
        | 23  // select
        | 202 // futex
        | 232 // epoll_wait
        | 270 // pselect6
        | 281 // epoll_pwait
        | 441 // epoll_pwait2
        | 20  // writev: X11 request framing
        | 45  // recvfrom: X11 reply/event reads
        | 46  // sendmsg: Unix IPC writes
        | 47  // recvmsg: X11 reply/event reads
        | 56  // clone
        | 57  // fork
        | 58  // vfork
        | 59  // execve
        | 231 // exit_group
        | 322 // execveat
        | 435 // clone3
    )
}

#[cfg(not(test))]
fn firefox_trace_event_allowed(nr: u64) -> bool {
    let (counter, limit) = match nr {
        7 | 23 | 35 | 230 | 232 | 233 | 234 | 270 | 271 | 281 | 441 => {
            (&TRACE_FIREFOX_POLL_COUNT, 120)
        }
        202 => (&TRACE_FIREFOX_FUTEX_COUNT, 400),
        435 => (&TRACE_FIREFOX_CLONE_COUNT, 120),
        20 | 45..=47 => (&TRACE_FIREFOX_RECVMSG_COUNT, 160),
        _ => return true,
    };
    counter.fetch_add(1, core::sync::atomic::Ordering::AcqRel) < limit
}

/// Capture scheduler ownership at the Firefox syscall boundary without
/// taking an rq lock.  The diagnostic is intentionally read-only: a Firefox
/// process reported `R` while all four CPUs were idle in the no-window replay,
/// so the relevant distinction is TASK_RUNNING versus actual runqueue/current
/// ownership.  The fields mirror Linux's task_struct::on_cpu/on_rq and
/// sched_entity::on_rq checks used by `try_to_wake_up()` and `__schedule()`.
#[cfg(not(test))]
fn trace_firefox_scheduler_state(
    task: *mut crate::kernel::task::TaskStruct,
    seq: u32,
    point: &str,
) {
    if task.is_null() {
        return;
    }
    let cpu = crate::kernel::sched::current_cpu();
    let current = unsafe { crate::kernel::sched::get_current() };
    let state = unsafe { (*task).__state.load(core::sync::atomic::Ordering::Acquire) };
    let on_cpu = unsafe {
        (*task)
            .m29
            .on_cpu
            .load(core::sync::atomic::Ordering::Acquire)
    };
    let on_rq = unsafe { (*task).m29.on_rq };
    let se_on_rq = unsafe { (*task).m29.se.on_rq };
    let task_cpu = unsafe { (*task).thread_info.cpu };
    let current_pid = if current.is_null() {
        -1
    } else {
        unsafe { (*current).pid }
    };
    let current_match = u8::from(current == task);
    let need_resched = u8::from(unsafe {
        (*task)
            .thread_info
            .flags
            .load(core::sync::atomic::Ordering::Acquire)
            & crate::kernel::task::TIF_NEED_RESCHED
            != 0
    });
    let rq_running = crate::kernel::sched::rq::rq_nr_running(cpu).unwrap_or(u32::MAX);
    crate::linux_driver_abi::tty::serial_println!(
        "trace-firefox-state seq={} point={} pid={} cpu={} task-cpu={} state={:#x} on-cpu={} on-rq={} se-on-rq={} current-pid={} current-match={} need-resched={} rq-running={}",
        seq,
        point,
        unsafe { (*task).pid },
        cpu,
        task_cpu,
        state,
        on_cpu,
        on_rq,
        se_on_rq,
        current_pid,
        current_match,
        need_resched,
        rq_running,
    );
}

/// Report the PTE backing a user address: whether it is present, writable
/// (i.e. already COW-copied for this task) and how many mappings share it.
#[cfg(not(test))]
fn trace_firefox_user_pte(pid: i32, addr: u64) {
    use crate::arch::x86::mm::paging::{
        p4d_offset, pgd_offset_pgd, pmd_offset, pte_offset_kernel, pud_offset,
    };
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return;
    }
    let pgd = unsafe { (*mm).pgd } as *mut crate::arch::x86::mm::paging::pgd_t;
    if pgd.is_null() {
        return;
    }
    unsafe {
        let pgdp = pgd_offset_pgd(pgd, addr);
        if crate::arch::x86::mm::paging::pgd_none(*pgdp) {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-firefox-pte pid={} addr={:#x} pgd=none",
                pid,
                addr
            );
            return;
        }
        let pudp = pud_offset(p4d_offset(pgdp, addr), addr);
        if crate::arch::x86::mm::paging::pud_none(*pudp) {
            return;
        }
        let pmdp = pmd_offset(pudp, addr);
        if crate::arch::x86::mm::paging::pmd_none(*pmdp) {
            return;
        }
        let ptep = pte_offset_kernel(pmdp, addr);
        let pte = *ptep;
        let present = crate::arch::x86::mm::paging::pte_present(pte);
        let writable = crate::arch::x86::mm::paging::pte_write(pte);
        let pfn = crate::arch::x86::mm::paging::pte_pfn(pte) as usize;
        let (refcount, mapcount) = if present && crate::mm::buddy::pfn_valid(pfn) {
            let page = crate::mm::buddy::pfn_to_page(pfn);
            if page.is_null() {
                (-1i64, -1i64)
            } else {
                (
                    (*page).refcount() as i64,
                    (*page)
                        ._mapcount()
                        .load(core::sync::atomic::Ordering::Relaxed) as i64,
                )
            }
        } else {
            (-1i64, -1i64)
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-firefox-pte pid={} addr={:#x} present={} writable={} pfn={:#x} refcount={} mapcount={}",
            pid,
            addr,
            present as u8,
            writable as u8,
            pfn,
            refcount,
            mapcount
        );
    }
}

/// Read one 64-bit user word through the fault-tolerant accessor and log it.
#[cfg(not(test))]
fn trace_firefox_user_word(pid: i32, tag: &str, addr: u64) {
    match unsafe { crate::arch::x86::kernel::uaccess::get_user_u64(addr as *const u64) } {
        Ok(value) => crate::linux_driver_abi::tty::serial_println!(
            "trace-firefox-word pid={} at={} addr={:#x} value={:#x}",
            pid,
            tag,
            addr,
            value
        ),
        Err(errno) => crate::linux_driver_abi::tty::serial_println!(
            "trace-firefox-word pid={} at={} addr={:#x} unreadable={}",
            pid,
            tag,
            addr,
            errno
        ),
    }
}

/// Dump every other thread of this task's process with the syscall it is
/// currently blocked in, using the same saved frame `/proc/<pid>/syscall` reads.
#[cfg(not(test))]
fn trace_firefox_sibling_threads(task: *mut crate::kernel::task::TaskStruct, pid: i32) {
    let tgid = unsafe { (*task).tgid };
    let mut reported = 0u32;
    let mut running = 0u32;
    crate::kernel::fork::for_each_heap_task(&mut |other: *mut crate::kernel::task::TaskStruct| {
        if other.is_null() || other == task || reported >= 24 {
            return;
        }
        if unsafe { (*other).tgid } != tgid {
            return;
        }
        if crate::kernel::sched::task_on_cpu(other) {
            running += 1;
            return;
        }
        let regs = unsafe { crate::fs::proc::base::task_pt_regs(other) };
        if regs.is_null() {
            return;
        }
        reported += 1;
        crate::linux_driver_abi::tty::serial_println!(
            "trace-firefox-sibling forker={} tid={} comm={} nr={} ip={:#x}",
            pid,
            unsafe { (*other).pid },
            comm_for_trace(unsafe { &(*other).comm }),
            unsafe { (*regs).orig_ax } as i64,
            unsafe { (*regs).ip }
        );
    });
    crate::linux_driver_abi::tty::serial_println!(
        "trace-firefox-sibling-summary forker={} blocked={} on_cpu={}",
        pid,
        reported,
        running
    );
}

#[cfg(not(test))]
fn trace_firefox_user_path(pid: i32, nr: u64, ptr: u64) {
    let mut bytes = [0u8; 192];
    let copied = unsafe {
        crate::arch::x86::kernel::uaccess::strncpy_from_user(
            bytes.as_mut_ptr(),
            ptr as *const u8,
            bytes.len(),
        )
    };
    if copied < 0 {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-firefox-path pid={} nr={} path=<fault:{}>",
            pid,
            nr,
            copied
        );
        return;
    }
    let len = (copied as usize).min(bytes.len());
    let path = core::str::from_utf8(&bytes[..len]).unwrap_or("<non-utf8>");
    crate::linux_driver_abi::tty::serial_println!(
        "trace-firefox-path pid={} nr={} path={}",
        pid,
        nr,
        path
    );
}

#[cfg(test)]
fn trace_bwrap_syscall_enter(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
static TRACE_XFCE_SYSCALL_ENTER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

/// Trace the blocking/IPC/process syscalls involved in the stock Xfce
/// session's private D-Bus bootstrap.  This is deliberately behind the same
/// opt-in image-loader diagnostic flag used by the graphical gate: normal
/// boots pay only the flag/comm checks, and vendor programs/configuration are
/// never wrapped or changed to obtain the trace.
#[cfg(not(test))]
fn trace_xfce_syscall_enter(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null() || !crate::kernel::debug_trace::glycin_enabled() {
        return;
    }
    // Keep this diagnostic scoped to the authenticated desktop session.  A
    // root-owned standalone D-Bus probe uses the same executable names and
    // otherwise consumes the trace budget before the real session reaches
    // its daemonisation path.
    if task_euid_for_trace(task) != Some(1000) {
        return;
    }
    let comm = unsafe { &(*task).comm };
    if !comm_starts_with(comm, b"xfce4-session")
        && !comm_starts_with(comm, b"dbus-launch")
        && !comm_starts_with(comm, b"dbus-daemon")
    {
        return;
    }
    let nr = regs.orig_rax;
    if !matches!(
        nr,
        0 | 1 | 3 | 7 | 13 | 14 | 16 | 21 | 23 | 32 | 33 | 41..=62 | 72 | 109..=126
            | 202 | 217 | 231..=234 | 247 | 257 | 262 | 270 | 271 | 281 | 288
            | 290..=293 | 302 | 318 | 322 | 435 | 441
    ) {
        return;
    }
    let count = TRACE_XFCE_SYSCALL_ENTER_COUNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    if count >= 8_000 {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-xfce-sys-enter seq={} pid={} tgid={} comm={} nr={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x}",
        count,
        unsafe { (*task).pid },
        unsafe { (*task).tgid },
        comm_for_trace(comm),
        nr,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3(),
        regs.arg4(),
        regs.arg5()
    );

    let path_ptr = match nr {
        21 => regs.arg0(),        // access(2)
        257 | 262 => regs.arg1(), // openat/newfstatat
        _ => 0,
    };
    if path_ptr != 0 {
        trace_xfce_user_path(unsafe { (*task).pid }, nr, path_ptr);
    }
}

#[cfg(not(test))]
fn task_euid_for_trace(task: *mut crate::kernel::task::TaskStruct) -> Option<u32> {
    if task.is_null() {
        return None;
    }
    let cred = unsafe { (*task).cred };
    if cred.is_null() {
        None
    } else {
        Some(unsafe { (*cred).euid.0 })
    }
}

#[cfg(not(test))]
fn trace_xfce_user_path(pid: i32, nr: u64, ptr: u64) {
    let mut bytes = [0u8; 192];
    let copied = unsafe {
        crate::arch::x86::kernel::uaccess::strncpy_from_user(
            bytes.as_mut_ptr(),
            ptr as *const u8,
            bytes.len(),
        )
    };
    if copied < 0 {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-xfce-path pid={} nr={} path=<fault:{}>",
            pid,
            nr,
            copied
        );
        return;
    }
    let len = (copied as usize).min(bytes.len());
    let path = core::str::from_utf8(&bytes[..len]).unwrap_or("<non-utf8>");
    crate::linux_driver_abi::tty::serial_println!(
        "trace-xfce-path pid={} nr={} path={}",
        pid,
        nr,
        path
    );
}

#[cfg(test)]
fn trace_xfce_syscall_enter(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
fn trace_systemd_service_syscall(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    ret: i64,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null() {
        return;
    }
    let comm = unsafe { &(*task).comm };
    let pid = unsafe { (*task).pid };
    let syscall_trace = crate::kernel::debug_trace::syscall_enabled();
    let proc_trace = crate::kernel::debug_trace::proc_enabled();
    let ping_trace = crate::kernel::debug_trace::ping_enabled();
    let systemctl_trace = crate::kernel::debug_trace::systemctl_enabled();
    let glycin_trace = crate::kernel::debug_trace::glycin_enabled();
    let pixbuf_trace = crate::kernel::debug_trace::pixbuf_enabled();
    if !syscall_trace
        && !proc_trace
        && !ping_trace
        && !systemctl_trace
        && !glycin_trace
        && !pixbuf_trace
    {
        return;
    }
    let trace_pid1 = syscall_trace && pid == 1;
    let trace_systemd_service = syscall_trace && comm_starts_with(comm, b"systemd-");
    let trace_dbus_broker = syscall_trace && comm_starts_with(comm, b"dbus-broker");
    let trace_systemctl =
        (syscall_trace || systemctl_trace) && comm_starts_with(comm, b"systemctl");
    let trace_dbus = systemctl_trace && comm_starts_with(comm, b"dbus-daemon");
    let trace_ping = ping_trace
        && (comm_starts_with(comm, b"ping") || crate::kernel::debug_trace::ping_pid_matches(pid));
    let trace_desktop_session = task_euid_for_trace(task) == Some(1000)
        && (comm_starts_with(comm, b"xfce4-session")
            || comm_starts_with(comm, b"dbus-launch")
            || comm_starts_with(comm, b"dbus-daemon"));
    let trace_user_manager = proc_trace
        && task_euid_for_trace(task) == Some(1000)
        && (comm_starts_with(comm, b"systemd") || comm_starts_with(comm, b"dbus-broker"));
    let trace_glycin = glycin_trace
        && (comm_starts_with(comm, b"glycin")
            || comm_starts_with(comm, b"bwrap")
            || comm_starts_with(comm, b"glycin-image")
            || comm_starts_with(comm, b"lightdm-gtk-gre")
            || trace_desktop_session
            || trace_user_manager);
    // Narrow bridge diagnostic: gdk-pixbuf-pixdata/csource drive the
    // in-process gdk-pixbuf→glycin bridge in the graphics probe.  Trace only
    // their failing syscalls plus readlink/readlinkat so the serial console
    // is not flooded the way the full `glycin` flag floods it.
    let trace_pixbuf = pixbuf_trace
        && comm_starts_with(comm, b"gdk-pixbuf")
        && (ret < 0 || matches!(regs.orig_rax, 89 | 267));
    if !trace_pid1
        && !trace_systemd_service
        && !trace_dbus_broker
        && !trace_systemctl
        && !trace_dbus
        && !trace_ping
        && !trace_user_manager
        && !trace_glycin
        && !trace_pixbuf
    {
        return;
    }
    let nr = regs.orig_rax;
    let interesting = trace_service_syscall_is_interesting(
        nr,
        ret,
        trace_pid1,
        trace_ping,
        trace_systemctl || trace_dbus,
    );
    if !interesting
        && !(trace_glycin && (ret < 0 || comm_starts_with(comm, b"bwrap")))
        && !trace_pixbuf
    {
        return;
    }
    crate::linux_driver_abi::tty::serial_println!(
        "trace-svc-sys pid={} comm={} nr={} ret={} a0={:#x} a1={:#x} a2={:#x} a3={:#x} a4={:#x} a5={:#x}",
        pid,
        comm_for_trace(comm),
        nr,
        ret,
        regs.arg0(),
        regs.arg1(),
        regs.arg2(),
        regs.arg3(),
        regs.arg4(),
        regs.arg5()
    );
}

fn trace_service_syscall_is_interesting(
    nr: u64,
    ret: i64,
    trace_pid1: bool,
    trace_ping: bool,
    trace_systemctl: bool,
) -> bool {
    trace_pid1
        || ret < 0
        || matches!(
            nr,
            41..=55
                | 72
                | 116..=126
                | 157
                | 165
                | 166
                | 232
                | 233
                | 259
                | 272
                | 281
                | 288
                | 291
                | 321
                | 441
        )
        || (trace_ping
            && matches!(
                nr,
                0 | 1 | 7 | 13 | 14 | 15 | 35 | 37 | 38 | 41..=55 | 230 | 271 | 283 | 286 | 287
            ))
        || (trace_systemctl
            && matches!(
                nr,
                0 | 1 | 7 | 23 | 41..=55 | 157 | 232 | 233 | 270 | 271 | 281 | 291 | 441
            ))
}

#[cfg(test)]
fn trace_systemd_service_syscall(
    _regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    _ret: i64,
    _task: *mut crate::kernel::task::TaskStruct,
) {
}

#[cfg(not(test))]
fn comm_starts_with(comm: &[u8; 16], prefix: &[u8]) -> bool {
    comm.len() >= prefix.len() && &comm[..prefix.len()] == prefix
}

#[cfg(not(test))]
fn comm_for_trace(comm: &[u8; 16]) -> &str {
    let len = comm.iter().position(|b| *b == 0).unwrap_or(comm.len());
    core::str::from_utf8(&comm[..len]).unwrap_or("?")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SeccompCheck {
    Allow,
    Errno(i64),
    Trap(u16),
}

const SYS_SECCOMP: i32 = 1;

pub(crate) unsafe fn queue_seccomp_trap(
    regs: &mut crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
    data: u16,
) {
    regs.rax = regs.orig_rax;
    if task.is_null() {
        return;
    }
    let info = crate::kernel::signal::SigInfo::with_sigsys(
        crate::kernel::signal::SIGSYS,
        SYS_SECCOMP,
        regs.rip,
        regs.orig_rax as i32,
        AUDIT_ARCH_X86_64,
        data as i32,
    );
    let _ = unsafe { crate::kernel::signal::send_signal_info_to_task(task, info) };
}

fn syscall_seccomp_check(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) -> SeccompCheck {
    if task.is_null() {
        return SeccompCheck::Allow;
    }

    let seccomp = unsafe { &(*task).m27_seccomp };
    let check = syscall_seccomp_check_state(regs, seccomp);
    #[cfg(not(test))]
    trace_seccomp_decision(regs, task, seccomp, check);
    check
}

pub(crate) fn syscall_seccomp_check_state(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    seccomp: &Seccomp,
) -> SeccompCheck {
    if seccomp.mode.load(core::sync::atomic::Ordering::Acquire) == SECCOMP_MODE_STRICT
        && !strict_seccomp_allows(regs.orig_rax)
    {
        return SeccompCheck::Errno(-EPERM);
    }

    let data = SeccompData {
        nr: regs.orig_rax as i32,
        arch: AUDIT_ARCH_X86_64,
        instruction_pointer: regs.rip,
        args: [regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9],
    };
    seccomp_action_to_result(seccomp_run_filters(seccomp, &data))
}

fn strict_seccomp_allows(nr: u64) -> bool {
    matches!(nr, 0 | 1 | SYS_EXIT | SYS_RT_SIGRETURN)
}

fn seccomp_action_to_result(action: u32) -> SeccompCheck {
    match action & SECCOMP_RET_ACTION_FULL {
        SECCOMP_RET_ALLOW | SECCOMP_RET_LOG => SeccompCheck::Allow,
        SECCOMP_RET_ERRNO => SeccompCheck::Errno(-((action & SECCOMP_RET_DATA) as i64)),
        SECCOMP_RET_TRAP => SeccompCheck::Trap((action & SECCOMP_RET_DATA) as u16),
        SECCOMP_RET_TRACE | SECCOMP_RET_USER_NOTIF => SeccompCheck::Errno(-ENOSYS),
        SECCOMP_RET_KILL_THREAD | SECCOMP_RET_KILL_PROCESS => SeccompCheck::Errno(-EPERM),
        _ => SeccompCheck::Errno(-EPERM),
    }
}

#[cfg(not(test))]
fn trace_seccomp_decision(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
    seccomp: &Seccomp,
    check: SeccompCheck,
) {
    if !crate::kernel::debug_trace::seccomp_enabled() || task.is_null() {
        return;
    }
    let nr = regs.orig_rax as usize;
    let comm = unsafe { &(*task).comm };
    // Firefox installs filters in its browser/content processes. Keep the
    // trace focused on the control plane, process creation, affinity, and
    // any syscall whose filter decision is already denying it.
    let firefox_process = comm_starts_with(comm, b"firefox")
        || comm_starts_with(comm, b"bwrap")
        || comm_starts_with(comm, b"glycin");
    if !firefox_process {
        return;
    }
    if !matches!(nr, 157 | 202 | 204 | 231 | 317 | 435) && check == SeccompCheck::Allow {
        return;
    }
    let mode = seccomp.mode.load(core::sync::atomic::Ordering::Acquire);
    let action = if mode == SECCOMP_MODE_FILTER {
        let data = SeccompData {
            nr: regs.orig_rax as i32,
            arch: AUDIT_ARCH_X86_64,
            instruction_pointer: regs.rip,
            args: [regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9],
        };
        seccomp_run_filters(seccomp, &data)
    } else {
        SECCOMP_RET_ALLOW
    };
    let pid = unsafe { (*task).pid };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-seccomp pid={} comm={} nr={} mode={} filter={} action={:#x} decision={:?}",
        pid,
        comm_for_trace(comm),
        nr,
        mode,
        !seccomp
            .filter
            .load(core::sync::atomic::Ordering::Acquire)
            .is_null(),
        action,
        check
    );
}

#[cfg(not(test))]
fn trace_seccomp_control(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
    ret: i64,
) {
    if !crate::kernel::debug_trace::seccomp_enabled() || task.is_null() {
        return;
    }
    if !matches!(regs.orig_rax, 157 | 317) {
        return;
    }
    let comm = unsafe { &(*task).comm };
    if !comm_starts_with(comm, b"firefox")
        && !comm_starts_with(comm, b"bwrap")
        && !comm_starts_with(comm, b"glycin")
    {
        return;
    }
    let pid = unsafe { (*task).pid };
    let mode = unsafe {
        (*task)
            .m27_seccomp
            .mode
            .load(core::sync::atomic::Ordering::Acquire)
    };
    crate::linux_driver_abi::tty::serial_println!(
        "trace-seccomp-control pid={} comm={} nr={} arg0={:#x} arg1={:#x} ret={} mode={}",
        pid,
        comm_for_trace(comm),
        regs.orig_rax,
        regs.arg0(),
        regs.arg1(),
        ret,
        mode
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::kernel::ptrace::PtRegs;
    use crate::kernel::bpf::{BPF_K, BPF_RET, SockFilter};
    use crate::kernel::seccomp::{
        SECCOMP_RET_ERRNO, SeccompFilter, seccomp_attach_filter, seccomp_prepare_filter,
    };
    use crate::kernel::trace::ring_buffer::{TRACE_RING_SIZE, TraceEvent};
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};
    use core::sync::atomic::Ordering;

    fn regs_for_syscall(nr: u64) -> PtRegs {
        PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            r11: 0,
            r10: 4,
            r9: 6,
            r8: 5,
            rax: nr,
            rcx: 0,
            rdx: 3,
            rsi: 2,
            rdi: 1,
            orig_rax: nr,
            rip: 0x400000,
            cs: 0,
            eflags: 0,
            rsp: 0,
            ss: 0,
        }
    }

    #[test]
    fn exec_register_image_matches_x86_64_elf_common_init() {
        let mut regs = PtRegs {
            r15: 15,
            r14: 14,
            r13: 13,
            r12: 12,
            rbp: 6,
            rbx: 3,
            r11: 11,
            r10: 10,
            r9: 9,
            r8: 8,
            rax: 59,
            rcx: 4,
            rdx: 0xdead_beef,
            rsi: 2,
            rdi: 1,
            orig_rax: 59,
            rip: 0x400000,
            cs: 0,
            eflags: 0,
            rsp: 0x7000,
            ss: 0,
        };
        let ctx = UserStartContext {
            ip: 0x5555_5555_7bc0,
            sp: 0x7fff_ffff_f000,
            rflags: 0x202,
            old_mm: 0,
        };

        initialize_exec_registers(&mut regs, &ctx);

        assert_eq!(
            [
                regs.r15, regs.r14, regs.r13, regs.r12, regs.rbp, regs.rbx, regs.r11, regs.r10,
                regs.r9, regs.r8, regs.rax, regs.rcx, regs.rdx, regs.rsi, regs.rdi,
            ],
            [0; 15]
        );
        assert_eq!(regs.orig_rax, 59);
        assert_eq!(regs.rip, ctx.ip);
        assert_eq!(regs.rsp, ctx.sp);
        assert_eq!(regs.eflags, ctx.rflags);
        assert_eq!(regs.cs, sel::USER_CS as u64);
        assert_eq!(regs.ss, sel::USER_DS as u64);
        assert!(!syscall_frame_allows_sysret(&regs));
    }

    #[test]
    fn successful_exec_resets_live_tls_only_at_image_transition() {
        let source = include_str!("syscall.rs");
        let dispatch = source
            .split("unsafe fn syscall_dispatch_ptregs_inner(")
            .nth(1)
            .expect("pt_regs syscall dispatcher must exist")
            .split("/// Install the new image's live FS/GS bases")
            .next()
            .expect("dispatcher must end before exec TLS helper");
        let exec_success = dispatch
            .split("if ret == 0")
            .nth(1)
            .expect("exec transition must require a successful syscall");
        let take_context = exec_success
            .find("take_exec_start_for_current()")
            .expect("successful exec must consume its start context");
        let reset_tls = exec_success
            .find("reset_successful_exec_user_tls_bases(task)")
            .expect("successful exec must install its reset TLS bases");
        let initialize_regs = exec_success
            .find("initialize_exec_registers")
            .expect("successful exec must initialize its user registers");

        assert!(take_context < reset_tls);
        assert!(reset_tls < initialize_regs);
    }

    #[test]
    fn star_msr_value_encodes_correct_selectors() {
        // Verify STAR layout without touching MSRs (host-side test).
        let star: u64 = ((sel::USER32_CS as u64) << 48) | ((sel::KERNEL_CS as u64) << 32);

        let syscall_cs = ((star >> 32) & 0xFFFF) as u16;
        let sysret_base = ((star >> 48) & 0xFFFF) as u16;

        assert_eq!(syscall_cs, sel::KERNEL_CS, "SYSCALL CS selector");
        assert_eq!(sysret_base + 8, sel::USER_DS, "SYSRET SS selector");
        assert_eq!(sysret_base + 16, sel::USER_CS, "SYSRET CS selector");
    }

    #[test]
    fn rflags_mask_covers_if_df_tf() {
        // All three flags must be masked to prevent dangerous re-entrancy.
        assert_ne!(SYSCALL_RFLAGS_MASK & RFLAGS_IF, 0, "IF must be in FMASK");
        assert_ne!(SYSCALL_RFLAGS_MASK & RFLAGS_DF, 0, "DF must be in FMASK");
        assert_ne!(SYSCALL_RFLAGS_MASK & RFLAGS_TF, 0, "TF must be in FMASK");
    }

    #[test]
    fn syscall_entry_reenables_irqs_only_inside_kernel_body() {
        let source = include_str!("syscall.rs");
        let entry = source
            .split("pub unsafe extern \"C\" fn syscall_entry()")
            .nth(1)
            .expect("syscall entry stub must exist")
            .split("pub unsafe extern \"C\" fn syscall_exit_slowpath")
            .next()
            .expect("syscall entry stub must end before exit slowpath");
        assert!(
            !entry.contains("fxsave64") && !entry.contains("fxrstor64"),
            "soft-float syscall entry must not save FPU state per call"
        );
        assert!(
            !entry.contains("[rsp + 512]"),
            "minimal syscall scratch must not retain the removed FXSAVE offset"
        );
        assert!(
            !entry.contains("load_user_tls") && !entry.contains("wrmsr"),
            "syscall return must retain the live FS base; arch_prctl and context switch own updates"
        );
        let frame_complete = entry
            .find("\"mov [rsp], rdi\"")
            .expect("syscall entry must preserve the pt_regs pointer");
        let irq_enable = entry[frame_complete..]
            .find("\"sti\"")
            .map(|off| frame_complete + off)
            .expect("syscall entry must enable IRQs before dispatch");
        let dispatch = entry
            .find("\"call {dispatch_ptregs}\"")
            .expect("syscall entry must call syscall dispatch");
        let exit_decision = entry
            .find("\"call {should_use_sysret}\"")
            .expect("syscall entry must choose SYSRET vs IRET");
        let irq_disable = entry[exit_decision..]
            .find("\"cli\"")
            .map(|off| exit_decision + off)
            .expect("syscall entry must disable IRQs before user restore");
        let restore = entry
            .find("\"mov rsp, [rsp]\"")
            .expect("syscall entry must restore pt_regs stack pointer");

        assert!(frame_complete < irq_enable);
        assert!(irq_enable < dispatch);
        assert!(dispatch < exit_decision);
        assert!(exit_decision < irq_disable);
        assert!(irq_disable < restore);
    }

    #[test]
    fn syscall_entry_uses_linux_cpu_local_scratch_and_current_stack() {
        // test-origin: linux:vendor/linux/arch/x86/entry/entry_64.S:entry_SYSCALL_64
        let source = include_str!("syscall.rs");
        let entry = source
            .split("pub unsafe extern \"C\" fn syscall_entry()")
            .nth(1)
            .expect("syscall entry stub must exist")
            .split("pub unsafe extern \"C\" fn syscall_exit_slowpath")
            .next()
            .expect("syscall entry stub must end before exit slowpath");

        assert!(entry.contains("gs:[rip + {percpu_base} + {user_rsp_offset}]"));
        assert!(entry.contains("gs:[rip + {percpu_base} + {current_top_of_stack_offset}]"));
        assert!(!entry.contains("{syscall_tss_offset}"));
        assert!(!entry.contains("sym crate::arch::x86::kernel::tss::TSS"));
    }

    #[test]
    fn syscall_console_drain_claim_avoids_same_jiffy_exchange() {
        let last = core::sync::atomic::AtomicU64::new(u64::MAX);

        assert!(syscall_console_drain_due(&last, 100));
        assert!(!syscall_console_drain_due(&last, 100));
        assert!(syscall_console_drain_due(&last, 101));
        assert!(!syscall_console_drain_due(&last, 101));
    }

    #[test]
    fn enter_userspace_disables_irqs_before_user_rsp_switch() {
        let source = include_str!("syscall.rs");
        let trampoline = source
            .split("pub unsafe extern \"C\" fn enter_userspace(ctx: &UserStartContext) -> !")
            .nth(1)
            .expect("enter_userspace trampoline must exist");
        let irq_disable = trampoline
            .find("\"cli\"")
            .expect("enter_userspace must disable IRQs before SYSRET restore");
        let user_rsp_load = trampoline
            .find("\"mov rsp, [rdi + 8]\"")
            .expect("enter_userspace must load user RSP before SYSRET");
        let swapgs = trampoline
            .find("\"swapgs\"")
            .expect("enter_userspace must swap to user GS before SYSRET");
        let sysret = trampoline
            .find("\"sysretq\"")
            .expect("enter_userspace must return through SYSRET");

        assert!(irq_disable < user_rsp_load);
        assert!(user_rsp_load < swapgs);
        assert!(swapgs < sysret);
    }

    #[test]
    fn syscall_sysret_fast_path_requires_clean_linux_frame() {
        let mut regs = regs_for_syscall(39);
        regs.rcx = regs.rip;
        regs.r11 = regs.eflags;
        regs.cs = sel::USER_CS as u64;
        regs.ss = sel::USER_DS as u64;
        assert!(syscall_frame_allows_sysret(&regs));

        regs.rip = 0x401000;
        assert!(!syscall_frame_allows_sysret(&regs));
        regs.rcx = regs.rip;
        regs.r11 = regs.eflags ^ 0x40;
        assert!(!syscall_frame_allows_sysret(&regs));
        regs.r11 = regs.eflags;
        regs.rip = crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX;
        regs.rcx = regs.rip;
        assert!(!syscall_frame_allows_sysret(&regs));
    }

    #[test]
    fn syscall_exit_sanitizes_invalid_user_rflags() {
        let mut regs = regs_for_syscall(39);
        regs.eflags = 0x1;
        regs.r11 = 0x1;

        sanitize_syscall_user_rflags(&mut regs);

        assert_ne!(regs.eflags & RFLAGS_FIXED, 0);
        assert_ne!(regs.eflags & RFLAGS_IF, 0);
        assert_eq!(regs.r11, regs.eflags);
    }

    #[test]
    fn syscall_sysret_fast_path_rejects_signal_frames() {
        let mut regs = regs_for_syscall(39);
        regs.rcx = 0x401000;
        regs.rip = 0x700000;
        regs.r11 = 0x202;
        regs.eflags = 0x202;
        regs.cs = sel::USER_CS as u64;
        regs.ss = sel::USER_DS as u64;
        assert!(
            !syscall_frame_allows_sysret(&regs),
            "signal delivery changes RIP to the handler while RCX remains user state"
        );

        regs.rcx = regs.rip;
        regs.eflags |= crate::arch::x86::kernel::ptrace::X86_EFLAGS_RF;
        regs.r11 = regs.eflags;
        assert!(!syscall_frame_allows_sysret(&regs));
    }

    #[test]
    fn syscall_exit_slowpath_delivers_user_signal_frame() {
        let previous = unsafe { sched::get_current() };
        crate::kernel::signal::reset_for_tests();
        let mut task = unsafe { core::mem::zeroed::<TaskStruct>() };
        task.pid = 8100;
        task.tgid = 8100;
        task.cred = &raw const INIT_CRED;
        let mut stack = [0u8; 4096];
        let stack_top = unsafe { stack.as_mut_ptr().add(stack.len()) as u64 };
        let mut regs = regs_for_syscall(39);
        regs.rip = 0x401000;
        regs.rsp = stack_top;
        regs.cs = sel::USER_CS as u64;
        regs.ss = sel::USER_DS as u64;

        unsafe {
            sched::set_current(&mut task);
            let action = crate::kernel::signal::RtSigAction {
                sa_handler: 0x1234,
                sa_flags: crate::kernel::signal::SA_SIGINFO,
                sa_restorer: 0x5678,
                sa_mask: crate::kernel::signal::SigSet::default(),
            };
            assert_eq!(
                crate::kernel::signal::sys_rt_sigaction(
                    crate::kernel::signal::SIGCHLD,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<crate::kernel::signal::SigSet>(),
                ),
                0
            );
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut task as *mut TaskStruct,
                    crate::kernel::signal::SIGCHLD,
                ),
                0
            );

            syscall_exit_slowpath(&mut regs);

            assert_eq!(regs.rip, action.sa_handler as u64);
            assert_eq!(regs.rdi, crate::kernel::signal::SIGCHLD as u64);
            assert!(regs.rsp < stack_top);
            assert!(regs.rsi >= regs.rsp);
            assert!(regs.rdx >= regs.rsp);

            crate::kernel::signal::reset_for_tests();
            sched::set_current(previous);
        }
    }

    #[test]
    fn efer_sce_is_bit0() {
        // Linux and AMD64 APM both define SCE as EFER bit 0.
        assert_eq!(EFER_SCE, 1, "EFER.SCE must be bit 0");
    }

    #[test]
    fn syscall_trace_filter_includes_ping_wait_and_timer_calls() {
        assert!(trace_service_syscall_is_interesting(
            47, 0, false, true, false
        ));
        assert!(trace_service_syscall_is_interesting(
            271, 0, false, true, false
        ));
        assert!(trace_service_syscall_is_interesting(
            38, 0, false, true, false
        ));
        assert!(!trace_service_syscall_is_interesting(
            39, 0, false, true, false
        ));
    }

    #[test]
    fn syscall_trace_filter_includes_systemctl_poll_waits() {
        assert!(trace_service_syscall_is_interesting(
            47, 0, false, false, true
        ));
        assert!(trace_service_syscall_is_interesting(
            271, 0, false, false, true
        ));
        assert!(trace_service_syscall_is_interesting(
            232, 0, false, false, true
        ));
    }

    #[test]
    fn dispatch_records_audit_and_trace_for_syscall() {
        let _audit_guard = audit::test_lock();
        audit::reset_for_test();
        audit::audit_add_rule(audit::AuditRule {
            syscall_nr: 9999,
            pid: -1,
        });

        TRACE_RB.set_enabled(true);
        let mut drained = [TraceEvent::empty(); TRACE_RING_SIZE];
        let _ = TRACE_RB.drain(&mut drained);

        let mut regs = regs_for_syscall(9999);
        let ret = unsafe { syscall_dispatch_ptregs_inner(&mut regs as *mut PtRegs) };

        assert_eq!(ret, -ENOSYS);
        assert_eq!(audit::match_count(), 1);
        assert!(audit::ring_contains("syscall=9999"));
        assert!(audit::ring_contains("phase=exit"));

        let mut out = [TraceEvent::empty(); 4];
        let n = TRACE_RB.drain(&mut out);
        TRACE_RB.set_enabled(false);

        assert_eq!(n, 2);
        assert_eq!(out[0].ev_type, TRACE_SYSCALL_ENTER);
        assert_eq!(out[0].arg0, 9999);
        assert_eq!(out[1].ev_type, TRACE_SYSCALL_EXIT);
        assert_eq!(out[1].arg0, 9999);
        assert_eq!(out[1].arg1 as i64, -ENOSYS);
    }

    #[test]
    fn seccomp_errno_action_blocks_before_dispatch() {
        let seccomp = Seccomp::default();
        let filter = seccomp_prepare_filter(alloc::vec![SockFilter::stmt(
            BPF_RET | BPF_K,
            SECCOMP_RET_ERRNO | 13,
        )])
        .unwrap();
        unsafe {
            seccomp_attach_filter(&seccomp, filter);
        }

        let regs = regs_for_syscall(39);
        assert_eq!(
            syscall_seccomp_check_state(&regs, &seccomp),
            SeccompCheck::Errno(-13)
        );

        unsafe {
            SeccompFilter::put(seccomp.filter.load(Ordering::Acquire));
        }
    }

    #[test]
    fn strict_seccomp_only_allows_linux_strict_set() {
        let seccomp = Seccomp::default();
        seccomp
            .mode
            .store(SECCOMP_MODE_STRICT, core::sync::atomic::Ordering::Release);

        assert_eq!(
            syscall_seccomp_check_state(&regs_for_syscall(39), &seccomp),
            SeccompCheck::Errno(-EPERM)
        );
        assert_eq!(
            syscall_seccomp_check_state(&regs_for_syscall(SYS_EXIT), &seccomp),
            SeccompCheck::Allow
        );
    }

    #[test]
    fn seccomp_trap_preserves_filter_data_for_sigsys() {
        let seccomp = Seccomp::default();
        let filter = seccomp_prepare_filter(alloc::vec![SockFilter::stmt(
            BPF_RET | BPF_K,
            SECCOMP_RET_TRAP | 0x1234,
        )])
        .unwrap();
        unsafe {
            seccomp_attach_filter(&seccomp, filter);
        }

        assert_eq!(
            syscall_seccomp_check_state(&regs_for_syscall(204), &seccomp),
            SeccompCheck::Trap(0x1234)
        );

        unsafe {
            SeccompFilter::put(seccomp.filter.load(Ordering::Acquire));
        }
    }
}
