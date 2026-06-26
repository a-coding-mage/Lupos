//! linux-parity: complete
//! linux-source: vendor/linux/kernel/ptrace.c
//! test-origin: linux:vendor/linux/kernel/ptrace.c
//! Process tracing — Milestone 26 (minimal subset).
//!
//! Implements the kernel-side scaffold for `ptrace(2)` with the requests
//! needed by M65's ptrace/selftest closure: attach/seize/detach, continue,
//! syscall-stop tracing, register get/set/regset, memory peek/poke, event
//! messages, and syscall-info snapshots.
//!
//! Reference: `vendor/linux/include/uapi/linux/ptrace.h`,
//! `vendor/linux/kernel/ptrace.c`.

use core::sync::atomic::Ordering;

use crate::arch::x86::kernel::uaccess::{copy_from_user, copy_to_user};
use crate::kernel::capability::{CAP_SYS_PTRACE, capable};
use crate::kernel::cred::{Cred, INIT_CRED};
use crate::kernel::printk::log_info;
use crate::kernel::sched;
use crate::kernel::signal::{self, SIGKILL, SIGSTOP};
use crate::kernel::task::TaskStruct;
use crate::kernel::task::task_state::{__TASK_TRACED, TASK_RUNNING};

// ── Linux ABI request numbers ────────────────────────────────────────────────

pub const PTRACE_TRACEME: i64 = 0;
pub const PTRACE_PEEKTEXT: i64 = 1;
pub const PTRACE_PEEKDATA: i64 = 2;
pub const PTRACE_POKETEXT: i64 = 4;
pub const PTRACE_POKEDATA: i64 = 5;
pub const PTRACE_CONT: i64 = 7;
pub const PTRACE_KILL: i64 = 8;
pub const PTRACE_SINGLESTEP: i64 = 9;
pub const PTRACE_GETREGS: i64 = 12;
pub const PTRACE_SETREGS: i64 = 13;
pub const PTRACE_ATTACH: i64 = 16;
pub const PTRACE_DETACH: i64 = 17;
pub const PTRACE_SYSCALL: i64 = 24;
pub const PTRACE_SETOPTIONS: i64 = 0x4200;
pub const PTRACE_GETEVENTMSG: i64 = 0x4201;
pub const PTRACE_GETREGSET: i64 = 0x4204;
pub const PTRACE_SEIZE: i64 = 0x4206;
pub const PTRACE_INTERRUPT: i64 = 0x4207;
pub const PTRACE_LISTEN: i64 = 0x4208;
pub const PTRACE_GET_SYSCALL_INFO: i64 = 0x420e;

// ── ptrace flags ─────────────────────────────────────────────────────────────

pub const PT_PTRACED: u32 = 1 << 0;
pub const PT_SEIZED: u32 = 1 << 1;
pub const PT_TRACESYSGOOD: u32 = 1 << 2;
pub const PT_SYSCALL_TRACE: u32 = 1 << 3;
pub const PT_SINGLESTEP: u32 = 1 << 4;

pub const PTRACE_O_TRACESYSGOOD: u64 = 0x0000_0001;
pub const NT_PRSTATUS: u64 = 1;
pub const PTRACE_SYSCALL_INFO_NONE: u8 = 0;
pub const PTRACE_SYSCALL_INFO_ENTRY: u8 = 1;
pub const PTRACE_SYSCALL_INFO_EXIT: u8 = 2;

const EPERM: i64 = -1;
const ESRCH: i64 = -3;
const EFAULT: i64 = -14;
const EINVAL: i64 = -22;
const SIGTRAP: i32 = 5;

#[inline]
unsafe fn task_cred_or_init(task: *mut TaskStruct) -> *const Cred {
    if task.is_null() {
        return &raw const INIT_CRED;
    }
    unsafe {
        if !(*task).cred.is_null() {
            (*task).cred
        } else if !(*task).m27.real_cred.is_null() {
            (*task).m27.real_cred
        } else {
            &raw const INIT_CRED
        }
    }
}

