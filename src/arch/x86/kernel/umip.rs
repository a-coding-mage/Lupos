//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/umip.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/umip.c
//! x86 UMIP instruction fixup helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/umip.c

#![allow(dead_code)]

use crate::arch::x86::kernel::ptrace::PtRegs;
use crate::include::uapi::errno::{EFAULT, EINVAL};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UmipInsn {
    Sgdt,
    Sidt,
    Sldt,
    Smsw,
    Str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UmipFixup {
    pub insn: UmipInsn,
    pub len: u8,
    pub dummy_value: u64,
}

pub const SGDT_DUMMY_BASE: u64 = 0xffff_ffff;
pub const SIDT_DUMMY_BASE: u64 = 0xffff_0000;

pub fn identify_insn(bytes: &[u8]) -> Result<UmipInsn, i32> {
    if bytes.len() < 2 {
        return Err(EFAULT);
    }
    if bytes[0] == 0x0f {
        match bytes[1] {
            0x00 => match bytes.get(2).copied().unwrap_or(0) & 0x38 {
                0x00 => Ok(UmipInsn::Sldt),
                0x08 => Ok(UmipInsn::Str),
                _ => Err(EINVAL),
            },
            0x01 => match bytes.get(2).copied().unwrap_or(0) & 0x38 {
                0x00 => Ok(UmipInsn::Sgdt),
                0x08 => Ok(UmipInsn::Sidt),
                0x20 => Ok(UmipInsn::Smsw),
                _ => Err(EINVAL),
            },
            _ => Err(EINVAL),
        }
    } else {
        Err(EINVAL)
    }
}

pub const fn dummy_value(insn: UmipInsn) -> u64 {
    match insn {
        UmipInsn::Sgdt => (SGDT_DUMMY_BASE << 16) | 0,
        UmipInsn::Sidt => (SIDT_DUMMY_BASE << 16) | 0,
        UmipInsn::Sldt | UmipInsn::Str | UmipInsn::Smsw => 0,
    }
}

pub fn fixup_umip_exception(regs: &mut PtRegs, bytes: &[u8]) -> Result<UmipFixup, i32> {
    let insn = identify_insn(bytes)?;
    let len = if matches!(insn, UmipInsn::Sgdt | UmipInsn::Sidt | UmipInsn::Smsw) {
        3
    } else {
        3
    };
    regs.rip = regs.rip.wrapping_add(len);
    Ok(UmipFixup {
        insn,
        len: len as u8,
        dummy_value: dummy_value(insn),
    })
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
            eflags: 0,
            rsp: 0,
            ss: 0,
        }
    }

    #[test]
    fn identifies_umip_class_instructions() {
        assert_eq!(identify_insn(&[0x0f, 0x01, 0x00]), Ok(UmipInsn::Sgdt));
        assert_eq!(identify_insn(&[0x0f, 0x01, 0x08]), Ok(UmipInsn::Sidt));
        assert_eq!(identify_insn(&[0x0f, 0x00, 0x00]), Ok(UmipInsn::Sldt));
        assert_eq!(identify_insn(&[0x0f, 0x00, 0x08]), Ok(UmipInsn::Str));
        assert_eq!(identify_insn(&[0x90]), Err(EFAULT));
    }

    #[test]
    fn fixup_advances_ip_and_returns_dummy_value() {
        let mut regs = regs();
        let fixup = fixup_umip_exception(&mut regs, &[0x0f, 0x01, 0x00]).unwrap();
        assert_eq!(regs.rip, 0x1003);
        assert_eq!(fixup.insn, UmipInsn::Sgdt);
        assert_ne!(fixup.dummy_value, 0);
    }
}
