//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/signal.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/signal.c
//! test-origin: linux:vendor/linux/tools/testing/selftests/x86/xstate.c
//! x86_64 signal frame setup and restoration.
//!
//! When a signal is delivered to a user-space task, the kernel constructs
//! a "signal frame" on the user stack containing:
//! 1. The signal handler's address (synthesised by the kernel)
//! 2. A `struct rt_sigframe` (ucontext + siginfo)
//! 3. Return address (`sa_restorer`) that the handler calls to return
//!
//! Layout (from `arch/x86/kernel/signal.c` and `uapi/asm/sigcontext.h`):
//! ```
//! [pretcode/sa_restorer]  <- signal stack frame start (8 bytes)
//! [ucontext_t]            <- includes uc_mcontext (SigContext)
//! [siginfo_t]
//! [alignment padding]
//! [64-byte-aligned fpstate]
//! ```
//!
//! References:
//!   Linux `arch/x86/include/uapi/asm/sigcontext.h`
//!   Linux `arch/x86/include/uapi/asm/ucontext.h`
//!   vendor/linux/arch/x86/kernel/signal.c
//!   vendor/linux/arch/x86/kernel/signal_64.c

use crate::kernel::signal::{SigAltStack, SigInfo, SigSet};
use crate::kernel::task::PtRegs;

/// Machine context (register state) saved in signal frame.
///
/// Matches `struct sigcontext` in Linux `uapi/asm/sigcontext.h`.
/// Total size: 256 bytes (conservative estimate; actual is ~232).
#[repr(C)]
pub struct SigContext {
    // General-purpose registers (in same order as PtRegs for easy copying).
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rsp: u64,
    pub rip: u64,
    pub eflags: u64,
    // Segment registers (stored as u16, padded to u64).
    pub cs: u16,
    pub gs: u16,
    pub fs: u16,
    pub ss: u16,
    // Exception context.
    pub err: u64,
    pub trapno: u64,
    pub oldmask: u64,
    pub cr2: u64,
    pub fpstate: u64,
    pub reserved1: [u64; 8],
}

/// User context — includes the machine context plus signal mask and alternate stack.
///
/// Matches `struct ucontext` in Linux `uapi/asm/ucontext.h`.
#[repr(C)]
pub struct UContext {
    pub uc_flags: u64,
    pub uc_link: u64,
    pub uc_stack: SigAltStack,
    pub uc_mcontext: SigContext,
    pub uc_sigmask: SigSet,
}

/// Real-time signal frame pushed onto the user stack.
///
/// The layout matches `struct rt_sigframe` in Linux `arch/x86/kernel/signal.c`.
/// When a signal handler is invoked, RSI points to `&info`, RDX points to `&uc`.
#[repr(C)]
pub struct RtSigFrame {
    /// Pointer to signal handler's return code (restorer).
    /// Set to `action.sa_restorer` by the kernel.
    pub pretcode: u64,
    /// User context — includes register state (SigContext) and signal mask.
    pub uc: UContext,
    /// Signal info — contains signal number, si_code, si_value, etc.
    pub info: SigInfo,
}

const FRAME_ALIGNMENT: u64 = 16;
const REDZONE_SIZE: u64 = 128;
const UC_FP_XSTATE: u64 = 0x1;
const UC_SIGCONTEXT_SS: u64 = 0x2;
const UC_STRICT_RESTORE_SS: u64 = 0x4;

fn x64_rt_sigframe_sp(user_sp: u64, frame_size: usize) -> u64 {
    x64_rt_sigframe_layout(
        user_sp,
        frame_size,
        crate::arch::x86::kernel::fpu_signal::signal_fpstate_size(),
    )
    .map(|(frame_sp, _)| frame_sp)
    .unwrap_or(0)
}

fn x64_rt_sigframe_layout(
    user_sp: u64,
    frame_size: usize,
    fpstate_size: usize,
) -> Option<(u64, u64)> {
    // Linux get_sigframe(): redzone, fpu__alloc_mathframe(), rt_sigframe,
    // then x86-64 function-entry alignment.
    let sp = user_sp.checked_sub(REDZONE_SIZE)?;
    let fpstate_sp =
        crate::arch::x86::kernel::fpu_signal::round_down_64(sp.checked_sub(fpstate_size as u64)?);
    let sp = fpstate_sp.checked_sub(frame_size as u64)?;
    let frame_sp = (sp & !(FRAME_ALIGNMENT - 1)).checked_sub(8)?;
    Some((frame_sp, fpstate_sp))
}

