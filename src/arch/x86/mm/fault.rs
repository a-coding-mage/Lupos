//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/mm/fault.c
//! test-origin: linux:vendor/linux/arch/x86/mm/fault.c
// x86-specific page-fault entry and dispatch.
//
// Maps to vendor/linux/arch/x86/mm/fault.c
//
// This module is the thin architecture layer that:
//   1. Reads CR2 immediately on entry (before any other work).
//   2. Decodes the x86 page-fault error code bits.
//   3. Separates kernel-address faults from user-address faults.
//   4. Looks up the faulting VMA and checks access permissions.
//   5. Delegates to `memory::fault::handle_mm_fault` for the generic path.
//
// The generic MM layer (`memory/fault.rs`) owns the page-table walk and the
// demand-paging state machine; the arch layer only knows about x86 specifics.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::arch::x86::kernel::idt::ExceptionFrame;
use crate::kernel::printk::log_error;
use crate::kernel::sched;
use crate::kernel::task::{TIF_NEED_RESCHED, TaskStruct};
use crate::mm::fault::{
    FAULT_FLAG_DEFAULT, FAULT_FLAG_INSTRUCTION, FAULT_FLAG_USER, FAULT_FLAG_WRITE, FaultFlags,
    VM_FAULT_ERROR, VmFaultFlags, handle_mm_fault,
};
use crate::mm::mm_types::{MmStruct, VmAreaStruct};
use crate::mm::mmap_lock::{MmapReadGuard, MmapWriteGuard};
use crate::mm::vm_flags::{VM_EXEC, VM_GROWSDOWN, VM_READ, VM_WRITE};
use crate::mm::vma::find_vma;

// ── x86 page-fault error code bits ──────────────────────────────────────────
//
// Ref: arch/x86/include/asm/trap_pf.h
// ---------------------------------------------------------------------------

/// Protection violation (bit 0).
/// 0 = page not present, 1 = protection (permissions or NX) violation.
pub const X86_PF_PROT: u64 = 1 << 0;

/// Write access (bit 1).
/// 0 = read, 1 = write.
pub const X86_PF_WRITE: u64 = 1 << 1;

/// User mode (bit 2).
/// 0 = supervisor (CPL < 3), 1 = user (CPL = 3).
pub const X86_PF_USER: u64 = 1 << 2;

/// Reserved bit violation (bit 3).
/// A reserved PTE/PDE bit was set.
pub const X86_PF_RSVD: u64 = 1 << 3;

/// Instruction fetch (bit 4).
/// NX-bit violation on an instruction fetch.
pub const X86_PF_INSTR: u64 = 1 << 4;

/// Protection-key violation (bit 5).
pub const X86_PF_PK: u64 = 1 << 5;

/// Shadow-stack access (bit 6).
pub const X86_PF_SHSTK: u64 = 1 << 6;

// Upper canonical-address boundary for user space on 4-level x86_64.
// Addresses >= TASK_SIZE_MAX are in kernel space.
// Ref: arch/x86/include/asm/processor.h — TASK_SIZE_MAX
const TASK_SIZE_MAX: u64 = 0x0000_8000_0000_0000;
const SEGV_MAPERR: i32 = 1;
const SEGV_ACCERR: i32 = 2;

// ---------------------------------------------------------------------------
// M12 placeholder: current mm_struct pointer.
//
// In a real kernel this comes from task_struct.mm (M20+).  For M12 we store
// it in an atomic so integration tests can install a test mm without unsafe
// static-mut access.
// ---------------------------------------------------------------------------

/// M12 placeholder — the mm_struct for the "current" process.
/// Set by `set_current_mm()`; replaced by task_struct in M20.
static CURRENT_TEST_MM: AtomicUsize = AtomicUsize::new(0);