unsafe fn ptrace_may_access(cur: *mut TaskStruct, target: *mut TaskStruct) -> bool {
    if cur.is_null() || target.is_null() {
        return false;
    }
    if unsafe { (*cur).tgid == (*target).tgid } {
        return true;
    }
    if capable(CAP_SYS_PTRACE) {
        return true;
    }

    let cur_cred = unsafe { task_cred_or_init(cur) };
    let target_cred = unsafe { task_cred_or_init(target) };
    if cur_cred.is_null() || target_cred.is_null() {
        return false;
    }
    unsafe {
        let uid_match = (*cur_cred).uid == (*target_cred).uid
            && (*cur_cred).uid == (*target_cred).euid
            && (*cur_cred).uid == (*target_cred).suid;
        let gid_match = (*cur_cred).gid == (*target_cred).gid
            && (*cur_cred).gid == (*target_cred).egid
            && (*cur_cred).gid == (*target_cred).sgid;
        uid_match && gid_match
    }
}

#[inline]
unsafe fn copy_struct_to_user<T>(dst: *mut T, src: &T) -> Result<(), i64> {
    if dst.is_null() {
        return Err(EFAULT);
    }
    let not_copied = unsafe {
        copy_to_user(
            dst.cast::<u8>(),
            (src as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if not_copied == 0 { Ok(()) } else { Err(EFAULT) }
}

#[inline]
unsafe fn copy_struct_from_user<T>(dst: &mut T, src: *const T) -> Result<(), i64> {
    if src.is_null() {
        return Err(EFAULT);
    }
    let not_copied = unsafe {
        copy_from_user(
            (dst as *mut T).cast::<u8>(),
            src.cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if not_copied == 0 { Ok(()) } else { Err(EFAULT) }
}

unsafe fn ptrace_collect_user_regs(
    target: *mut TaskStruct,
) -> crate::arch::x86::kernel::ptrace::UserRegsStruct {
    unsafe {
        let regs = crate::arch::x86::kernel::ptrace::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rax: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            orig_rax: 0,
            rip: 0,
            cs: 0,
            eflags: 0,
            rsp: (*target).thread.sp,
            ss: 0,
        };
        let thr = &(*target).thread;
        crate::arch::x86::kernel::ptrace::user_regs_from_pt_regs(
            &regs,
            crate::arch::x86::kernel::ptrace::SegmentState {
                fs_base: thr.fsbase,
                gs_base: thr.gsbase,
                ds: thr.ds as u64,
                es: thr.es as u64,
                fs: thr.fsindex as u64,
                gs: thr.gsindex as u64,
            },
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserIovec {
    pub iov_base: *mut core::ffi::c_void,
    pub iov_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PtraceSyscallInfoEntry {
    pub nr: u64,
    pub args: [u64; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PtraceSyscallInfoExit {
    pub rval: i64,
    pub is_error: u8,
    pub _pad: [u8; 7],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PtraceSyscallInfoSeccomp {
    pub nr: u64,
    pub args: [u64; 6],
    pub ret_data: u32,
    pub reserved2: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union PtraceSyscallInfoData {
    pub entry: PtraceSyscallInfoEntry,
    pub exit_: PtraceSyscallInfoExit,
    pub seccomp: PtraceSyscallInfoSeccomp,
    pub raw: [u8; 64],
}

impl Default for PtraceSyscallInfoData {
    fn default() -> Self {
        Self { raw: [0; 64] }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PtraceSyscallInfo {
    pub op: u8,
    pub reserved: u8,
    pub flags: u16,
    pub arch: u32,
    pub instruction_pointer: u64,
    pub stack_pointer: u64,
    pub data: PtraceSyscallInfoData,
}

// ── PID lookup helper ────────────────────────────────────────────────────────

/// Locate a task by `pid` in the global heap-task tracker plus the BSP / pool.
///
/// M26 scope: scan heap-allocated tasks (created via `kernel_clone`) plus the
/// static `TASK_POOL` slots used by `kthread_create`.  Returns NULL if no
/// match.  Replaced by an `IDR`-backed lookup in M28.
fn find_task_by_pid(pid: i32) -> *mut TaskStruct {
    // Heap path first.
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    // Pool path fallback (BSP + static kthreads).
    crate::kernel::sched::find_pool_task_by_pid(pid)
}

unsafe fn ptrace_stop(task: *mut TaskStruct, signal: i32, message: u64) {
    if task.is_null() {
        return;
    }
    unsafe {
        (*task).m26.ptrace_stop_signal = signal;
        (*task).m26.ptrace_message = message;
        (*task).__state.store(__TASK_TRACED, Ordering::Release);
        let waiter_count = (*task).m26.wait_count as usize;
        for idx in 0..waiter_count.min(crate::kernel::task::MAX_WAITERS) {
            let waiter = (*task).m26.wait_waiters[idx];
            if !waiter.is_null() {
                crate::kernel::sched::wake_task(waiter);
            }
        }
        if sched::get_current() == task {
            while (*task).__state.load(Ordering::Acquire) == __TASK_TRACED {
                sched::schedule_with_irqs_enabled();
            }
            (*task).m26.ptrace_stop_signal = 0;
        }
    }
}

unsafe fn apply_ptrace_options(task: *mut TaskStruct, options: u64) {
    if task.is_null() {
        return;
    }
    unsafe {
        if options & PTRACE_O_TRACESYSGOOD != 0 {
            (*task).m26.ptrace |= PT_TRACESYSGOOD;
        } else {
            (*task).m26.ptrace &= !PT_TRACESYSGOOD;
        }
    }
}

// ── Public ptrace dispatch ───────────────────────────────────────────────────

/// `sys_ptrace(request, pid, addr, data)` — Linux syscall 101.
///
/// Returns 0 (or a request-specific value) on success and a negated errno on
/// failure.
pub unsafe fn sys_ptrace(request: i64, pid: i32, addr: u64, data: u64) -> i64 {
    match request {
        PTRACE_TRACEME => unsafe { ptrace_traceme() },
        PTRACE_ATTACH => unsafe { ptrace_attach(pid) },
        PTRACE_SEIZE => unsafe { ptrace_seize(pid, data) },
        PTRACE_DETACH => unsafe { ptrace_detach(pid) },
        PTRACE_CONT => unsafe { ptrace_cont(pid) },
        PTRACE_SYSCALL => unsafe { ptrace_syscall(pid) },
        PTRACE_SINGLESTEP => unsafe { ptrace_singlestep(pid) },
        PTRACE_INTERRUPT => unsafe { ptrace_interrupt(pid) },
        PTRACE_LISTEN => unsafe { ptrace_listen(pid) },
        PTRACE_KILL => unsafe { ptrace_kill(pid) },
        PTRACE_SETOPTIONS => unsafe { ptrace_setoptions(pid, data) },
        PTRACE_GETEVENTMSG => unsafe { ptrace_geteventmsg(pid, data as *mut u64) },
        PTRACE_GETREGS => unsafe {
            ptrace_getregs(
                pid,
                data as *mut crate::arch::x86::kernel::ptrace::UserRegsStruct,
            )
        },
        PTRACE_SETREGS => unsafe {
            ptrace_setregs(
                pid,
                data as *const crate::arch::x86::kernel::ptrace::UserRegsStruct,
            )
        },
        PTRACE_GETREGSET => unsafe { ptrace_getregset(pid, addr, data as *mut UserIovec) },
        PTRACE_GET_SYSCALL_INFO => unsafe {
            ptrace_get_syscall_info(pid, addr as usize, data as *mut PtraceSyscallInfo)
        },
        PTRACE_PEEKDATA | PTRACE_PEEKTEXT => unsafe { ptrace_peekdata(pid, addr) },
        PTRACE_POKEDATA | PTRACE_POKETEXT => unsafe { ptrace_pokedata(pid, addr, data) },
        _ => EINVAL,
    }
}

/// `PTRACE_TRACEME` — current task asks its parent to trace it.
unsafe fn ptrace_traceme() -> i64 {
    let cur = unsafe { sched::get_current() };
    if cur.is_null() {
        return ESRCH;
    }
    if false && cfg!(feature = "test-exit-wait-ptrace") {
        log_info!(
            "ptrace",
            "ptrace_traceme: pid={} real_parent={:p}",
            unsafe { (*cur).pid },
            unsafe { (*cur).m26.real_parent }
        );
    }
    unsafe {
        (*cur).m26.ptrace |= PT_PTRACED;
        if false && cfg!(feature = "test-exit-wait-ptrace") {
            log_info!("ptrace", "ptrace_traceme: PT_PTRACED set");
        }
        (*cur).m26.tracer = (*cur).m26.real_parent;
        if false && cfg!(feature = "test-exit-wait-ptrace") {
            log_info!("ptrace", "ptrace_traceme: tracer set");
        }
    }
    0
}

/// `PTRACE_ATTACH` — current task attaches to `pid` as its tracer.
unsafe fn ptrace_attach(pid: i32) -> i64 {
    unsafe { ptrace_attach_common(pid, false, 0) }
}

unsafe fn ptrace_seize(pid: i32, options: u64) -> i64 {
    unsafe { ptrace_attach_common(pid, true, options) }
}

unsafe fn ptrace_attach_common(pid: i32, seize: bool, options: u64) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    let cur = unsafe { sched::get_current() };
    if cur.is_null() || cur == target {
        return EINVAL;
    }
    unsafe {
        if !ptrace_may_access(cur, target) {
            return EPERM;
        }
        if (*target).m26.ptrace & PT_PTRACED != 0 {
            return EINVAL; // already traced
        }
        (*target).m26.ptrace |= PT_PTRACED;
        if seize {
            (*target).m26.ptrace |= PT_SEIZED;
        }
        apply_ptrace_options(target, options);
        (*target).m26.tracer = cur;
        (*target).m26.ptracer_cred = (*cur).cred;

        // Stop the tracee — set TASK_TRACED and queue SIGSTOP for delivery.
        if !seize {
            ptrace_stop(target, SIGSTOP, 0);
            let _ = signal::send_signal_to_task(target, SIGSTOP);
        }
    }
    0
}

/// `PTRACE_DETACH` — clear tracer and resume.
unsafe fn ptrace_detach(pid: i32) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        (*target).m26.ptrace &=
            !(PT_PTRACED | PT_SEIZED | PT_TRACESYSGOOD | PT_SYSCALL_TRACE | PT_SINGLESTEP);
        (*target).m26.tracer = core::ptr::null_mut();
        (*target).m26.ptracer_cred = core::ptr::null();
        (*target).m26.ptrace_stop_signal = 0;
        (*target).m26.ptrace_syscall_op = PTRACE_SYSCALL_INFO_NONE;
        (*target).m26.ptrace_syscall_nr = -1;
        (*target).m26.ptrace_syscall_args = [0; 6];
        (*target).__state.store(TASK_RUNNING, Ordering::Release);
    }
    0
}

/// `PTRACE_CONT` — resume a stopped tracee.
unsafe fn ptrace_cont(pid: i32) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        (*target).m26.ptrace &= !(PT_SYSCALL_TRACE | PT_SINGLESTEP);
        (*target).m26.ptrace_stop_signal = 0;
        (*target).__state.store(TASK_RUNNING, Ordering::Release);
    }
    0
}

unsafe fn ptrace_syscall(pid: i32) -> i64 {
    unsafe { ptrace_resume_with_flags(pid, PT_SYSCALL_TRACE) }
}

unsafe fn ptrace_singlestep(pid: i32) -> i64 {
    unsafe { ptrace_resume_with_flags(pid, PT_SINGLESTEP) }
}

unsafe fn ptrace_resume_with_flags(pid: i32, flags: u32) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        (*target).m26.ptrace &= !(PT_SYSCALL_TRACE | PT_SINGLESTEP);
        (*target).m26.ptrace |= flags;
        (*target).m26.ptrace_stop_signal = 0;
        (*target).__state.store(TASK_RUNNING, Ordering::Release);
    }
    0
}

unsafe fn ptrace_interrupt(pid: i32) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_SEIZED == 0 {
            return EINVAL;
        }
        ptrace_stop(target, SIGTRAP, 0);
    }
    0
}

unsafe fn ptrace_listen(pid: i32) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_SEIZED == 0 {
            return EINVAL;
        }
        (*target).m26.ptrace_stop_signal = 0;
        (*target).__state.store(TASK_RUNNING, Ordering::Release);
    }
    0
}

unsafe fn ptrace_kill(pid: i32) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        let _ = signal::send_signal_to_task(target, SIGKILL);
        (*target).m26.exit_code = crate::kernel::wait::w_exitcode(0, SIGKILL);
    }
    0
}

unsafe fn ptrace_setoptions(pid: i32, options: u64) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        apply_ptrace_options(target, options);
    }
    0
}