/// Copy one scalar user-frame field without materializing the complete signal
/// frame on the current task's kernel stack.
///
/// Linux's `unsafe_put_user()` sequence in `x64_setup_rt_frame()` writes each
/// field directly. Keeping this helper scalar-sized is important: signal
/// delivery runs on a live task stack and a full `RtSigFrame` temporary can
/// overlap the entry/return state being protected by that stack.
#[inline]
unsafe fn put_user_frame_value<T: Copy>(frame_sp: u64, offset: usize, value: T) -> Result<(), i32> {
    let Some(dst) = frame_sp.checked_add(offset as u64) else {
        return Err(-14); // EFAULT
    };
    let value_ptr = &value as *const T;
    match core::mem::size_of::<T>() {
        2 => crate::arch::x86::kernel::uaccess::put_user_u16_nofault(dst as *mut u16, unsafe {
            value_ptr.cast::<u16>().read_unaligned()
        }),
        4 => crate::arch::x86::kernel::uaccess::put_user_u32_nofault(dst as *mut u32, unsafe {
            value_ptr.cast::<u32>().read_unaligned()
        }),
        8 => crate::arch::x86::kernel::uaccess::put_user_u64_nofault(dst as *mut u64, unsafe {
            value_ptr.cast::<u64>().read_unaligned()
        }),
        _ => Err(-14), // EFAULT: only Linux scalar fields are supported here.
    }
}

/// Copy a source object into one user-frame field without creating a kernel
/// copy of the object. This is used for `siginfo_t`, which is large enough
/// that a by-value temporary would recreate the problem this path avoids.
#[inline]
unsafe fn put_user_frame_bytes(
    frame_sp: u64,
    offset: usize,
    src: *const u8,
    len: usize,
) -> Result<(), i32> {
    let Some(dst) = frame_sp.checked_add(offset as u64) else {
        return Err(-14); // EFAULT
    };
    let not_copied =
        unsafe { crate::arch::x86::kernel::uaccess::copy_to_user(dst as *mut u8, src, len) };
    if not_copied == 0 {
        Ok(())
    } else {
        Err(-14) // EFAULT
    }
}

