//! linux-parity: partial
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Clone syscall API — Milestone 23.
//!
//! Defines the UAPI `struct clone_args`, all `CLONE_*` flag constants, the
//! three clone-family syscall handlers (`sys_fork`, `sys_clone`, `sys_clone3`),
//! and the argument-validation logic that mirrors Linux's `clone3_args_valid()`.
//!
//! # Scope
//!
//! - `CLONE_*` constants: bit-for-bit identical to Linux `uapi/linux/sched.h`.
//! - `CloneArgs`: UAPI struct layout identical to Linux (size = 88 bytes).
//! - `sys_fork` / `sys_clone` / `sys_clone3`: convert user args → `KernelCloneArgs`
//!   and call `fork::kernel_clone`.
//! - pidfd syscalls (`pidfd_open`, `pidfd_send_signal`, `pidfd_getfd`): stubs
//!   returning `-ENOSYS` until VFS lands in M39.
//!
//! # Deferred to later milestones
//! - `copy_from_user` / `access_ok` for safe user-pointer reads (M59).
//!   For now the syscall handlers assume they are called from kernel context
//!   with kernel-space `CloneArgs` pointers (testing only).
//! - Namespace flags (`CLONE_NEWPID`, `CLONE_NEWNS`, …): wired in M28.
//! - `CLONE_VFORK` completion waits in `fork::kernel_clone` until the child
//!   exits or replaces its mm, matching Linux's `wait_for_vfork_done()`.
//!
//! References:
//!   Linux `kernel/fork.c`
//!   Linux `include/uapi/linux/sched.h`
//!   Linux `tools/testing/selftests/clone3/`

use core::mem::size_of;

use crate::arch::x86::kernel::ptrace::PtRegs;
use crate::kernel::fork::{KernelCloneArgs, kernel_clone};

// ── CLONE_* flags (Linux uapi/linux/sched.h) ─────────────────────────────────

pub const CLONE_VM: u64 = 0x0000_0100;
pub const CLONE_FS: u64 = 0x0000_0200;
pub const CLONE_FILES: u64 = 0x0000_0400;
pub const CLONE_SIGHAND: u64 = 0x0000_0800;
pub const CLONE_NEWTIME: u64 = 0x0000_0080;
pub const CLONE_PIDFD: u64 = 0x0000_1000;
pub const CLONE_PTRACE: u64 = 0x0000_2000;
pub const CLONE_VFORK: u64 = 0x0000_4000;
pub const CLONE_PARENT: u64 = 0x0000_8000;
pub const CLONE_THREAD: u64 = 0x0001_0000;
pub const CLONE_NEWNS: u64 = 0x0002_0000;
pub const CLONE_SYSVSEM: u64 = 0x0004_0000;
pub const CLONE_SETTLS: u64 = 0x0008_0000;
pub const CLONE_PARENT_SETTID: u64 = 0x0010_0000;
pub const CLONE_CHILD_CLEARTID: u64 = 0x0020_0000;
pub const CLONE_DETACHED: u64 = 0x0040_0000;
pub const CLONE_UNTRACED: u64 = 0x0080_0000;
pub const CLONE_CHILD_SETTID: u64 = 0x0100_0000;
pub const CLONE_NEWCGROUP: u64 = 0x0200_0000;
pub const CLONE_NEWUTS: u64 = 0x0400_0000;
pub const CLONE_NEWIPC: u64 = 0x0800_0000;
pub const CLONE_NEWUSER: u64 = 0x1000_0000;
pub const CLONE_NEWPID: u64 = 0x2000_0000;
pub const CLONE_NEWNET: u64 = 0x4000_0000;
pub const CLONE_IO: u64 = 0x8000_0000;
/// Clear all signal handlers in the child (clone3-only).
pub const CLONE_CLEAR_SIGHAND: u64 = 1u64 << 32;
/// Place child in a specific cgroup (clone3-only).
pub const CLONE_INTO_CGROUP: u64 = 1u64 << 33;
/// Auto-reap the child when it exits without a waitpid (clone3-only).
pub const CLONE_AUTOREAP: u64 = 1u64 << 34;
/// Set no_new_privs on the child (clone3-only).
pub const CLONE_NNP: u64 = 1u64 << 35;
/// Kill the child when the clone pidfd is closed (clone3-only).
pub const CLONE_PIDFD_AUTOKILL: u64 = 1u64 << 36;
/// Create an empty mount namespace (clone3-only).
pub const CLONE_EMPTY_MNTNS: u64 = 1u64 << 37;

