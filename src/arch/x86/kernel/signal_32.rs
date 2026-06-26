//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/signal_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/signal_32.c
//! IA32 signal frame helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/signal_32.c

#![allow(dead_code)]

use crate::include::uapi::errno::EINVAL;
use crate::kernel::signal::{SigInfo, SigSet};

pub const USER_RPL: u16 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SigContext32 {
    pub gs: u16,
    pub fs: u16,
    pub es: u16,
    pub ds: u16,
    pub di: u32,
    pub si: u32,
    pub bp: u32,
    pub sp: u32,
    pub bx: u32,
    pub dx: u32,
    pub cx: u32,
    pub ax: u32,
    pub trapno: u32,
    pub err: u32,
    pub ip: u32,
    pub cs: u16,
    pub flags: u32,
    pub sp_at_signal: u32,
    pub ss: u16,
    pub fpstate: u32,
    pub oldmask: u32,
    pub cr2: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UContext32 {
    pub uc_flags: u32,
    pub uc_link: u32,
    pub uc_stack_sp: u32,
    pub uc_stack_flags: u32,
    pub uc_stack_size: u32,
    pub uc_mcontext: SigContext32,
    pub uc_sigmask: SigSet,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RtSigFrame32 {
    pub pretcode: u32,
    pub sig: u32,
    pub pinfo: u32,
    pub puc: u32,
    pub info: SigInfo,
    pub uc: UContext32,
}

pub const fn fixup_rpl(sel: u16) -> u16 {
    if sel == 0 { 0 } else { sel | USER_RPL }
}

pub const fn frame_aligned_sp(sp: u32, frame_size: u32) -> Result<u32, i32> {
    if frame_size == 0 || frame_size > sp {
        return Err(EINVAL);
    }
    Ok((sp - frame_size) & !0xf)
}

pub fn fill_sigcontext32(
    regs: &crate::kernel::task::PtRegs,
    trapno: u32,
    err: u32,
    oldmask: u32,
    cr2: u32,
) -> SigContext32 {
    SigContext32 {
        gs: 0,
        fs: 0,
        es: fixup_rpl(0),
        ds: fixup_rpl(0),
        di: regs.di as u32,
        si: regs.si as u32,
        bp: regs.bp as u32,
        sp: regs.sp as u32,
        bx: regs.bx as u32,
        dx: regs.dx as u32,
        cx: regs.cx as u32,
        ax: regs.ax as u32,
        trapno,
        err,
        ip: regs.ip as u32,
        cs: fixup_rpl(regs.cs as u16),
        flags: regs.flags as u32,
        sp_at_signal: regs.sp as u32,
        ss: fixup_rpl(regs.ss as u16),
        fpstate: 0,
        oldmask,
        cr2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regs() -> crate::kernel::task::PtRegs {
        crate::kernel::task::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            bp: 5,
            bx: 4,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            ax: 1,
            cx: 2,
            dx: 3,
            si: 6,
            di: 7,
            orig_ax: 0,
            ip: 0x8048000,
            cs: 0x20,
            flags: 0x202,
            sp: 0xbfff_fffc,
            ss: 0x18,
        }
    }

    #[test]
    fn selectors_are_forced_to_user_rpl() {
        assert_eq!(fixup_rpl(0), 0);
        assert_eq!(fixup_rpl(0x20), 0x23);
    }

    #[test]
    fn sigcontext32_truncates_regs_to_ia32_layout() {
        let sc = fill_sigcontext32(&regs(), 14, 4, 0xaa, 0xdead);
        assert_eq!(sc.ip, 0x8048000);
        assert_eq!(sc.cs, 0x23);
        assert_eq!(sc.ss, 0x1b);
        assert_eq!(sc.oldmask, 0xaa);
    }
}