/// Install `mm` as the mm_struct for the current context.
///
/// M12 test hook only; M20 will derive this from `task_struct.mm`.
pub fn set_current_mm(mm: *mut MmStruct) {
    CURRENT_TEST_MM.store(mm as usize, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Architecture entry point — do_page_fault
// ---------------------------------------------------------------------------

/// Handle a #PF exception delivered by the CPU.
///
/// CR2 must be read as the very first operation — the interrupt gate has
/// already cleared IF (no nested fault can overwrite CR2).
///
/// Ref: arch/x86/mm/fault.c — `exc_page_fault` / `handle_page_fault`
pub fn do_page_fault(frame: &ExceptionFrame) {
    let cr2: u64;
    // SAFETY: Reading CR2 is a privileged operation valid in ring 0.
    // The interrupt gate guarantees IF=0 on entry, so CR2 cannot be
    // overwritten by a nested fault before we read it here.
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2);
    }

    let ec = frame.error_code;

    if cr2 >= TASK_SIZE_MAX {
        do_kern_addr_fault(frame, ec, cr2);
    } else {
        do_user_addr_fault(frame, ec, cr2);
    }
}

// ---------------------------------------------------------------------------
// Kernel-address fault
// ---------------------------------------------------------------------------

/// Handle a #PF whose faulting address is in kernel space (cr2 >= TASK_SIZE_MAX).
///
/// For the M4 IDT smoke test (`test-page-fault` feature): exit QEMU with the
/// success code so the xtask harness can check that the banner was printed.
/// All other kernel faults are fatal.
///
/// Ref: arch/x86/mm/fault.c — `do_kern_addr_fault()`
fn do_kern_addr_fault(frame: &ExceptionFrame, ec: u64, addr: u64) {
    // Linux still routes user-mode accesses to kernel addresses through
    // bad_area_nosemaphore(), which delivers SIGSEGV instead of panicking.
    if is_user_mode_fault(frame, ec) {
        bad_area(frame, ec, addr);
        return;
    }

    if is_vmalloc_fault_candidate(ec, addr) && unsafe { crate::mm::vmalloc::vmalloc_fault(addr) } {
        return;
    }

    if is_ioremap_fault_candidate(ec, addr)
        && unsafe { crate::arch::x86::mm::ioremap::ioremap_fault(addr) }
    {
        return;
    }

    log_page_fault(frame, ec, addr);

    // Milestone 4 TDD: deliberate kernel #PF from main.rs → exit QEMU.
    #[cfg(all(feature = "qemu-test", feature = "test-page-fault"))]
    {
        // SAFETY: isa-debug-exit port write; valid in any privilege level.
        unsafe { crate::linux_driver_abi::platform::qemu::exit_success() };
    }

    panic!(
        "Kernel page fault: addr={:#018x} error={:#010x} rip={:#018x}",
        addr, ec, frame.rip,
    );
}

fn is_vmalloc_fault_candidate(ec: u64, addr: u64) -> bool {
    (ec & (X86_PF_PROT | X86_PF_RSVD)) == 0
        && crate::mm::vmalloc::is_vmalloc_addr(addr as *const u8)
}

fn is_ioremap_fault_candidate(ec: u64, addr: u64) -> bool {
    (ec & (X86_PF_PROT | X86_PF_RSVD)) == 0
        && crate::arch::x86::mm::ioremap::is_ioremap_addr(addr as *const u8)
}

// ---------------------------------------------------------------------------
// User-address fault
// ---------------------------------------------------------------------------

/// Reduced `lock_mm_and_find_vma()` for Lupos's current fault feature set.
///
/// The common path holds `mmap_lock` for reading across VMA lookup and
/// `handle_mm_fault()`. A downward-growing stack follows Linux's fallback
/// upgrade path: drop read, take write, repeat the lookup and checks, expand,
/// then atomically downgrade to read.
fn lock_mm_and_find_vma(
    mm: *mut MmStruct,
    addr: u64,
) -> Option<(MmapReadGuard, *mut VmAreaStruct)> {
    let read_guard = unsafe { MmapReadGuard::lock(mm) };
    let vma = find_vma(unsafe { &*mm }, addr)?;
    if unsafe { (*vma).vm_start <= addr } {
        return Some((read_guard, vma));
    }
    if unsafe { (*vma).vm_flags & VM_GROWSDOWN == 0 } {
        return None;
    }

    // Linux's non-atomic upgrade fallback deliberately revalidates the VMA
    // after the unlocked window.
    drop(read_guard);
    let write_guard = unsafe { MmapWriteGuard::lock(mm) };
    let vma = find_vma(unsafe { &*mm }, addr)?;
    if unsafe { (*vma).vm_start > addr } {
        if unsafe { (*vma).vm_flags & VM_GROWSDOWN == 0 }
            || crate::mm::mm_public::expand_stack_locked(vma, addr) != 0
        {
            return None;
        }
    }
    Some((write_guard.downgrade(), vma))
}