/// SIGCHLD signal number (Linux `signal.h`).
pub const SIGCHLD: i32 = 17;

// ── Versioned size constants ─────────────────────────────────────────────────

/// Minimum valid `clone_args` size (version 0, 64 bytes).
pub const CLONE_ARGS_SIZE_VER0: usize = 64;
/// Version 1 added `set_tid` / `set_tid_size` (80 bytes).
pub const CLONE_ARGS_SIZE_VER1: usize = 80;
/// Version 2 added `cgroup` (88 bytes).
pub const CLONE_ARGS_SIZE_VER2: usize = 88;

// ── CloneArgs UAPI struct ────────────────────────────────────────────────────

/// Linux UAPI `struct clone_args` from `uapi/linux/sched.h`.
///
/// All fields are `__aligned_u64` (8-byte aligned, 8-byte wide).
/// The layout is fixed: `size_of::<CloneArgs>()` must equal
/// `CLONE_ARGS_SIZE_VER2` (88 bytes).
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Default)]
pub struct CloneArgs {
    /// `CLONE_*` flag bitmask.
    pub flags: u64,
    /// FD location where pidfd is written (CLONE_PIDFD).
    pub pidfd: u64,
    /// User pointer written with child TID (CLONE_CHILD_SETTID / CLEARTID).
    pub child_tid: u64,
    /// User pointer written with child TID (CLONE_PARENT_SETTID).
    pub parent_tid: u64,
    /// Signal sent to parent on child exit (`SIGCHLD` for fork).
    pub exit_signal: u64,
    /// Stack base for the child (0 = inherit).
    pub stack: u64,
    /// Stack size (0 = inherit).
    pub stack_size: u64,
    /// TLS descriptor value (CLONE_SETTLS).
    pub tls: u64,
    /// Pointer to array of desired PIDs per namespace level (set_tid).
    pub set_tid: u64,
    /// Length of `set_tid` array.
    pub set_tid_size: u64,
    /// Cgroup FD for CLONE_INTO_CGROUP.
    pub cgroup: u64,
}

// Compile-time layout check: must be exactly 88 bytes (version 2).
const _: () = {
    assert!(
        size_of::<CloneArgs>() == CLONE_ARGS_SIZE_VER2,
        "CloneArgs size must be 88 bytes (CLONE_ARGS_SIZE_VER2)"
    );
    assert!(
        core::mem::offset_of!(CloneArgs, flags) == 0,
        "CloneArgs.flags must be at offset 0"
    );
    assert!(
        core::mem::offset_of!(CloneArgs, cgroup) == 80,
        "CloneArgs.cgroup must be at offset 80"
    );
};

// ── clone3_args_valid ─────────────────────────────────────────────────────────

