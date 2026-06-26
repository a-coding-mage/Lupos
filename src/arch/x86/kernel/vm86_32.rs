//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/vm86_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/vm86_32.c
//! x86 VM86 compatibility state helpers.
//!
//! Provides VM86 state/eflags helpers; v8086 mode itself is not entered (Lupos
//! has no real-mode task path). Remaining work vs Linux for `complete`: the
//! `sys_vm86`/`sys_vm86old` entry points and v8086 fault/IRQ handling in
//! vm86_32.c.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/vm86_32.c

#![allow(dead_code)]

use crate::include::uapi::errno::{EINVAL, ENOSYS};

pub const X86_EFLAGS_CF: u32 = 1 << 0;
pub const X86_EFLAGS_IF: u32 = 1 << 9;
pub const X86_EFLAGS_TF: u32 = 1 << 8;
pub const X86_EFLAGS_AC: u32 = 1 << 18;
pub const VM86_TYPE: u32 = 0;
pub const VM86_ENTER: u32 = 1;
pub const VM86_ENTER_NO_BYPASS: u32 = 2;
pub const VM86_REQUEST_IRQ: u32 = 3;
pub const VM86_FREE_IRQ: u32 = 4;
pub const VM86_GET_IRQ_BITS: u32 = 5;
pub const VM86_GET_AND_RESET_IRQ: u32 = 6;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KernelVm86Regs {
    pub bx: u32,
    pub cx: u32,
    pub dx: u32,
    pub si: u32,
    pub di: u32,
    pub bp: u32,
    pub ax: u32,
    pub ip: u32,
    pub cs: u32,
    pub flags: u32,
    pub sp: u32,
    pub ss: u32,
    pub es: u32,
    pub ds: u32,
    pub fs: u32,
    pub gs: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Vm86State {
    pub regs: KernelVm86Regs,
    pub retval: i32,
    pub saved: bool,
}

pub fn save_v86_state(regs: KernelVm86Regs, retval: i32) -> Vm86State {
    Vm86State {
        regs,
        retval,
        saved: true,
    }
}

pub fn set_if(regs: &mut KernelVm86Regs) {
    regs.flags |= X86_EFLAGS_IF;
}

pub fn clear_if(regs: &mut KernelVm86Regs) {
    regs.flags &= !X86_EFLAGS_IF;
}

pub fn clear_tf(regs: &mut KernelVm86Regs) {
    regs.flags &= !X86_EFLAGS_TF;
}

pub fn clear_ac(regs: &mut KernelVm86Regs) {
    regs.flags &= !X86_EFLAGS_AC;
}

pub const fn get_vflags(regs: &KernelVm86Regs) -> u32 {
    regs.flags
}

pub const fn is_revectored(nr: u8, bitmap: &[u8; 32]) -> bool {
    (bitmap[(nr / 8) as usize] & (1 << (nr & 7))) != 0
}

pub const fn handle_vm86_trap(_trapno: i32) -> i32 {
    0
}

pub const fn sys_vm86(cmd: u64, arg: u64) -> i64 {
    if matches!(cmd as u32, VM86_ENTER | VM86_ENTER_NO_BYPASS) && arg == 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub const fn sys_vm86old(arg: u64) -> i64 {
    if arg == 0 {
        -(EINVAL as i64)
    } else {
        -(ENOSYS as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm86_flags_helpers_match_eflags_bits() {
        let mut regs = KernelVm86Regs::default();
        set_if(&mut regs);
        assert_ne!(get_vflags(&regs) & X86_EFLAGS_IF, 0);
        regs.flags |= X86_EFLAGS_TF | X86_EFLAGS_AC;
        clear_tf(&mut regs);
        clear_ac(&mut regs);
        assert_eq!(regs.flags & (X86_EFLAGS_TF | X86_EFLAGS_AC), 0);
    }

    #[test]
    fn revectored_bitmap_uses_irq_bit_number() {
        let mut bitmap = [0u8; 32];
        bitmap[2] = 1 << 3;
        assert!(is_revectored(19, &bitmap));
        assert_eq!(sys_vm86(VM86_ENTER as u64, 0), -(EINVAL as i64));
    }
}