/// Handle a #PF whose faulting address is in user space (cr2 < TASK_SIZE_MAX).
///
/// Ref: arch/x86/mm/fault.c — `do_user_addr_fault()`
fn do_user_addr_fault(frame: &ExceptionFrame, ec: u64, addr: u64) {
    // Build FAULT_FLAG_* from the error code and privilege level.
    let mut flags: FaultFlags = FAULT_FLAG_DEFAULT;
    if ec & X86_PF_WRITE != 0 {
        flags |= FAULT_FLAG_WRITE;
    }
    if ec & X86_PF_INSTR != 0 {
        flags |= FAULT_FLAG_INSTRUCTION;
    }
    // CS ring bits: 0 = kernel, 3 = user.
    if frame.cs & 3 == 3 {
        flags |= FAULT_FLAG_USER;
    }

    // Prefer the real current task mm; keep the M12 test hook as fallback for
    // unit/early-boot harnesses that install a synthetic mm directly.
    let task = unsafe { sched::get_current() };
    let task_mm = if task.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { (*task).mm }
    };
    let mm = if task_mm.is_null() {
        CURRENT_TEST_MM.load(Ordering::Relaxed) as *mut MmStruct
    } else {
        task_mm
    };
    if mm.is_null() {
        bad_area(frame, ec, addr);
        return;
    }

    // Linux enables interrupts before taking the sleepable mmap lock. A
    // normal user fault necessarily interrupted an IF-enabled context.
    const X86_EFLAGS_IF: u64 = 1 << 9;
    if frame.rflags & X86_EFLAGS_IF == 0 {
        bad_area(frame, ec, addr);
        return;
    }
    crate::kernel::locking::local_irq_enable();

    let (mmap_guard, vma) = match lock_mm_and_find_vma(mm, addr) {
        Some(locked) => locked,
        None => {
            bad_area(frame, ec, addr);
            return;
        }
    };

    if crate::mm::huge::hwpoison_fault_pfn_for_addr(addr).is_some() {
        drop(mmap_guard);
        if is_user_mode_fault(frame, ec) && deliver_user_sigbus_hwpoison(frame, task, addr) {
            return;
        }
        bad_area(frame, ec, addr);
        return;
    }

    // Check that the access type is permitted by the VMA flags.
    if access_error(ec, unsafe { &*vma }) {
        drop(mmap_guard);
        bad_area(frame, ec, addr);
        return;
    }

    // Run the generic demand-paging state machine.
    let ret: VmFaultFlags = handle_mm_fault(vma, addr, flags);
    drop(mmap_guard);
    if ret & VM_FAULT_ERROR != 0 {
        bad_area(frame, ec, addr);
        return;
    }

    resched_after_user_fault(frame, ec, task);
}

fn should_resched_after_user_fault(frame: &ExceptionFrame, ec: u64, task: *mut TaskStruct) -> bool {
    if !is_user_mode_fault(frame, ec) || task.is_null() {
        return false;
    }
    let need_resched = unsafe {
        (*task)
            .thread_info
            .flags
            .load(core::sync::atomic::Ordering::Acquire)
            & TIF_NEED_RESCHED
            != 0
    };
    need_resched && crate::kernel::locking::preempt::preempt_count() == 0
}

fn resched_after_user_fault(frame: &ExceptionFrame, ec: u64, task: *mut TaskStruct) {
    if should_resched_after_user_fault(frame, ec, task) {
        // Mirror Linux's exit-to-user-mode loop: enable interrupts around the
        // runnable reschedule, then restore the IRQ-off page-fault return
        // invariant.  This is not a blocking sleep, so it must not use
        // `schedule_with_irqs_enabled()`; that helper may halt when the
        // faulting task is the only runnable task.
        crate::kernel::locking::local_irq_enable();
        let _ = unsafe { sched::schedule() };
        crate::kernel::locking::local_irq_disable();
    }
}