/// Validate `clone_args` for `sys_clone3`.
///
/// Mirrors Linux `clone3_args_valid()` from `kernel/fork.c`.
///
/// # Errors
///
/// Returns `Err(errno)` where errno is a negative Linux error code:
/// - `-EINVAL` (`-22`): invalid flag combination, reserved flag, or bad sizes.
pub fn clone3_args_valid(args: &CloneArgs, size: usize) -> Result<(), i32> {
    const KNOWN_FLAGS: u64 = CLONE_VM
        | CLONE_FS
        | CLONE_FILES
        | CLONE_SIGHAND
        | CLONE_NEWTIME
        | CLONE_PIDFD
        | CLONE_PTRACE
        | CLONE_VFORK
        | CLONE_PARENT
        | CLONE_THREAD
        | CLONE_NEWNS
        | CLONE_SYSVSEM
        | CLONE_SETTLS
        | CLONE_PARENT_SETTID
        | CLONE_CHILD_CLEARTID
        | CLONE_UNTRACED
        | CLONE_CHILD_SETTID
        | CLONE_NEWCGROUP
        | CLONE_NEWUTS
        | CLONE_NEWIPC
        | CLONE_NEWUSER
        | CLONE_NEWPID
        | CLONE_NEWNET
        | CLONE_IO
        | CLONE_CLEAR_SIGHAND
        | CLONE_INTO_CGROUP
        | CLONE_AUTOREAP
        | CLONE_NNP
        | CLONE_PIDFD_AUTOKILL
        | CLONE_EMPTY_MNTNS;

    // Size must be at least VER0.
    if size < CLONE_ARGS_SIZE_VER0 {
        return Err(-22); // EINVAL
    }

    // clone3 reuses the legacy CSIGNAL bits for flags; only CLONE_NEWTIME is valid there.
    if args.flags & ((0xffu64) & !CLONE_NEWTIME) != 0 {
        return Err(-22);
    }
    if args.flags & !KNOWN_FLAGS != 0 {
        return Err(-22);
    }

    // CLONE_DETACHED is reserved; Linux removed it and returns EINVAL.
    if args.flags & CLONE_DETACHED != 0 {
        return Err(-22);
    }

    // CLONE_SIGHAND and CLONE_CLEAR_SIGHAND are mutually exclusive.
    if args.flags & CLONE_SIGHAND != 0 && args.flags & CLONE_CLEAR_SIGHAND != 0 {
        return Err(-22);
    }

    // CLONE_THREAD and CLONE_PARENT require exit_signal == 0 (the signal is
    // implicit: both join the parent's thread group).
    if args.flags & (CLONE_THREAD | CLONE_PARENT) != 0 && args.exit_signal != 0 {
        return Err(-22);
    }

    // Stack consistency: either both or neither of stack and stack_size must
    // be specified.
    if args.stack == 0 && args.stack_size != 0 {
        return Err(-22);
    }
    if args.stack != 0 && args.stack_size == 0 {
        return Err(-22);
    }

    // Validate flag implication chain.
    // CLONE_THREAD requires CLONE_SIGHAND.
    if args.flags & CLONE_THREAD != 0 && args.flags & CLONE_SIGHAND == 0 {
        return Err(-22);
    }
    // CLONE_SIGHAND requires CLONE_VM.
    if args.flags & CLONE_SIGHAND != 0 && args.flags & CLONE_VM == 0 {
        return Err(-22);
    }
    if args.exit_signal & !0xff != 0 {
        return Err(-22);
    }
    let exit_signal = args.exit_signal as i32;
    if exit_signal != 0 && !(1..=crate::kernel::signal::NSIG as i32).contains(&exit_signal) {
        return Err(-22);
    }
    if args.set_tid_size > 1 {
        return Err(-22);
    }
    if args.set_tid == 0 && args.set_tid_size != 0 {
        return Err(-22);
    }
    if args.set_tid != 0 && args.set_tid_size == 0 {
        return Err(-22);
    }
    if args.flags & CLONE_INTO_CGROUP != 0
        && (size < CLONE_ARGS_SIZE_VER2 || args.cgroup > i32::MAX as u64)
    {
        return Err(-22);
    }
    if args.flags & CLONE_AUTOREAP != 0
        && (args.flags & (CLONE_THREAD | CLONE_PARENT) != 0 || args.exit_signal != 0)
    {
        return Err(-22);
    }
    if args.flags & CLONE_NNP != 0 && args.flags & CLONE_THREAD != 0 {
        return Err(-22);
    }
    if args.flags & CLONE_PIDFD_AUTOKILL != 0
        && (args.flags & CLONE_PIDFD == 0
            || args.flags & CLONE_AUTOREAP == 0
            || args.flags & CLONE_THREAD != 0)
    {
        return Err(-22);
    }
    if args.flags & CLONE_PIDFD != 0
        && args.flags & CLONE_PARENT_SETTID != 0
        && args.pidfd == args.parent_tid
    {
        return Err(-22);
    }

    // ── M28: namespace flag validation ──────────────────────────────────────
    // Linux semantics:
    //   - CLONE_NEWPID is incompatible with CLONE_THREAD: a thread cannot land
    //     in a different PID namespace from its thread group leader.
    //   - CLONE_NEWNS / CLONE_NEWUSER / etc. cannot be combined with
    //     CLONE_THREAD (the new namespace would only apply to the new thread,
    //     which contradicts the shared mm/sighand requirement).
    //   - CLONE_NEWUSER cannot be combined with CLONE_FS (Linux explicitly
    //     forbids it because shared `fs_struct` would leak the parent's root
    //     into the user_ns).
    const NS_FLAGS_ALL: u64 = CLONE_NEWNS
        | CLONE_NEWIPC
        | CLONE_NEWUTS
        | CLONE_NEWPID
        | CLONE_NEWNET
        | CLONE_NEWUSER
        | CLONE_NEWCGROUP;
    if args.flags & CLONE_THREAD != 0 && args.flags & NS_FLAGS_ALL != 0 {
        return Err(-22);
    }
    if args.flags & CLONE_NEWUSER != 0 && args.flags & CLONE_FS != 0 {
        return Err(-22);
    }

    Ok(())
}

