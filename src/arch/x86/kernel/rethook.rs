//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/rethook.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/rethook.c
//! x86 return hook trampoline helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/rethook.c

#![allow(dead_code)]

use crate::arch::x86::kernel::gdt::sel;
use crate::arch::x86::kernel::ptrace::PtRegs;
use crate::include::uapi::errno::EFAULT;

pub const RETHOOK_TRAMPOLINE_IP: u64 = 0xffff_ffff_8000_0f00;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RethookNode {
    pub ret_addr: u64,
    pub frame: u64,
}

pub fn arch_rethook_prepare(
    rh: &mut RethookNode,
    regs: &PtRegs,
    stack: &mut [u64],
) -> Result<(), i32> {
    let ret = stack.first_mut().ok_or(EFAULT)?;
    rh.ret_addr = *ret;
    rh.frame = regs.rsp;
    *ret = RETHOOK_TRAMPOLINE_IP;
    Ok(())
}

pub fn arch_rethook_fixup_return(frame_pointer: &mut u64, correct_ret_addr: u64) {
    *frame_pointer = correct_ret_addr;
}

pub fn arch_rethook_trampoline_callback_fixup(regs: &mut PtRegs) {
    regs.cs = sel::KERNEL_CS as u64;
    regs.rip = RETHOOK_TRAMPOLINE_IP;
    regs.orig_rax = u64::MAX;
    regs.rsp = regs
        .rsp
        .saturating_add(2 * core::mem::size_of::<u64>() as u64);
    regs.ss = regs.eflags;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regs() -> PtRegs {
        PtRegs {
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
            rip: 0x1000,
            cs: 0,
            eflags: 0x202,
            rsp: 0x8000,
            ss: 0,
        }
    }

    #[test]
    fn prepare_replaces_return_address_with_trampoline() {
        let regs = regs();
        let mut node = RethookNode::default();
        let mut stack = [0x1234];
        arch_rethook_prepare(&mut node, &regs, &mut stack).unwrap();
        assert_eq!(node.ret_addr, 0x1234);
        assert_eq!(node.frame, 0x8000);
        assert_eq!(stack[0], RETHOOK_TRAMPOLINE_IP);
    }

    #[test]
    fn callback_fixup_marks_kernel_frame() {
        let mut regs = regs();
        arch_rethook_trampoline_callback_fixup(&mut regs);
        assert_eq!(regs.cs, sel::KERNEL_CS as u64);
        assert_eq!(regs.orig_rax, u64::MAX);
        assert_eq!(regs.ss, 0x202);
        assert_eq!(regs.rsp, 0x8010);
    }
}