// ---------------------------------------------------------------------------
// access_error
// ---------------------------------------------------------------------------

/// Return `true` if the fault type is incompatible with the VMA permissions.
///
/// A return of `true` means the fault should produce a SIGSEGV.
///
/// Ref: arch/x86/mm/fault.c — `access_error()`
fn access_error(ec: u64, vma: &VmAreaStruct) -> bool {
    if ec & X86_PF_INSTR != 0 {
        // Instruction fetch: VMA must have the execute bit.
        return vma.vm_flags & VM_EXEC == 0;
    }
    if ec & X86_PF_WRITE != 0 {
        // Write access: VMA must have the write bit.
        return vma.vm_flags & VM_WRITE == 0;
    }
    // Read access: VMA must have the read bit.
    vma.vm_flags & VM_READ == 0
}

// ---------------------------------------------------------------------------
// bad_area — SIGSEGV delivery (stub for M12)
// ---------------------------------------------------------------------------

/// Signal a bad-area condition (SIGSEGV in a real kernel).
///
/// In M12 we simply panic; proper signal delivery arrives in M25.
///
/// Ref: arch/x86/mm/fault.c — `bad_area_nosemaphore()`
fn bad_area(frame: &ExceptionFrame, ec: u64, addr: u64) {
    let task = unsafe { sched::get_current() };
    if is_user_mode_fault(frame, ec) {
        // Match __bad_area_nosemaphore(): a userspace fault may run signal
        // delivery with interrupts enabled.  Firefox intentionally recovers
        // from many SIGSEGVs; keeping IF clear across that work starves timer
        // ticks and makes the whole desktop appear frozen.
        crate::kernel::locking::local_irq_enable();
        if !task.is_null() {
            let pid = unsafe { (*task).pid };
            if pid == 1 {
                panic!(
                    "init died from SIGSEGV: addr={:#018x} error={:#010x} rip={:#018x}",
                    addr, ec, frame.rip
                );
            }
            let si_code = user_sigsegv_code(task, addr);
            if deliver_user_sigsegv(frame, task, addr, si_code) {
                return;
            }
            unsafe {
                crate::kernel::exit::do_exit(segv_exit_code() as i64);
            }
        }
    }

    log_page_fault(frame, ec, addr);
    log_error!(
        "cpu",
        "cpu: bad area addr={:#018x} error={:#010x} rip={:#018x} rsp={:#018x}",
        addr,
        ec,
        frame.rip,
        frame.user_rsp,
    );
    panic!("Segmentation fault");
}

fn user_sigsegv_code(task: *mut TaskStruct, addr: u64) -> i32 {
    if task.is_null() {
        return SEGV_MAPERR;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return SEGV_MAPERR;
    }
    match find_vma(unsafe { &*mm }, addr) {
        Some(vma) if addr >= unsafe { (*vma).vm_start } => SEGV_ACCERR,
        _ => SEGV_MAPERR,
    }
}

fn exception_frame_to_ptregs(frame: &ExceptionFrame) -> crate::kernel::task::PtRegs {
    crate::kernel::task::PtRegs {
        r15: frame.r15,
        r14: frame.r14,
        r13: frame.r13,
        r12: frame.r12,
        bp: frame.rbp,
        bx: frame.rbx,
        r11: frame.r11,
        r10: frame.r10,
        r9: frame.r9,
        r8: frame.r8,
        ax: frame.rax,
        cx: frame.rcx,
        dx: frame.rdx,
        si: frame.rsi,
        di: frame.rdi,
        orig_ax: frame.error_code,
        ip: frame.rip,
        cs: frame.cs,
        flags: frame.rflags,
        sp: frame.user_rsp,
        ss: frame.user_ss,
    }
}

