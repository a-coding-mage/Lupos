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
//!   * Per-CPU syscall scratch: the entry stub stores user RSP/RIP into slot 0
//!     of `SYSCALL_USER_RSP[]` (single-CPU safe only). Linux uses GS-relative
//!     per-CPU storage; needs per-CPU GS base before SMP syscall entry.
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
//! With STAR[63:48] = KERNEL_DS = 0x10 (see `gdt::sel`):
//!   SYSRET SS = 0x10 + 8  = 0x18 → gdt[USER_DS] ✓
//!   SYSRET CS = 0x10 + 16 = 0x20 → gdt[USER_CS] ✓
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
    SECCOMP_MODE_STRICT, SECCOMP_RET_ACTION_FULL, SECCOMP_RET_ALLOW, SECCOMP_RET_DATA,
    SECCOMP_RET_ERRNO, SECCOMP_RET_KILL_PROCESS, SECCOMP_RET_KILL_THREAD, SECCOMP_RET_LOG,
    SECCOMP_RET_TRACE, SECCOMP_RET_TRAP, SECCOMP_RET_USER_NOTIF, Seccomp, SeccompData,
    seccomp_run_filters,
};
use crate::kernel::signal;
use crate::kernel::task::{TIF_NEED_RESCHED, TIF_SIGPENDING};
use crate::kernel::trace::ring_buffer::{
    TRACE_RB, TRACE_SYSCALL_ENTER, TRACE_SYSCALL_EXIT, TraceEvent,
};
use crate::kernel::{audit, ptrace, sched};

// ── Per-CPU scratch storage ──────────────────────────────────────────────────
//
// On syscall entry, we need to save the user RSP before switching to the kernel
// stack. This static array (indexed by CPU) holds per-CPU scratch space for that.
//
// Accessed directly from assembly via:
//   mov [rax + SYSCALL_USER_RSP_ARRAY], rsp   (to save)
//   mov rsp, [rax + SYSCALL_USER_RSP_ARRAY]   (to restore)
//
// Where RAX = &SYSCALL_USER_RSP_ARRAY + (cpu_id * 8).