unsafe fn ptrace_geteventmsg(pid: i32, out: *mut u64) -> i64 {
    if out.is_null() {
        return EFAULT;
    }
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        let message = (*target).m26.ptrace_message;
        if let Err(errno) = copy_struct_to_user(out, &message) {
            return errno;
        }
    }
    0
}

/// `PTRACE_GETREGS` — copy a snapshot of the tracee's saved register state
/// into `regs_out`.
///
/// M26 limitation: in-kernel-only tracees (kthreads) never enter user mode,
/// so general-purpose registers are zero by construction. Segment selectors
/// and FS/GS base values are read from the tracee's `ThreadStruct`.
unsafe fn ptrace_getregs(
    pid: i32,
    regs_out: *mut crate::arch::x86::kernel::ptrace::UserRegsStruct,
) -> i64 {
    if regs_out.is_null() {
        return EFAULT;
    }
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        let user_regs = ptrace_collect_user_regs(target);
        if let Err(errno) = copy_struct_to_user(regs_out, &user_regs) {
            return errno;
        }
    }
    0
}

unsafe fn ptrace_setregs(
    pid: i32,
    regs_in: *const crate::arch::x86::kernel::ptrace::UserRegsStruct,
) -> i64 {
    if regs_in.is_null() {
        return EFAULT;
    }
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        let mut user = crate::arch::x86::kernel::ptrace::UserRegsStruct::default();
        if let Err(errno) = copy_struct_from_user(&mut user, regs_in) {
            return errno;
        }
        let mut regs = crate::arch::x86::kernel::ptrace::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rax: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            orig_rax: 0,
            rip: 0,
            cs: 0x33,
            eflags: 0x202,
            rsp: (*target).thread.sp,
            ss: 0x2b,
        };
        let segments =
            match crate::arch::x86::kernel::ptrace::apply_user_regs_to_pt_regs(&user, &mut regs) {
                Ok(segments) => segments,
                Err(_) => return EINVAL,
            };
        (*target).thread.sp = regs.rsp;
        (*target).thread.fsbase = segments.fs_base;
        (*target).thread.gsbase = segments.gs_base;
        (*target).thread.ds = segments.ds as u16;
        (*target).thread.es = segments.es as u16;
        (*target).thread.fsindex = segments.fs as u16;
        (*target).thread.gsindex = segments.gs as u16;
    }
    0
}