unsafe fn write_ptregs_to_exception_frame(
    frame: *mut ExceptionFrame,
    regs: &crate::kernel::task::PtRegs,
) {
    unsafe {
        (*frame).r15 = regs.r15;
        (*frame).r14 = regs.r14;
        (*frame).r13 = regs.r13;
        (*frame).r12 = regs.r12;
        (*frame).r11 = regs.r11;
        (*frame).r10 = regs.r10;
        (*frame).r9 = regs.r9;
        (*frame).r8 = regs.r8;
        (*frame).rdi = regs.di;
        (*frame).rsi = regs.si;
        (*frame).rbp = regs.bp;
        (*frame).rdx = regs.dx;
        (*frame).rcx = regs.cx;
        (*frame).rbx = regs.bx;
        (*frame).rax = regs.ax;
        (*frame).rip = regs.ip;
        (*frame).cs = regs.cs;
        (*frame).rflags = regs.flags;
        (*frame).user_rsp = regs.sp;
        (*frame).user_ss = regs.ss;
    }
}

fn deliver_user_sigsegv(
    frame: &ExceptionFrame,
    task: *mut crate::kernel::task::TaskStruct,
    addr: u64,
    si_code: i32,
) -> bool {
    let mut regs = exception_frame_to_ptregs(frame);
    let info = crate::kernel::signal::SigInfo::with_sigfault(
        crate::kernel::signal::SIGSEGV,
        si_code,
        addr,
        0,
    );
    unsafe {
        let _ = crate::kernel::signal::send_signal_info_to_task(task, info);
        if crate::kernel::signal::do_signal(&mut regs as *mut crate::kernel::task::PtRegs) {
            let frame_mut = frame as *const ExceptionFrame as *mut ExceptionFrame;
            write_ptregs_to_exception_frame(frame_mut, &regs);
            true
        } else {
            false
        }
    }
}

fn deliver_user_sigbus_hwpoison(
    frame: &ExceptionFrame,
    task: *mut crate::kernel::task::TaskStruct,
    addr: u64,
) -> bool {
    let mut regs = exception_frame_to_ptregs(frame);
    let info = crate::kernel::signal::SigInfo::with_sigfault(
        crate::kernel::signal::SIGBUS,
        crate::kernel::signal::BUS_MCEERR_AR,
        addr,
        crate::arch::x86::mm::paging::PAGE_SHIFT as i16,
    );
    unsafe {
        let _ = crate::kernel::signal::send_signal_info_to_task(task, info);
        if crate::kernel::signal::do_signal(&mut regs as *mut crate::kernel::task::PtRegs) {
            let frame_mut = frame as *const ExceptionFrame as *mut ExceptionFrame;
            write_ptregs_to_exception_frame(frame_mut, &regs);
            true
        } else {
            false
        }
    }
}

fn log_page_fault(frame: &ExceptionFrame, ec: u64, addr: u64) {
    // Log in the format the xtask smoke-test banner expects:
    //   "cpu: #PF cr2=0xffffdeadc0dedead"
    log_error!(
        "cpu",
        "cpu: #PF cr2={:#018x} error={:#010x} P={} W={} U={} I={} rip={:#018x}",
        addr,
        ec,
        u8::from((ec & X86_PF_PROT) != 0),
        u8::from((ec & X86_PF_WRITE) != 0),
        u8::from((ec & X86_PF_USER) != 0),
        u8::from((ec & X86_PF_INSTR) != 0),
        frame.rip,
    );
}

fn is_user_mode_fault(frame: &ExceptionFrame, ec: u64) -> bool {
    (frame.cs & 3) == 3 || (ec & X86_PF_USER) != 0
}