/// Set up the signal frame on the user stack.
///
/// Constructs the user-visible `RtSigFrame` in place, then modifies `regs` to
/// point to the signal handler.
///
/// # Arguments
/// - `regs` — mutable pointer to the current `PtRegs` (from syscall entry or interrupt).
///            Will be modified to set RIP = handler, RSP = frame, RDI = signum, etc.
/// - `signum` — signal number (1–64)
/// - `action` — pointer to the `RtSigAction` for this signal
/// - `info` — pointer to the `SigInfo` for this signal
///
/// # Safety
/// - `regs` must point to valid writable kernel memory (the interrupted context).
/// - `action` must point to a valid `RtSigAction` structure.
/// - `info` must point to a valid `SigInfo` structure.
/// - The user stack pointer (from `regs.sp`) must have enough space for `RtSigFrame`.
///
/// # Returns
/// - `Ok(())` on success
/// - `Err(EFAULT)` if user memory access fails (e.g., stack overflow)
pub unsafe fn setup_rt_frame(
    regs: *mut PtRegs,
    signum: i32,
    action: *const crate::kernel::signal::RtSigAction,
    info: *const SigInfo,
    mask: SigSet,
) -> Result<(), i32> {
    // Linux `get_sigframe()` honors the x86-64 red zone, allocates the frame,
    // then leaves `%rsp % 16 == 8` for function-entry ABI alignment.
    let frame_size = core::mem::size_of::<RtSigFrame>();
    let fpstate_size = crate::arch::x86::kernel::fpu_signal::signal_fpstate_size();
    let saved_altstack = crate::kernel::signal::current_altstack_for_signal();
    let nested_altstack = crate::kernel::signal::altstack_on_sig_stack(saved_altstack, (*regs).sp);
    let entering_altstack = (*action).sa_flags & crate::kernel::signal::SA_ONSTACK != 0
        && !nested_altstack
        && saved_altstack.ss_size != 0;
    let frame_input_sp = if entering_altstack {
        saved_altstack
            .ss_sp
            .checked_add(saved_altstack.ss_size)
            .ok_or(-14)? as u64
    } else {
        (*regs).sp
    };
    let Some((user_sp, fpstate_sp)) =
        x64_rt_sigframe_layout(frame_input_sp, frame_size, fpstate_size)
    else {
        return Err(-14); // EFAULT
    };

    // Linux rejects a frame which would overflow an entered alternate stack
    // after the red-zone/math-frame allocation.
    if entering_altstack && !crate::kernel::signal::altstack_contains(saved_altstack, user_sp) {
        return Err(-14);
    }

    // Verify we're not going off the edge of the stack.
    if user_sp == 0
        || !crate::arch::x86::kernel::uaccess::access_ok(user_sp, frame_size as u64)
        || !unsafe { crate::arch::x86::kernel::fpu_signal::copy_fpstate_to_sigframe(fpstate_sp) }
    {
        return Err(-14); // EFAULT
    }

    // Linux writes the user frame in place with unsafe_put_user(). Keep the
    // same order and do not create a complete RtSigFrame temporary on the
    // current task's kernel stack.
    let uc_offset = core::mem::offset_of!(RtSigFrame, uc);
    let uc_stack_offset = uc_offset + core::mem::offset_of!(UContext, uc_stack);
    let sc_offset = uc_offset + core::mem::offset_of!(UContext, uc_mcontext);
    let sigmask_offset = uc_offset + core::mem::offset_of!(UContext, uc_sigmask);

    // 1. Create the ucontext: flags, link, and alternate-stack state.
    let mut uc_flags = UC_SIGCONTEXT_SS | UC_STRICT_RESTORE_SS;
    if crate::arch::x86::kernel::fpu::signal_uses_xsave() {
        uc_flags |= UC_FP_XSTATE;
    }

    unsafe {
        put_user_frame_value(
            user_sp,
            uc_offset + core::mem::offset_of!(UContext, uc_flags),
            uc_flags,
        )?;
        put_user_frame_value(
            user_sp,
            uc_offset + core::mem::offset_of!(UContext, uc_link),
            0u64,
        )?;
        put_user_frame_value(
            user_sp,
            uc_stack_offset + core::mem::offset_of!(SigAltStack, ss_sp),
            saved_altstack.ss_sp,
        )?;
        put_user_frame_value(
            user_sp,
            uc_stack_offset + core::mem::offset_of!(SigAltStack, ss_flags),
            saved_altstack.ss_flags,
        )?;
        put_user_frame_value(
            user_sp,
            uc_stack_offset + core::mem::offset_of!(SigAltStack, ss_size),
            saved_altstack.ss_size,
        )?;

        // 2. Set sa_restorer (the return code address).
        put_user_frame_value(
            user_sp,
            core::mem::offset_of!(RtSigFrame, pretcode),
            (*action).sa_restorer as u64,
        )?;

        // 3. Copy machine context from current PtRegs in Linux's field order.
        let regs_ref = &*regs;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rdi),
            regs_ref.di,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rsi),
            regs_ref.si,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rbp),
            regs_ref.bp,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rsp),
            regs_ref.sp,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rbx),
            regs_ref.bx,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rdx),
            regs_ref.dx,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rcx),
            regs_ref.cx,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rax),
            regs_ref.ax,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r8),
            regs_ref.r8,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r9),
            regs_ref.r9,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r10),
            regs_ref.r10,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r11),
            regs_ref.r11,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r12),
            regs_ref.r12,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r13),
            regs_ref.r13,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r14),
            regs_ref.r14,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, r15),
            regs_ref.r15,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, err),
            regs_ref.orig_ax,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, trapno),
            0u64,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, rip),
            regs_ref.ip,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, eflags),
            regs_ref.flags,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, cs),
            regs_ref.cs as u16,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, gs),
            0u16,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, fs),
            0u16,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, ss),
            regs_ref.ss as u16,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, fpstate),
            fpstate_sp,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, oldmask),
            mask.bits,
        )?;
        put_user_frame_value(
            user_sp,
            sc_offset + core::mem::offset_of!(SigContext, cr2),
            0u64,
        )?;
        put_user_frame_value(user_sp, sigmask_offset, mask.bits)?;

        // Linux copies siginfo only for SA_SIGINFO. RSI still points at the
        // fixed frame member for traditional handlers.
        if (*action).sa_flags & crate::kernel::signal::SA_SIGINFO != 0 {
            put_user_frame_bytes(
                user_sp,
                core::mem::offset_of!(RtSigFrame, info),
                info.cast::<u8>(),
                core::mem::size_of::<SigInfo>(),
            )?;
        }
    }

    // 4. Modify PtRegs to transfer control to the signal handler.
    let regs_mut = unsafe { &mut *regs };
    regs_mut.ip = (*action).sa_handler as u64; // RIP = signal handler entry
    regs_mut.sp = user_sp; // RSP = frame base
    regs_mut.di = signum as u64; // RDI = signal number (arg 0)
    regs_mut.ax = 0; // Linux clears AX in case the handler lacks prototypes.
    regs_mut.si = user_sp
        .checked_add(core::mem::offset_of!(RtSigFrame, info) as u64)
        .ok_or(-14)?; // RSI = &siginfo_t
    regs_mut.dx = user_sp
        .checked_add(core::mem::offset_of!(RtSigFrame, uc) as u64)
        .ok_or(-14)?; // RDX = &ucontext_t
    regs_mut.cs = crate::arch::x86::kernel::gdt::sel::USER_CS as u64;
    if regs_mut.ss & 0x3 != 0x3 {
        regs_mut.ss = crate::arch::x86::kernel::gdt::sel::USER_DS as u64;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::signal::RtSigAction;

    fn sample_regs(stack_top: u64) -> crate::kernel::task::PtRegs {
        crate::kernel::task::PtRegs {
            r15: 15,
            r14: 14,
            r13: 13,
            r12: 12,
            bp: 5,
            bx: 4,
            r11: 11,
            r10: 10,
            r9: 9,
            r8: 8,
            ax: 0,
            cx: 1,
            dx: 2,
            si: 3,
            di: 4,
            orig_ax: 39,
            ip: 0x401000,
            cs: 0x33,
            flags: 0x202,
            sp: stack_top,
            ss: 0x2b,
        }
    }

    #[test]
    fn sig_context_layout() {
        assert_eq!(core::mem::size_of::<SigContext>(), 256);
    }

    #[test]
    fn rt_sig_frame_layout() {
        // Verify RtSigFrame contains all required fields.
        assert_eq!(
            core::mem::offset_of!(RtSigFrame, pretcode),
            0,
            "pretcode must be at offset 0"
        );
        assert_eq!(
            core::mem::offset_of!(RtSigFrame, uc),
            8,
            "uc must follow pretcode"
        );
        assert_eq!(core::mem::size_of::<SigInfo>(), 128);
        assert_eq!(
            core::mem::offset_of!(RtSigFrame, info),
            312,
            "info must follow the Linux-sized ucontext"
        );
    }

    #[test]
    fn setup_rt_frame_points_handler_args_at_user_frame_members() {
        let mut stack = [0u8; 4096];
        let stack_top = unsafe { stack.as_mut_ptr().add(stack.len()) as u64 };
        let mut regs = sample_regs(stack_top);
        let action = RtSigAction {
            sa_handler: 0x5000,
            sa_flags: crate::kernel::signal::SA_SIGINFO,
            sa_restorer: 0x6000,
            sa_mask: SigSet { bits: 0x55 },
        };
        let mut info = SigInfo::default();
        info.signo = 10;
        info.code = 1;

        unsafe {
            setup_rt_frame(
                &mut regs as *mut crate::kernel::task::PtRegs,
                10,
                &action,
                &info,
                action.sa_mask,
            )
            .unwrap();
        }

        assert_eq!(regs.ip, action.sa_handler as u64);
        assert_eq!(regs.sp & 0xF, 8);
        assert_eq!(regs.di, 10);
        assert_eq!(regs.ax, 0);
        assert_eq!(
            regs.si,
            regs.sp + core::mem::offset_of!(RtSigFrame, info) as u64
        );
        assert_eq!(
            regs.dx,
            regs.sp + core::mem::offset_of!(RtSigFrame, uc) as u64
        );

        let frame = unsafe { &*(regs.sp as *const RtSigFrame) };
        assert_eq!(frame.pretcode, action.sa_restorer as u64);
        assert_eq!(frame.uc.uc_mcontext.rip, 0x401000);
        assert_eq!(frame.uc.uc_mcontext.oldmask, action.sa_mask.bits);
        assert_eq!(frame.uc.uc_sigmask, action.sa_mask);
        let expected_uc_flags = UC_SIGCONTEXT_SS
            | UC_STRICT_RESTORE_SS
            | if crate::arch::x86::kernel::fpu::signal_uses_xsave() {
                UC_FP_XSTATE
            } else {
                0
            };
        assert_eq!(frame.uc.uc_flags, expected_uc_flags);
        assert_eq!(frame.info.signo, 10);

        let fpstate = frame.uc.uc_mcontext.fpstate;
        assert_ne!(fpstate, 0);
        assert_eq!(fpstate & 63, 0);
        assert!(fpstate >= regs.sp + core::mem::size_of::<RtSigFrame>() as u64);
        assert!(
            fpstate + crate::arch::x86::kernel::fpu_signal::signal_fpstate_size() as u64
                <= stack_top - REDZONE_SIZE
        );
        let sw = unsafe {
            &*((fpstate + crate::arch::x86::kernel::fpu_signal::FXSAVE_SW_RESERVED_OFFSET as u64)
                as *const crate::arch::x86::kernel::fpu_signal::FpxSwBytes)
        };
        assert_eq!(
            sw.magic1,
            crate::arch::x86::kernel::fpu_signal::FP_XSTATE_MAGIC1
        );
        if crate::arch::x86::kernel::fpu::signal_uses_xsave() {
            let magic2 = unsafe { *((fpstate + sw.xstate_size as u64) as *const u32) };
            assert_eq!(
                magic2,
                crate::arch::x86::kernel::fpu_signal::FP_XSTATE_MAGIC2
            );
        }
    }

    #[test]
    fn setup_rt_frame_matches_linux_redzone_alignment() {
        let stack_top = 0x7fff_ffff_f000u64;
        let frame_sp = x64_rt_sigframe_sp(stack_top, core::mem::size_of::<RtSigFrame>());

        assert_eq!(frame_sp & 0xF, 8);
        assert!(frame_sp + core::mem::size_of::<RtSigFrame>() as u64 <= stack_top - REDZONE_SIZE);
    }

    #[test]
    fn setup_rt_frame_does_not_materialize_full_frame_on_kernel_stack() {
        // test-origin: linux:vendor/linux/arch/x86/kernel/signal_64.c:x64_setup_rt_frame
        // Linux's unsafe_put_user sequence writes the user frame in place.  A
        // full RtSigFrame temporary is both a parity divergence and an
        // avoidable live-task kernel-stack footprint during signal delivery.
        let linux = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/signal_64.c"
        ));
        let linux_body = linux
            .split("int x64_setup_rt_frame(")
            .nth(1)
            .and_then(|body| {
                body.split("/* Set up registers for signal handler */")
                    .next()
            })
            .expect("Linux x64_setup_rt_frame body must remain present");
        assert!(linux_body.contains("user_access_begin(frame, sizeof(*frame))"));

        let source = include_str!("signal.rs");
        let start = source
            .find("pub unsafe fn setup_rt_frame(")
            .expect("setup_rt_frame must remain present");
        let end = source[start..]
            .find("\n#[cfg(test)]")
            .map(|offset| start + offset)
            .expect("setup_rt_frame body must end before its tests");
        let body = &source[start..end];

        assert!(!body.contains("let mut frame: RtSigFrame"));
        assert!(!body.contains("core::mem::zeroed()"));
        assert!(!body.contains("(&frame as *const RtSigFrame)"));
        assert!(body.contains("access_ok(user_sp, frame_size as u64)"));
    }
}