unsafe fn ptrace_getregset(pid: i32, note_type: u64, iov: *mut UserIovec) -> i64 {
    if note_type != NT_PRSTATUS {
        return EINVAL;
    }
    if iov.is_null() {
        return EFAULT;
    }
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        let mut iov_local = UserIovec {
            iov_base: core::ptr::null_mut(),
            iov_len: 0,
        };
        if let Err(errno) = copy_struct_from_user(&mut iov_local, iov as *const UserIovec) {
            return errno;
        }
        if iov_local.iov_base.is_null() {
            return EFAULT;
        }
        let regs = ptrace_collect_user_regs(target);
        let n = iov_local.iov_len.min(core::mem::size_of::<
            crate::arch::x86::kernel::ptrace::UserRegsStruct,
        >());
        let not_copied = copy_to_user(
            iov_local.iov_base.cast::<u8>(),
            (&regs as *const crate::arch::x86::kernel::ptrace::UserRegsStruct).cast::<u8>(),
            n,
        );
        if not_copied != 0 {
            return EFAULT;
        }
        iov_local.iov_len = n;
        if let Err(errno) = copy_struct_to_user(iov, &iov_local) {
            return errno;
        }
    }
    0
}

unsafe fn ptrace_get_syscall_info(pid: i32, user_size: usize, out: *mut PtraceSyscallInfo) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        let op = match (*target).m26.ptrace_syscall_op {
            PTRACE_SYSCALL_INFO_ENTRY | PTRACE_SYSCALL_INFO_EXIT => (*target).m26.ptrace_syscall_op,
            _ => PTRACE_SYSCALL_INFO_NONE,
        };
        let ip = if (*target).m26.ptrace_syscall_ip != 0 {
            (*target).m26.ptrace_syscall_ip
        } else {
            (*target).thread.sp as u64
        };
        let sp = if (*target).m26.ptrace_syscall_sp != 0 {
            (*target).m26.ptrace_syscall_sp
        } else {
            (*target).thread.sp as u64
        };
        let mut info = PtraceSyscallInfo {
            op,
            reserved: 0,
            flags: 0,
            arch: 0xC000_003E,
            instruction_pointer: ip.max(1),
            stack_pointer: sp.max(1),
            data: PtraceSyscallInfoData::default(),
        };
        let reported_size = match op {
            PTRACE_SYSCALL_INFO_ENTRY => {
                info.data.entry = PtraceSyscallInfoEntry {
                    nr: (*target).m26.ptrace_syscall_nr.max(0) as u64,
                    args: (*target).m26.ptrace_syscall_args,
                };
                core::mem::offset_of!(PtraceSyscallInfo, data)
                    + core::mem::size_of::<PtraceSyscallInfoEntry>()
            }
            PTRACE_SYSCALL_INFO_EXIT => {
                info.data.exit_ = PtraceSyscallInfoExit {
                    rval: (*target).m26.ptrace_syscall_ret,
                    is_error: ((*target).m26.ptrace_syscall_ret < 0) as u8,
                    _pad: [0; 7],
                };
                core::mem::offset_of!(PtraceSyscallInfo, data)
                    + core::mem::offset_of!(PtraceSyscallInfoExit, is_error)
                    + core::mem::size_of::<u8>()
            }
            _ => core::mem::offset_of!(PtraceSyscallInfo, data),
        };
        if !out.is_null() {
            let copy_len = user_size.min(reported_size);
            let not_copied = copy_to_user(
                out.cast::<u8>(),
                (&info as *const PtraceSyscallInfo).cast::<u8>(),
                copy_len,
            );
            if not_copied != 0 {
                return EFAULT;
            }
        }
        reported_size as i64
    }
}