fn segv_exit_code() -> i32 {
    crate::kernel::wait::w_exitcode(0, crate::kernel::signal::SIGSEGV)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::mm_types::VmAreaStruct;
    use crate::mm::vm_flags::{VM_EXEC, VM_READ, VM_WRITE};
    use alloc::boxed::Box;

    #[test]
    fn x86_pf_bit_values_match_linux() {
        assert_eq!(X86_PF_PROT, 1 << 0);
        assert_eq!(X86_PF_WRITE, 1 << 1);
        assert_eq!(X86_PF_USER, 1 << 2);
        assert_eq!(X86_PF_RSVD, 1 << 3);
        assert_eq!(X86_PF_INSTR, 1 << 4);
        assert_eq!(X86_PF_PK, 1 << 5);
        assert_eq!(X86_PF_SHSTK, 1 << 6);
    }

    #[test]
    fn access_error_write_denied_on_read_only_vma() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, VM_READ); // no VM_WRITE
        assert!(access_error(X86_PF_WRITE, &vma));
    }

    #[test]
    fn access_error_write_allowed_on_writable_vma() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, VM_WRITE);
        assert!(!access_error(X86_PF_WRITE, &vma));
    }

    #[test]
    fn access_error_exec_denied_on_noexec_vma() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, VM_READ); // no VM_EXEC
        assert!(access_error(X86_PF_INSTR, &vma));
    }

    #[test]
    fn access_error_exec_allowed_on_exec_vma() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, VM_EXEC);
        assert!(!access_error(X86_PF_INSTR, &vma));
    }

    #[test]
    fn access_error_read_denied_on_no_read_vma() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, 0); // no VM_READ
        // No INSTR, no WRITE → read fault.
        assert!(access_error(0, &vma));
    }

    #[test]
    fn access_error_read_allowed_on_readable_vma() {
        let vma = VmAreaStruct::new(0x1000, 0x2000, VM_READ);
        assert!(!access_error(0, &vma));
    }

    fn test_frame(cs: u64) -> ExceptionFrame {
        ExceptionFrame {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rdi: 0,
            rsi: 0,
            rbp: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            vector: 14,
            error_code: 0,
            rip: 0x4000,
            cs,
            rflags: 0x202,
            user_rsp: 0x8000,
            user_ss: 0x1b,
        }
    }

    #[test]
    fn user_mode_fault_is_detected_from_cs_or_error_code() {
        let mut frame = test_frame(0x23);
        assert!(is_user_mode_fault(&frame, 0));
        frame.cs = 0x8;
        assert!(is_user_mode_fault(&frame, X86_PF_USER));
        assert!(!is_user_mode_fault(&frame, 0));
    }

    #[test]
    fn user_fault_reschedule_gate_requires_user_mode_need_resched_and_preemptible() {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let task_ptr = &mut *task as *mut TaskStruct;
        let mut frame = test_frame(0x23);

        assert!(!should_resched_after_user_fault(&frame, 0, task_ptr));

        task.thread_info
            .flags
            .fetch_or(TIF_NEED_RESCHED, core::sync::atomic::Ordering::Release);
        assert!(should_resched_after_user_fault(&frame, 0, task_ptr));

        frame.cs = 0x8;
        assert!(!should_resched_after_user_fault(&frame, 0, task_ptr));
        assert!(should_resched_after_user_fault(
            &frame,
            X86_PF_USER,
            task_ptr
        ));
        assert!(!should_resched_after_user_fault(
            &frame,
            X86_PF_USER,
            core::ptr::null_mut()
        ));
    }

    #[test]
    fn successful_user_fault_checks_resched_before_returning_to_user() {
        let source = include_str!("fault.rs");
        let tail = source
            .split("let ret: VmFaultFlags = handle_mm_fault(vma, addr, flags);")
            .nth(1)
            .expect("do_user_addr_fault must call handle_mm_fault");
        let error_return = tail
            .find("bad_area(frame, ec, addr);\n        return;")
            .expect("VM_FAULT_ERROR path must return before reschedule check");
        let resched = tail
            .find("resched_after_user_fault(frame, ec, task);")
            .expect("successful user fault path must check reschedule");
        assert!(error_return < resched);
    }

    #[test]
    fn vmalloc_fault_candidate_is_only_absent_vmalloc_fault() {
        let addr = crate::mm::vmalloc::VMALLOC_START;
        assert!(is_vmalloc_fault_candidate(0, addr));
        assert!(!is_vmalloc_fault_candidate(X86_PF_PROT, addr));
        assert!(!is_vmalloc_fault_candidate(X86_PF_RSVD, addr));
        assert!(!is_vmalloc_fault_candidate(0, TASK_SIZE_MAX));
    }

    #[test]
    fn segv_exit_code_matches_linux_wait_status() {
        assert_eq!(
            segv_exit_code(),
            crate::kernel::wait::w_exitcode(0, crate::kernel::signal::SIGSEGV)
        );
    }

    #[test]
    fn task_size_max_is_canonical_boundary() {
        // Upper canonical boundary for 4-level paging on x86_64.
        assert_eq!(TASK_SIZE_MAX, 0x0000_8000_0000_0000);
    }
}
