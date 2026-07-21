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

/// Set up the signal frame on the user stack.
///
/// Constructs a `RtSigFrame` with the current register state, signal info, and
/// alternate stack info, then modifies `regs` to point to the signal handler.
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
    let Some((user_sp, fpstate_sp)) = x64_rt_sigframe_layout((*regs).sp, frame_size, fpstate_size)
    else {
        return Err(-14); // EFAULT
    };

    // Verify we're not going off the edge of the stack.
    if user_sp == 0
        || !unsafe { crate::arch::x86::kernel::fpu_signal::copy_fpstate_to_sigframe(fpstate_sp) }
    {
        return Err(-14); // EFAULT
    }

    // Build the signal frame in kernel memory, then copy to user stack.
    let mut frame: RtSigFrame = unsafe { core::mem::zeroed() };

    // 1. Set sa_restorer (the return code address).
    frame.pretcode = (*action).sa_restorer as u64;

    // 2. Fill in ucontext_t: flags, link, stack, mcontext, sigmask.
    frame.uc.uc_flags = UC_SIGCONTEXT_SS | UC_STRICT_RESTORE_SS;
    if crate::arch::x86::kernel::fpu::signal_uses_xsave() {
        frame.uc.uc_flags |= UC_FP_XSTATE;
    }
    frame.uc.uc_link = 0; // No linked context
    // Placeholder: proper stack setup in later work
    frame.uc.uc_stack = SigAltStack {
        ss_sp: 0,
        ss_flags: 0,
        ss_size: 0,
    };

    // 3. Copy machine context (SigContext) from current PtRegs.
    {
        let regs_ref = unsafe { &*regs };
        let sc = &mut frame.uc.uc_mcontext;
        sc.r8 = regs_ref.r8;
        sc.r9 = regs_ref.r9;
        sc.r10 = regs_ref.r10;
        sc.r11 = regs_ref.r11;
        sc.r12 = regs_ref.r12;
        sc.r13 = regs_ref.r13;
        sc.r14 = regs_ref.r14;
        sc.r15 = regs_ref.r15;
        sc.rdi = regs_ref.di;
        sc.rsi = regs_ref.si;
        sc.rbp = regs_ref.bp;
        sc.rbx = regs_ref.bx;
        sc.rdx = regs_ref.dx;
        sc.rax = regs_ref.ax;
        sc.rcx = regs_ref.cx;
        sc.rsp = regs_ref.sp;
        sc.rip = regs_ref.ip;
        sc.eflags = regs_ref.flags;
        sc.cs = regs_ref.cs as u16;
        sc.ss = regs_ref.ss as u16;
        sc.gs = 0; // Placeholder
        sc.fs = 0; // Placeholder
        sc.err = regs_ref.orig_ax as u64; // Store orig_ax as err temporarily
        sc.trapno = 0;
        sc.oldmask = 0;
        sc.cr2 = 0;
        sc.fpstate = fpstate_sp;
        sc.reserved1 = [0; 8];
    }

    // 4. Copy signal info (siginfo_t).
    frame.info = unsafe { *info };
    frame.uc.uc_sigmask = mask;
    frame.uc.uc_mcontext.oldmask = mask.bits;

    // 5. Write the frame to user stack.
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(
            user_sp as *mut u8,
            (&frame as *const RtSigFrame).cast::<u8>(),
            frame_size,
        )
    };
    if not_copied != 0 {
        return Err(-14); // EFAULT
    }

    // 6. Modify PtRegs to transfer control to the signal handler.
    let regs_mut = unsafe { &mut *regs };
    regs_mut.ip = (*action).sa_handler as u64; // RIP = signal handler entry
    regs_mut.sp = user_sp; // RSP = frame base
    regs_mut.di = signum as u64; // RDI = signal number (arg 0)
    regs_mut.ax = 0; // Linux clears AX in case the handler lacks prototypes.
    regs_mut.si = user_sp + core::mem::offset_of!(RtSigFrame, info) as u64; // RSI = &siginfo_t
    regs_mut.dx = user_sp + core::mem::offset_of!(RtSigFrame, uc) as u64; // RDX = &ucontext_t
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
            sa_flags: 0,
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
}