/// `PTRACE_PEEKDATA` — read one machine word from `addr` in the tracee's mm.
///
/// Returns the word as a non-negative `i64` on success.  Linux convention is
/// to set `errno` and return -1; we return a negative errno directly because
/// kernel callers don't have access to `errno`.
unsafe fn ptrace_peekdata(pid: i32, addr: u64) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        let mm = (*target).mm;
        if mm.is_null() {
            // Kthread tracees have no mm — nothing to read.
            return EFAULT;
        }
        let len = core::mem::size_of::<usize>();
        if crate::mm::mmap::range_contains_secretmem(&*mm, addr, len) {
            return EFAULT;
        }
        let cur = sched::get_current();
        if cur.is_null() || (*cur).mm != mm {
            return EFAULT;
        }
        let mut word: usize = 0;
        let not_copied = copy_from_user(
            (&mut word as *mut usize).cast::<u8>(),
            (addr as *const usize).cast::<u8>(),
            len,
        );
        if not_copied != 0 {
            return EFAULT;
        }
        word as i64
    }
}

unsafe fn ptrace_pokedata(pid: i32, addr: u64, data: u64) -> i64 {
    let target = find_task_by_pid(pid);
    if target.is_null() {
        return ESRCH;
    }
    unsafe {
        if (*target).m26.ptrace & PT_PTRACED == 0 {
            return EINVAL;
        }
        let mm = (*target).mm;
        if mm.is_null() {
            return EFAULT;
        }
        let len = core::mem::size_of::<u64>();
        if crate::mm::mmap::range_contains_secretmem(&*mm, addr, len) {
            return EFAULT;
        }
        let cur = sched::get_current();
        if cur.is_null() || (*cur).mm != mm {
            return EFAULT;
        }
        let not_copied = copy_to_user(
            (addr as *mut u64).cast::<u8>(),
            (&data as *const u64).cast::<u8>(),
            len,
        );
        if not_copied != 0 {
            return EFAULT;
        }
    }
    0
}