// ── Syscall handlers ──────────────────────────────────────────────────────────

/// `sys_fork` — traditional `fork()` syscall.
///
/// Creates a child that shares nothing with the parent (no CLONE_VM, etc.)
/// and receives `SIGCHLD` when it exits.
///
/// Mirrors Linux `SYSCALL_DEFINE0(fork)` in `kernel/fork.c`.
///
/// # Safety
/// Must be called from a valid task context (after `sched_init()`).
pub unsafe fn sys_fork() -> i64 {
    unsafe { sys_fork_with_regs(None) }
}

pub unsafe fn sys_fork_with_regs(user_regs: Option<PtRegs>) -> i64 {
    let args = KernelCloneArgs {
        flags: 0,
        exit_signal: SIGCHLD,
        user_regs,
        ..KernelCloneArgs::default()
    };
    let ret = unsafe { kernel_clone(&args) };
    #[cfg(not(test))]
    if ret > 1 && crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { crate::kernel::sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-clone parent={} flags={:#x} child={}",
            pid,
            0,
            ret
        );
    }
    ret
}

pub unsafe fn sys_vfork_with_regs(user_regs: Option<PtRegs>) -> i64 {
    let args = KernelCloneArgs {
        flags: CLONE_VM | CLONE_VFORK,
        exit_signal: SIGCHLD,
        user_regs,
        ..KernelCloneArgs::default()
    };
    unsafe { kernel_clone(&args) }
}

/// `sys_clone` — legacy 5-argument `clone()` syscall.
///
/// Arguments correspond to the x86_64 syscall register layout:
/// - `flags`:        CLONE_* bitmask (low 32 bits include the exit signal).
/// - `newsp`:        child stack pointer (0 = inherit parent SP).
/// - `parent_tid`:   user pointer for CLONE_PARENT_SETTID.
/// - `child_tid`:    user pointer for CLONE_CHILD_SETTID / CLEARTID.
/// - `tls`:          TLS base value for CLONE_SETTLS.
///
/// Mirrors Linux `SYSCALL_DEFINE5(clone, …)` in `kernel/fork.c`.
///
/// # Safety
/// Must be called from a valid task context.
pub unsafe fn sys_clone(
    flags: u64,
    newsp: u64,
    parent_tid: *mut i32,
    child_tid: *mut i32,
    tls: u64,
) -> i64 {
    unsafe { sys_clone_with_regs(flags, newsp, parent_tid, child_tid, tls, None) }
}

pub unsafe fn sys_clone_with_regs(
    flags: u64,
    newsp: u64,
    parent_tid: *mut i32,
    child_tid: *mut i32,
    tls: u64,
    user_regs: Option<PtRegs>,
) -> i64 {
    let exit_signal = (flags & 0xFF) as i32; // low byte of flags_lo
    let clone_flags = flags & !0xFF;
    if clone_flags & CLONE_PIDFD != 0 && clone_flags & CLONE_PARENT_SETTID != 0 {
        return -22;
    }
    let args = KernelCloneArgs {
        flags: clone_flags,
        pidfd: if clone_flags & CLONE_PIDFD != 0 {
            parent_tid
        } else {
            core::ptr::null_mut()
        },
        parent_tid: if clone_flags & CLONE_PIDFD != 0 {
            core::ptr::null_mut()
        } else {
            parent_tid
        },
        child_tid,
        exit_signal,
        stack: newsp,
        tls,
        user_regs,
        ..KernelCloneArgs::default()
    };
    let ret = unsafe { kernel_clone(&args) };
    #[cfg(not(test))]
    if ret > 1 && crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { crate::kernel::sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-clone parent={} flags={:#x} child={}",
            pid,
            flags,
            ret
        );
    }
    ret
}

