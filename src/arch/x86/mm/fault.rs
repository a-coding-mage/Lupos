//! linux-parity: complete
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

    // Look up the VMA containing (or just above) the faulting address.
    let vma_opt = find_vma(unsafe { &*mm }, addr);
    let vma = match vma_opt {
        Some(v) => v,
        None => {
            bad_area(frame, ec, addr);
            return;
        }
    };

    // The faulting address may lie *below* the VMA start if the VMA can grow
    // downward (e.g., a stack).  For non-growsdown VMAs it is a bad area.
    if addr < unsafe { (*vma).vm_start } {
        if unsafe { (*vma).vm_flags } & VM_GROWSDOWN == 0 {
            bad_area(frame, ec, addr);
            return;
        }
        if crate::mm::mm_public::expand_stack_locked(vma, addr) != 0
            || addr < unsafe { (*vma).vm_start }
        {
            bad_area(frame, ec, addr);
            return;
        }
    }

    if crate::mm::huge::hwpoison_fault_pfn_for_addr(addr).is_some() {
        if is_user_mode_fault(frame, ec) && deliver_user_sigbus_hwpoison(frame, task, addr) {
            return;
        }
        bad_area(frame, ec, addr);
        return;
    }

    // Check that the access type is permitted by the VMA flags.
    if access_error(ec, unsafe { &*vma }) {
        bad_area(frame, ec, addr);
        return;
    }

    // Run the generic demand-paging state machine.
    let ret: VmFaultFlags = handle_mm_fault(vma, addr, flags);
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
    let need_resched = unsafe { (*task).thread_info.flags & TIF_NEED_RESCHED != 0 };
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
    log_page_fault(frame, ec, addr);
    let task = unsafe { sched::get_current() };
    #[cfg(not(test))]
    if is_user_mode_fault(frame, ec) && !task.is_null() {
        dump_vmas_near(task, addr);
    }
    log_error!(
        "cpu",
        "cpu: bad area addr={:#018x} error={:#010x} rip={:#018x} r12={:#018x} r13={:#018x} rbp={:#018x} rbx={:#018x}",
        addr,
        ec,
        frame.rip,
        frame.r12,
        frame.r13,
        frame.rbp,
        frame.rbx,
    );
    if is_user_mode_fault(frame, ec) {
        if !task.is_null() {
            let pid = unsafe { (*task).pid };
            #[cfg(not(test))]
            {
                let fs_msr = unsafe {
                    crate::arch::x86::kernel::msr::read(crate::arch::x86::kernel::msr::MSR_FS_BASE)
                };
                let task_fs = unsafe { (*task).thread.fsbase };
                let comm = unsafe { (*task).comm };
                let comm_str = {
                    let end = comm.iter().position(|&c| c == 0).unwrap_or(comm.len());
                    core::str::from_utf8(&comm[..end]).unwrap_or("<?>")
                };
                // Dump the faulting instruction bytes — but only for data faults,
                // where rip is guaranteed present (it was fetched).  On an
                // instruction-fetch fault rip itself may be unmapped, so reading
                // it from the kernel would double-fault.
                let mut insn = [0u8; 16];
                if ec & X86_PF_INSTR == 0 {
                    for (i, b) in insn.iter_mut().enumerate() {
                        *b = unsafe { core::ptr::read_volatile((frame.rip as *const u8).add(i)) };
                    }
                }
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-user-pf pid={} comm={} fs_msr={:#x} task_fs={:#x} cr2={:#x} rip={:#x} insn={:02x?}",
                    pid,
                    comm_str,
                    fs_msr,
                    task_fs,
                    addr,
                    frame.rip,
                    insn
                );
            }
            if pid == 1 {
                panic!(
                    "init died from SIGSEGV: addr={:#018x} error={:#010x} rip={:#018x}",
                    addr, ec, frame.rip
                );
            }
            if deliver_user_sigsegv(frame, task) {
                return;
            }
            unsafe {
                crate::kernel::exit::do_exit(segv_exit_code() as i64);
            }
        }
    }
    panic!("Segmentation fault");
}

/// Diagnostic: dump the VMAs surrounding a faulting user address so we can tell
/// whether the fault is a missing VMA, a permission mismatch (e.g. a leftover
/// PROT_NONE ld.so reservation), or a file-backed mapping. Removable.
#[cfg(not(test))]
fn dump_vmas_near(task: *mut TaskStruct, addr: u64) {
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return;
    }
    let mm_ref = unsafe { &*mm };
    let lo = addr.saturating_sub(0x40_0000);
    let hi = addr.saturating_add(0x40_0000);
    crate::linux_driver_abi::tty::serial_println!(
        "bad-area-vma-dump pid={} cr2={:#x} window=[{:#x},{:#x})",
        unsafe { (*task).pid },
        addr,
        lo,
        hi
    );
    let mut idx = lo;
    let mut printed = 0;
    while printed < 24 {
        let next = mm_ref.mm_mt.find_first_gte(idx);
        let (start, end, value) = match next {
            Some(t) => t,
            None => break,
        };
        if start > hi {
            break;
        }
        // mm_mt stores [vm_start, vm_end-1]; recover vm_end.
        let vm_end = end.saturating_add(1);
        let vma = value as *const VmAreaStruct;
        let (flags, file, ops, pgoff) = if vma.is_null() {
            (0, 0, 0, 0)
        } else {
            unsafe {
                (
                    (*vma).vm_flags,
                    (*vma).vm_file,
                    (*vma).vm_ops,
                    (*vma).vm_pgoff,
                )
            }
        };
        let covers = addr >= start && addr < vm_end;
        crate::linux_driver_abi::tty::serial_println!(
            "  vma [{:#x},{:#x}) flags={:#x} file={:#x} ops={:#x} pgoff={:#x}{}",
            start,
            vm_end,
            flags,
            file,
            ops,
            pgoff,
            if covers { "  <== COVERS cr2" } else { "" }
        );
        printed += 1;
        if vm_end <= idx {
            break;
        }
        idx = vm_end;
    }
    if printed == 0 {
        crate::linux_driver_abi::tty::serial_println!("  (no VMAs in window)");
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
) -> bool {
    let mut regs = exception_frame_to_ptregs(frame);
    unsafe {
        let _ = crate::kernel::signal::send_signal_to_task(task, crate::kernel::signal::SIGSEGV);
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

        task.thread_info.flags |= TIF_NEED_RESCHED;
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