pub unsafe fn syscall_trace_enter(
    task: *mut TaskStruct,
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
) {
    if task.is_null() {
        return;
    }
    unsafe {
        if (*task).m26.ptrace & PT_SYSCALL_TRACE == 0 {
            return;
        }
        (*task).m26.ptrace_syscall_op = PTRACE_SYSCALL_INFO_ENTRY;
        (*task).m26.ptrace_syscall_nr = regs.orig_rax as i64;
        (*task).m26.ptrace_syscall_args =
            [regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9];
        (*task).m26.ptrace_syscall_ret = 0;
        (*task).m26.ptrace_syscall_ip = regs.rip;
        (*task).m26.ptrace_syscall_sp = regs.rsp;
        let sig = if (*task).m26.ptrace & PT_TRACESYSGOOD != 0 {
            SIGTRAP | 0x80
        } else {
            SIGTRAP
        };
        ptrace_stop(task, sig, regs.orig_rax);
    }
}

pub unsafe fn syscall_trace_exit(
    task: *mut TaskStruct,
    regs: &crate::arch::x86::kernel::ptrace::PtRegs,
    ret: i64,
) {
    if task.is_null() {
        return;
    }
    unsafe {
        if (*task).m26.ptrace & PT_SYSCALL_TRACE == 0 {
            return;
        }
        (*task).m26.ptrace_syscall_op = PTRACE_SYSCALL_INFO_EXIT;
        (*task).m26.ptrace_syscall_nr = regs.orig_rax as i64;
        (*task).m26.ptrace_syscall_ret = ret;
        (*task).m26.ptrace_syscall_ip = regs.rip;
        (*task).m26.ptrace_syscall_sp = regs.rsp;
        let sig = if (*task).m26.ptrace & PT_TRACESYSGOOD != 0 {
            SIGTRAP | 0x80
        } else {
            SIGTRAP
        };
        ptrace_stop(task, sig, ret as u64);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    fn task(pid: i32) -> Box<TaskStruct> {
        let mut t = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        t.pid = pid;
        t.tgid = pid;
        t.m26 = crate::kernel::task::M26Fields::zeroed();
        t
    }

    #[test]
    fn ptrace_request_constants_match_linux() {
        assert_eq!(PTRACE_TRACEME, 0);
        assert_eq!(PTRACE_PEEKDATA, 2);
        assert_eq!(PTRACE_CONT, 7);
        assert_eq!(PTRACE_GETREGS, 12);
        assert_eq!(PTRACE_ATTACH, 16);
        assert_eq!(PTRACE_DETACH, 17);
        assert_eq!(PTRACE_SYSCALL, 24);
        assert_eq!(PTRACE_SEIZE, 0x4206);
    }

    #[test]
    fn unsupported_requests_return_einval() {
        // M65 supports the formerly-deferred ptrace requests; unknown requests
        // now follow Linux and return EINVAL.
        unsafe {
            assert_eq!(sys_ptrace(99999, 1, 0, 0), EINVAL);
        }
    }

    #[test]
    fn syscall_trace_stop_records_syscall_info() {
        let mut t = task(42);
        t.m26.ptrace = PT_PTRACED | PT_SYSCALL_TRACE | PT_TRACESYSGOOD;
        let regs = crate::arch::x86::kernel::ptrace::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rax: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            orig_rax: 39,
            rip: 0x401000,
            cs: 0,
            eflags: 0,
            rsp: 0x7fff0000,
            ss: 0,
        };

        unsafe {
            syscall_trace_enter(&mut *t as *mut TaskStruct, &regs);
        }

        assert_eq!(t.m26.ptrace_syscall_nr, 39);
        assert_eq!(t.m26.ptrace_stop_signal, SIGTRAP | 0x80);
        assert_eq!(
            t.__state.load(core::sync::atomic::Ordering::Acquire),
            __TASK_TRACED
        );
    }

    #[test]
    fn syscall_m76_process_control_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = task(86);
        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(sys_ptrace(PTRACE_TRACEME, 0, 0, 0), 0);
            assert_eq!(current.m26.ptrace & PT_PTRACED, PT_PTRACED);
            assert_eq!(sys_ptrace(PTRACE_ATTACH, 9999, 0, 0), ESRCH);
            assert_eq!(
                sys_ptrace(PTRACE_SETOPTIONS, 9999, 0, PTRACE_O_TRACESYSGOOD),
                ESRCH
            );
            assert_eq!(sys_ptrace(99999, 0, 0, 0), EINVAL);
            sched::set_current(previous);
        }
    }

    #[test]
    fn ptrace_usercopy_helpers_reject_kernel_pointers() {
        let message = 0xfeed_face_cafe_beefu64;
        unsafe {
            assert_eq!(
                copy_struct_to_user(0xffff_8000_0000_0000usize as *mut u64, &message),
                Err(EFAULT)
            );

            let mut regs = crate::arch::x86::kernel::ptrace::UserRegsStruct::default();
            assert_eq!(
                copy_struct_from_user(
                    &mut regs,
                    0xffff_8000_0000_0000usize
                        as *const crate::arch::x86::kernel::ptrace::UserRegsStruct,
                ),
                Err(EFAULT)
            );
        }
    }

    #[test]
    fn ptrace_traceme_records_real_parent_as_tracer() {
        let previous = unsafe { sched::get_current() };
        let mut parent = task(90);
        let mut current = task(91);
        current.m26.real_parent = &mut *parent as *mut TaskStruct;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(sys_ptrace(PTRACE_TRACEME, 0, 0, 0), 0);
            assert_eq!(current.m26.ptrace & PT_PTRACED, PT_PTRACED);
            assert_eq!(current.m26.tracer, &mut *parent as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