/// `sys_clone3` — new 2-argument `clone3()` syscall.
///
/// `uargs` is a pointer to a `CloneArgs` struct (kernel-space in M23;
/// user-space `copy_from_user` arrives in M59).
/// `size` is the caller-provided struct size for version detection.
///
/// Validates the arguments via `clone3_args_valid` before delegating to
/// `kernel_clone`.
///
/// Mirrors Linux `SYSCALL_DEFINE2(clone3, …)` in `kernel/fork.c`.
///
/// # Safety
/// `uargs` must be a valid non-null pointer to a `CloneArgs`.
/// Must be called from a valid task context.
pub unsafe fn sys_clone3(uargs: *const CloneArgs, size: usize) -> i64 {
    unsafe { sys_clone3_with_regs(uargs, size, None) }
}

pub unsafe fn sys_clone3_with_regs(
    uargs: *const CloneArgs,
    size: usize,
    user_regs: Option<PtRegs>,
) -> i64 {
    const E2BIG: i64 = -7;
    const PAGE_SIZE: usize = 4096;

    if uargs.is_null() {
        return -14; // EFAULT
    }
    if size > PAGE_SIZE {
        return E2BIG;
    }
    if size > 4096 {
        return -22; // EINVAL — absurdly large
    }
    let args = unsafe { *uargs }; // copy from user (kernel pointer in M23)
    let mut copied_args = CloneArgs::default();
    let copy_len = size.min(core::mem::size_of::<CloneArgs>());
    unsafe {
        core::ptr::copy_nonoverlapping(
            uargs as *const u8,
            (&mut copied_args as *mut CloneArgs).cast::<u8>(),
            copy_len,
        );
    }
    if size > core::mem::size_of::<CloneArgs>() {
        let extra = size - core::mem::size_of::<CloneArgs>();
        let extra_ptr = unsafe { (uargs as *const u8).add(core::mem::size_of::<CloneArgs>()) };
        let extra_bytes = unsafe { core::slice::from_raw_parts(extra_ptr, extra) };
        if extra_bytes.iter().any(|byte| *byte != 0) {
            return E2BIG;
        }
    }
    let args = copied_args;

    if let Err(e) = clone3_args_valid(&args, size) {
        return e as i64;
    }

    // Extract set_tid[0] for level-0 namespace if set_tid is provided.
    let set_tid = if args.set_tid_size > 0 && args.set_tid != 0 {
        // SAFETY: M23 only supports a single namespace level; read first entry.
        let tid_ptr = args.set_tid as *const i32;
        if tid_ptr.is_null() {
            None
        } else {
            Some(unsafe { *tid_ptr })
        }
    } else {
        None
    };

    let kargs = KernelCloneArgs {
        flags: args.flags,
        pidfd: args.pidfd as *mut i32,
        child_tid: args.child_tid as *mut i32,
        parent_tid: args.parent_tid as *mut i32,
        exit_signal: args.exit_signal as i32,
        kthread: 0,
        stack: args.stack,
        stack_size: args.stack_size,
        tls: args.tls,
        set_tid,
        cgroup: args.cgroup as i32,
        fn_ptr: None,
        fn_arg: core::ptr::null_mut(),
        user_regs,
    };

    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { crate::kernel::sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-clone3-enter pid={} flags={:#x} exit_signal={} pidfd={:#x} size={}",
            pid,
            args.flags,
            args.exit_signal,
            args.pidfd,
            size
        );
    }
    let ret = unsafe { kernel_clone(&kargs) };
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        crate::linux_driver_abi::tty::serial_println!("trace-proc-clone3-ret ret={}", ret);
    }
    ret
}