const MAX_CPUS: usize = 64; // Must match MAX_CPUS in sched.rs
static mut SYSCALL_USER_RSP: [u64; MAX_CPUS] = [0; MAX_CPUS];
static mut SYSCALL_USER_RIP: [u64; MAX_CPUS] = [0; MAX_CPUS];
static mut SYSCALL_ORIG_RAX: [u64; MAX_CPUS] = [0; MAX_CPUS];

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
const MSR_FS_BASE: u32 = 0xC000_0100; // User FS base (x86-64 TLS)

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
        // TODO: SMP-safe indexing (cpu id) — for now store into slot 0.
        "mov qword ptr [rip + {user_rsp_array}], rsp",
        "mov qword ptr [rip + {user_rip_array}], rcx",
        "mov qword ptr [rip + {orig_rax_array}], rax",

        // Switch to the current task's kernel stack (RSP0 is maintained by __switch_to()).
        "lea rcx, [rip + {tss}]",
        "mov rsp, [rcx + 4]",

        // Construct a Linux-shaped `struct pt_regs` on the kernel stack.
        "push {user_ds}", // ss
        "mov rax, qword ptr [rip + {user_rsp_array}]",
        "push rax", // rsp
        "push r11", // eflags
        "push {user_cs}", // cs
        "mov rax, qword ptr [rip + {user_rip_array}]",
        "push rax", // rip
        "mov rax, qword ptr [rip + {orig_rax_array}]",
        "push rax", // orig_rax

        // General purpose registers (reverse order so RSP points at r15).
        "push rdi",
        "push rsi",
        "push rdx",
        "mov rax, qword ptr [rip + {user_rip_array}]",
        "push rax", // rcx (syscall clobbers rcx to RIP)
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

        // Preserve the interrupted user x87/SSE state while Rust handles the
        // syscall.  The dynamic linker keeps bootstrap state in vector
        // registers across syscalls, matching Linux's user-visible contract.
        "mov rdi, rsp",
        "sub rsp, 528",
        "and rsp, -16",
        "mov [rsp + 512], rdi",
        "fxsave64 [rsp]",

        // rdi = &pt_regs, do_syscall_64-like table dispatch.
        // Linux runs normal syscall bodies with IRQs enabled after the entry
        // frame is complete; otherwise blocking syscalls can schedule with IF
        // masked and starve timer-driven wakeups.
        "sti",
        "mov rdi, [rsp + 512]",
        "call {dispatch_ptregs}",

        // Store return value into pt_regs.rax so the restore path reloads it.
        "mov rdi, [rsp + 512]",
        "mov [rdi + 80], rax",

        // Run exit slowpath work before we reload user TLS and SYSRET.
        "mov rdi, [rsp + 512]",
        "call {exit_slowpath}",

        // Linux reloads user TLS state before returning from the syscall path.
        // `arch_prctl(ARCH_SET_FS)` updates task.thread.fsbase; make that value
        // effective for libc before SYSRET resumes userspace.
        "call {load_user_tls}",

        // Linux only uses SYSRET for a clean syscall frame. Signal delivery
        // and rt_sigreturn can make user-visible RCX/R11 differ from the
        // SYSRET target/flags pair, so those paths must return via IRET.
        "mov rdi, [rsp + 512]",
        "call {should_use_sysret}",
        // Keep the branch decision in the FXSAVE scratch tail across the
        // restore and stack-pointer reload below.
        "mov [rsp + 520], al",

        // Keep interrupts closed while restoring userspace state and doing
        // SWAPGS/SYSRET or IRET, matching Linux's exit-to-user discipline.
        "cli",

        // Restore the user vector state and pt_regs stack pointer.
        "fxrstor64 [rsp]",
        "mov al, [rsp + 520]",
        "mov rsp, [rsp + 512]",

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

        user_rsp_array = sym SYSCALL_USER_RSP,
        user_rip_array = sym SYSCALL_USER_RIP,
        orig_rax_array = sym SYSCALL_ORIG_RAX,
        tss = sym crate::arch::x86::kernel::tss::TSS,
        dispatch_ptregs = sym syscall_dispatch_ptregs,
        exit_slowpath = sym syscall_exit_slowpath,
        load_user_tls = sym load_current_user_tls_base,
        should_use_sysret = sym syscall_should_use_sysret,
        user_cs = const sel::USER_CS as u64,
        user_ds = const sel::USER_DS as u64,
    );
}

pub(crate) unsafe extern "C" fn load_current_user_tls_base() {
    let task = sched::get_current();
    if task.is_null() {
        return;
    }
    let fsbase = unsafe { (*task).thread.fsbase };
    unsafe { wrmsr(MSR_FS_BASE, fsbase) };
}