// ── pidfd stubs (VFS arrives in M39) ─────────────────────────────────────────

/// `sys_pidfd_open` — stub returning `-ENOSYS` until M39.
pub unsafe fn sys_pidfd_open(_pid: i32, _flags: u32) -> i64 {
    -38 // ENOSYS
}

/// `sys_pidfd_send_signal` — stub returning `-ENOSYS` until M25+M39.
pub unsafe fn sys_pidfd_send_signal(_pidfd: i32, _sig: i32, _info: *const (), _flags: u32) -> i64 {
    -38
}

/// `sys_pidfd_getfd` — stub returning `-ENOSYS` until M39.
pub unsafe fn sys_pidfd_getfd(_pidfd: i32, _targetfd: i32, _flags: u32) -> i64 {
    -38
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── CloneArgs layout — mirrors Linux uapi/linux/sched.h ─────────────────

    #[test]
    fn clone_args_size_ver2_is_88() {
        assert_eq!(size_of::<CloneArgs>(), CLONE_ARGS_SIZE_VER2);
        assert_eq!(size_of::<CloneArgs>(), 88);
    }

    #[test]
    fn syscall_m76_process_control_parity() {
        assert_eq!(
            unsafe { sys_clone3(core::ptr::null(), CLONE_ARGS_SIZE_VER0) },
            -14
        );
        let args = CloneArgs {
            flags: CLONE_DETACHED,
            ..CloneArgs::default()
        };
        assert_eq!(unsafe { sys_clone3(&args, CLONE_ARGS_SIZE_VER2) }, -22);
        assert_eq!(unsafe { sys_clone3(&CloneArgs::default(), 4097) }, -7);
        let _: unsafe fn() -> i64 = sys_fork;
        let _: unsafe fn(u64, u64, *mut i32, *mut i32, u64) -> i64 = sys_clone;
        let _: unsafe fn(*const CloneArgs, usize) -> i64 = sys_clone3;
        let _: unsafe fn(*const i8, *const *const i8, *const *const i8) -> i64 =
            crate::kernel::exec::sys_execve;
        let _: unsafe fn(i32, *const i8, *const *const i8, *const *const i8, i32) -> i64 =
            crate::kernel::exec::sys_execveat;
    }

    #[test]
    fn clone_args_flags_at_offset_0() {
        assert_eq!(core::mem::offset_of!(CloneArgs, flags), 0);
    }

    #[test]
    fn clone_args_exit_signal_at_offset_32() {
        assert_eq!(core::mem::offset_of!(CloneArgs, exit_signal), 32);
    }

    #[test]
    fn clone_args_cgroup_at_offset_80() {
        assert_eq!(core::mem::offset_of!(CloneArgs, cgroup), 80);
    }

    #[test]
    fn clone_args_tls_at_offset_56() {
        assert_eq!(core::mem::offset_of!(CloneArgs, tls), 56);
    }

    // ── CLONE_* flag values match Linux uapi ─────────────────────────────────
    // These are spot-checked against `uapi/linux/sched.h`.

    #[test]
    fn clone_flags_match_linux_uapi() {
        assert_eq!(CLONE_VM, 0x0000_0100u64);
        assert_eq!(CLONE_FS, 0x0000_0200u64);
        assert_eq!(CLONE_FILES, 0x0000_0400u64);
        assert_eq!(CLONE_SIGHAND, 0x0000_0800u64);
        assert_eq!(CLONE_PIDFD, 0x0000_1000u64);
        assert_eq!(CLONE_THREAD, 0x0001_0000u64);
        assert_eq!(CLONE_NEWNS, 0x0002_0000u64);
        assert_eq!(CLONE_SETTLS, 0x0008_0000u64);
        assert_eq!(CLONE_PARENT_SETTID, 0x0010_0000u64);
        assert_eq!(CLONE_CHILD_CLEARTID, 0x0020_0000u64);
        assert_eq!(CLONE_DETACHED, 0x0040_0000u64);
        assert_eq!(CLONE_CHILD_SETTID, 0x0100_0000u64);
        assert_eq!(CLONE_NEWPID, 0x2000_0000u64);
        assert_eq!(CLONE_CLEAR_SIGHAND, 1u64 << 32);
        assert_eq!(CLONE_INTO_CGROUP, 1u64 << 33);
        assert_eq!(CLONE_NNP, 1u64 << 35);
        assert_eq!(CLONE_PIDFD_AUTOKILL, 1u64 << 36);
        assert_eq!(CLONE_EMPTY_MNTNS, 1u64 << 37);
    }

    #[test]
    fn sigchld_is_17() {
        assert_eq!(SIGCHLD, 17);
    }

    // ── clone3_args_valid — mirrors Linux clone3 selftests error cases ───────

    fn valid_base() -> (CloneArgs, usize) {
        (CloneArgs::default(), CLONE_ARGS_SIZE_VER2)
    }

    #[test]
    fn clone3_accepts_valid_args() {
        let (args, size) = valid_base();
        assert_eq!(clone3_args_valid(&args, size), Ok(()));
    }

    #[test]
    fn clone3_accepts_modern_pidfd_spawn_flags() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_PIDFD | CLONE_NEWNS | CLONE_NNP;
        args.pidfd = 0x1000;
        args.exit_signal = SIGCHLD as u64;
        assert_eq!(clone3_args_valid(&args, size), Ok(()));
    }

    #[test]
    fn clone3_rejects_pidfd_autokill_without_autoreap() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_PIDFD | CLONE_PIDFD_AUTOKILL;
        args.pidfd = 0x1000;
        args.exit_signal = SIGCHLD as u64;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_size_below_ver0() {
        let (args, _) = valid_base();
        assert_eq!(clone3_args_valid(&args, 0), Err(-22));
        assert_eq!(clone3_args_valid(&args, 63), Err(-22));
    }

    #[test]
    fn clone3_accepts_size_at_ver0() {
        let (args, _) = valid_base();
        assert_eq!(clone3_args_valid(&args, CLONE_ARGS_SIZE_VER0), Ok(()));
    }

    #[test]
    fn clone3_rejects_detached_flag() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_DETACHED;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_sighand_and_clear_sighand_together() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_VM | CLONE_SIGHAND | CLONE_CLEAR_SIGHAND;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_thread_without_sighand() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_VM | CLONE_THREAD; // missing CLONE_SIGHAND
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_sighand_without_vm() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_SIGHAND; // missing CLONE_VM
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_accepts_full_thread_flags() {
        let (mut args, size) = valid_base();
        // CLONE_VM | CLONE_SIGHAND | CLONE_THREAD: the canonical new-thread combo.
        args.flags = CLONE_VM | CLONE_SIGHAND | CLONE_THREAD;
        assert_eq!(clone3_args_valid(&args, size), Ok(()));
    }

    #[test]
    fn clone3_rejects_thread_with_nonzero_exit_signal() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_VM | CLONE_SIGHAND | CLONE_THREAD;
        args.exit_signal = SIGCHLD as u64;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_parent_with_nonzero_exit_signal() {
        let (mut args, size) = valid_base();
        args.flags = CLONE_PARENT;
        args.exit_signal = SIGCHLD as u64;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_exit_signal_bits_in_flags() {
        let (mut args, size) = valid_base();
        args.flags = SIGCHLD as u64;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_stack_without_size() {
        let (mut args, size) = valid_base();
        args.stack = 0xdeadbeef;
        args.stack_size = 0;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_rejects_size_without_stack() {
        let (mut args, size) = valid_base();
        args.stack = 0;
        args.stack_size = 4096;
        assert_eq!(clone3_args_valid(&args, size), Err(-22));
    }

    #[test]
    fn clone3_accepts_both_stack_and_size() {
        let (mut args, size) = valid_base();
        args.stack = 0xdeadbeef;
        args.stack_size = 4096;
        assert_eq!(clone3_args_valid(&args, size), Ok(()));
    }

    // ── Size version constants ────────────────────────────────────────────────

    #[test]
    fn clone_args_size_constants() {
        assert_eq!(CLONE_ARGS_SIZE_VER0, 64);
        assert_eq!(CLONE_ARGS_SIZE_VER1, 80);
        assert_eq!(CLONE_ARGS_SIZE_VER2, 88);
    }
}