pub(crate) unsafe extern "C" fn syscall_exit_slowpath(
    regs: *mut crate::arch::x86::kernel::ptrace::PtRegs,
) {
    let task = sched::get_current();
    if task.is_null() {
        return;
    }
    if unsafe { (*task).thread_info.flags & TIF_SIGPENDING != 0 } {
        if regs.is_null() {
            while unsafe { crate::kernel::signal::do_signal_stop_only() } {}
        } else {
            // `arch::x86::kernel::ptrace::PtRegs` and `kernel::task::PtRegs` both mirror
            // Linux `struct pt_regs` with the same `repr(C)` layout; the latter
            // keeps Linux's short field names used by the signal frame builder.
            let _ = unsafe { crate::kernel::signal::do_signal(regs.cast()) };
        }
    }
    sanitize_syscall_user_rflags(regs);
    let need_resched = unsafe { (*task).thread_info.flags & TIF_NEED_RESCHED != 0 };
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
            core::arch::asm!(
                "mov cr3, {0}",
                in(reg) pgd_phys,
                options(nostack, preserves_flags)
            );
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
static SYSCALL_DRAIN_LAST_JIFFY: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(u64::MAX);

unsafe fn syscall_dispatch_ptregs_inner(
    regs: *mut crate::arch::x86::kernel::ptrace::PtRegs,
) -> i64 {
    use super::syscall_table::{NR_syscalls, SYS_CALL_TABLE};

    let nr = unsafe { (*regs).orig_rax } as usize;
    let task = current_task_for_syscall();
    let hook_state = syscall_enter(unsafe { &*regs }, task);
    trace_udev_syscall_enter(unsafe { &*regs }, task);
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
        if SYSCALL_DRAIN_LAST_JIFFY.swap(now, core::sync::atomic::Ordering::Relaxed) != now {
            crate::init::rootfs::drain_console_control_bytes();
        }
    }

    let ret = match syscall_seccomp_check(unsafe { &*regs }, task) {
        Ok(()) if nr < NR_syscalls => unsafe { SYS_CALL_TABLE[nr](regs) },
        Ok(()) => {
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
        Err(errno) => errno,
    };

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
                (*regs).rip = ctx.ip;
                (*regs).rcx = ctx.ip;
                (*regs).rsp = ctx.sp;
                (*regs).eflags = ctx.rflags;
                (*regs).r11 = ctx.rflags;
                (*regs).rax = 0;
            }
        }
    }
    syscall_exit(unsafe { &*regs }, ret, task, hook_state);
    trace_systemd_service_syscall(unsafe { &*regs }, ret, task);
    trace_udev_syscall_exit(unsafe { &*regs }, ret, task);
    ret
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
/// Returns 0–63, clamped to MAX_CPUS.
///
/// Called from the syscall entry stub on *every* syscall. Reading the LAPIC ID
/// is an MMIO access — a VM-exit under VirtualBox (no APICv) and slow emulated
/// MMIO under TCG — so doing it per syscall dominated VBox boot time (systemd's
/// thousands of fast generator/manager syscalls each paid a VM-exit; the cost is
/// invisible on KVM where APICv makes the read free). When no AP is online the
/// only CPU is the BSP (id 0), so skip the LAPIC entirely; otherwise read it.
#[unsafe(no_mangle)]
pub extern "C" fn current_cpu_id() -> usize {
    if crate::arch::x86::kernel::smp::AP_READY_COUNT.load(core::sync::atomic::Ordering::Acquire)
        == 0
    {
        return 0;
    }
    let cpu_id = unsafe { crate::arch::x86::kernel::apic::id() } as usize;
    cpu_id.min(MAX_CPUS - 1)
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
            (thread_info.flags & TIF_SIGPENDING) != 0
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
pub unsafe fn init() {
    crate::arch::x86::kernel::setup_percpu::setup_percpu_segment(0);
    unsafe {
        // 1. Enable SCE in EFER.
        //    arch/x86/boot/header.S set LME (bit 8) and the CPU set LMA (bit 10) automatically.
        //    We preserve those bits and add SCE (bit 0).
        let efer = rdmsr(MSR_EFER);
        wrmsr(MSR_EFER, efer | EFER_SCE);

        // 2. Configure IA32_STAR — segment selectors for SYSCALL/SYSRET.
        //
        //    STAR[47:32] = kernel CS selector (SYSCALL loads CS and CS+8 as SS)
        //      → KERNEL_CS = 0x08 → CS = 0x08, SS = 0x10 = KERNEL_DS ✓
        //
        //    STAR[63:48] = base for user selectors (SYSRET adds 8 for SS, 16 for CS)
        //      → KERNEL_DS = 0x10 → SS = 0x18 = USER_DS, CS = 0x20 = USER_CS ✓
        //
        //    CPU automatically forces RPL=3 on the CS/SS selectors loaded by SYSRET.
        let star: u64 = ((sel::KERNEL_DS as u64) << 48) | ((sel::KERNEL_CS as u64) << 32);
        wrmsr(MSR_STAR, star);

        // 3. Set LSTAR — the RIP the CPU jumps to on SYSCALL.
        wrmsr(MSR_LSTAR, syscall_entry as *const () as u64);

        // 4. Set FMASK — RFLAGS bits cleared on SYSCALL entry.
        //    Clearing IF disables hardware interrupts until the kernel explicitly
        //    re-enables them after switching to a safe kernel stack.
        wrmsr(MSR_FMASK, SYSCALL_RFLAGS_MASK);
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
    let ping_trace = crate::kernel::debug_trace::ping_enabled();
    let systemctl_trace = crate::kernel::debug_trace::systemctl_enabled();
    let glycin_trace = crate::kernel::debug_trace::glycin_enabled();
    if !syscall_trace && !ping_trace && !systemctl_trace && !glycin_trace {
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
    let trace_user_manager = task_euid_for_trace(task) == Some(1000)
        && (comm_starts_with(comm, b"systemd") || comm_starts_with(comm, b"dbus-broker"));
    let trace_glycin = glycin_trace
        && (comm_starts_with(comm, b"glycin")
            || comm_starts_with(comm, b"bwrap")
            || comm_starts_with(comm, b"glycin-image")
            || comm_starts_with(comm, b"lightdm-gtk-gre")
            || trace_desktop_session
            || trace_user_manager);
    if !trace_pid1
        && !trace_systemd_service
        && !trace_dbus_broker
        && !trace_systemctl
        && !trace_dbus
        && !trace_ping
        && !trace_glycin
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
    if !interesting && !(trace_glycin && (ret < 0 || comm_starts_with(comm, b"bwrap"))) {
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

fn syscall_seccomp_check(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    task: *mut crate::kernel::task::TaskStruct,
) -> Result<(), i64> {
    if task.is_null() {
        return Ok(());
    }

    let seccomp = unsafe { &(*task).m27_seccomp };
    syscall_seccomp_check_state(regs, seccomp)
}

pub(crate) fn syscall_seccomp_check_state(
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    seccomp: &Seccomp,
) -> Result<(), i64> {
    if seccomp.mode.load(core::sync::atomic::Ordering::Acquire) == SECCOMP_MODE_STRICT
        && !strict_seccomp_allows(regs.orig_rax)
    {
        return Err(-EPERM);
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

fn seccomp_action_to_result(action: u32) -> Result<(), i64> {
    match action & SECCOMP_RET_ACTION_FULL {
        SECCOMP_RET_ALLOW | SECCOMP_RET_LOG => Ok(()),
        SECCOMP_RET_ERRNO => Err(-((action & SECCOMP_RET_DATA) as i64)),
        SECCOMP_RET_TRACE | SECCOMP_RET_USER_NOTIF => Err(-ENOSYS),
        SECCOMP_RET_TRAP | SECCOMP_RET_KILL_THREAD | SECCOMP_RET_KILL_PROCESS => Err(-EPERM),
        _ => Err(-EPERM),
    }
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
    fn star_msr_value_encodes_correct_selectors() {
        // Verify STAR layout without touching MSRs (host-side test).
        let star: u64 = ((sel::KERNEL_DS as u64) << 48) | ((sel::KERNEL_CS as u64) << 32);

        let syscall_cs = ((star >> 32) & 0xFFFF) as u16;
        let sysret_base = ((star >> 48) & 0xFFFF) as u16;

        assert_eq!(syscall_cs, sel::KERNEL_CS, "SYSCALL CS selector");
        // SYSRET SS = sysret_base + 8 → USER_DS (without RPL bits)
        assert_eq!(sysret_base + 8, sel::USER_DS & !3, "SYSRET SS selector");
        // SYSRET CS = sysret_base + 16 → USER_CS (without RPL bits)
        assert_eq!(sysret_base + 16, sel::USER_CS & !3, "SYSRET CS selector");
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
        let frame_complete = source
            .find("\"fxsave64 [rsp]\"")
            .expect("syscall entry must save user vector state");
        let irq_enable = source[frame_complete..]
            .find("\"sti\"")
            .map(|off| frame_complete + off)
            .expect("syscall entry must enable IRQs before dispatch");
        let dispatch = source
            .find("\"call {dispatch_ptregs}\"")
            .expect("syscall entry must call syscall dispatch");
        let exit_decision = source
            .find("\"call {should_use_sysret}\"")
            .expect("syscall entry must choose SYSRET vs IRET");
        let irq_disable = source[exit_decision..]
            .find("\"cli\"")
            .map(|off| exit_decision + off)
            .expect("syscall entry must disable IRQs before user restore");
        let restore = source
            .find("\"fxrstor64 [rsp]\"")
            .expect("syscall entry must restore user vector state");

        assert!(frame_complete < irq_enable);
        assert!(irq_enable < dispatch);
        assert!(dispatch < exit_decision);
        assert!(exit_decision < irq_disable);
        assert!(irq_disable < restore);
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
        assert_eq!(syscall_seccomp_check_state(&regs, &seccomp), Err(-13));

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
            Err(-EPERM)
        );
        assert_eq!(
            syscall_seccomp_check_state(&regs_for_syscall(SYS_EXIT), &seccomp),
            Ok(())
        );
    }
}
