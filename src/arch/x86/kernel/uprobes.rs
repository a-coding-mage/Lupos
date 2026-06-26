//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/uprobes.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/uprobes.c
//! x86 uprobes instruction analysis and XOL emulation helpers.
//!
//! Implements a subset of uprobes.c: instruction analysis and execute-out-of-
//! line (XOL) emulation helpers. Remaining work vs Linux for `complete`: the
//! portions of uprobes.c not yet ported — full breakpoint insert/remove,
//! single-step fixups across all instruction classes, and the uretprobe
//! trampoline.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/uprobes.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::arch::x86::kernel::ptrace::PtRegs;
use crate::arch::x86::lib::insn::{Insn, MAX_INSN_SIZE};
use crate::include::uapi::errno::{EFAULT, EINVAL, ENOSYS};

pub const UPROBE_SWBP_INSN: u8 = 0xcc;
pub const UPROBE_XOL_SLOT_BYTES: usize = MAX_INSN_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UprobeOp {
    Default,
    Branch,
    Push,
    RipRelative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchUprobe {
    pub bytes: [u8; UPROBE_XOL_SLOT_BYTES],
    pub len: usize,
    pub op: UprobeOp,
    pub fixups: u32,
}

pub fn is_prefix_bad(insn: &Insn) -> bool {
    matches!(insn.prefixes.value as u8, 0xf0 | 0x66)
}

pub fn decode_user_insn(bytes: &[u8], x86_64: bool) -> Result<Insn, i32> {
    let mut insn = Insn::init(bytes, x86_64);
    let len = insn.get_length() as usize;
    if len == 0 || len > bytes.len() {
        Err(EFAULT)
    } else {
        Ok(insn)
    }
}

pub fn arch_uprobe_analyze_insn(bytes: &[u8], x86_64: bool) -> Result<ArchUprobe, i32> {
    if bytes.first().copied() == Some(UPROBE_SWBP_INSN) {
        return Err(EINVAL);
    }
    let insn = decode_user_insn(bytes, x86_64)?;
    if is_prefix_bad(&insn) {
        return Err(EINVAL);
    }
    let len = insn.length as usize;
    let mut out = [0u8; UPROBE_XOL_SLOT_BYTES];
    out[..len].copy_from_slice(&bytes[..len]);
    let op = match first_opcode(bytes) {
        0xe8 | 0xe9 | 0xeb | 0x70..=0x7f => UprobeOp::Branch,
        0x68 | 0x6a => UprobeOp::Push,
        _ if is_rip_relative(&insn) => UprobeOp::RipRelative,
        _ => UprobeOp::Default,
    };
    Ok(ArchUprobe {
        bytes: out,
        len,
        op,
        fixups: 0,
    })
}

pub fn arch_uprobe_pre_xol(
    auprobe: &ArchUprobe,
    regs: &mut PtRegs,
    xol_vaddr: u64,
) -> Result<(), i32> {
    if auprobe.len == 0 {
        return Err(EINVAL);
    }
    regs.rip = xol_vaddr;
    Ok(())
}

pub fn arch_uprobe_post_xol(
    auprobe: &ArchUprobe,
    regs: &mut PtRegs,
    probed_vaddr: u64,
) -> Result<(), i32> {
    match auprobe.op {
        UprobeOp::Branch => branch_post_xol_op(auprobe, regs, probed_vaddr),
        _ => {
            regs.rip = probed_vaddr + auprobe.len as u64;
            Ok(())
        }
    }
}

pub fn branch_post_xol_op(
    auprobe: &ArchUprobe,
    regs: &mut PtRegs,
    probed_vaddr: u64,
) -> Result<(), i32> {
    let bytes = &auprobe.bytes[..auprobe.len];
    let op = first_opcode(bytes);
    let next = probed_vaddr + auprobe.len as u64;
    regs.rip = match op {
        0xe8 | 0xe9 => (next as i64 + read_i32(bytes, 1)? as i64) as u64,
        0xeb | 0x70..=0x7f => (next as i64 + read_i8(bytes, 1)? as i64) as u64,
        _ => next,
    };
    Ok(())
}

pub fn arch_uprobe_abort_xol(auprobe: &ArchUprobe, regs: &mut PtRegs, probed_vaddr: u64) {
    let _ = auprobe;
    regs.rip = probed_vaddr;
}

pub fn arch_uprobe_skip_sstep(auprobe: &ArchUprobe, regs: &mut PtRegs, probed_vaddr: u64) -> bool {
    if matches!(auprobe.op, UprobeOp::Branch | UprobeOp::Push) {
        arch_uprobe_post_xol(auprobe, regs, probed_vaddr).is_ok()
    } else {
        false
    }
}

pub const fn arch_uprobe_get_xol_area(base: u64) -> u64 {
    base
}

pub fn arch_uretprobe_trampoline() -> &'static [u8] {
    &[UPROBE_SWBP_INSN, 0x48, 0x89, 0xc0]
}

pub fn sys_uretprobe() -> i64 {
    -(ENOSYS as i64)
}

pub fn sys_uprobe() -> i64 {
    -(ENOSYS as i64)
}

fn first_opcode(bytes: &[u8]) -> u8 {
    let mut i = 0;
    while i < bytes.len()
        && matches!(
            bytes[i],
            0x26 | 0x2e | 0x36 | 0x3e | 0x64 | 0x65 | 0x66 | 0x67 | 0xf0 | 0xf2 | 0xf3
        )
    {
        i += 1;
    }
    if i < bytes.len() && (0x40..=0x4f).contains(&bytes[i]) {
        i += 1;
    }
    bytes.get(i).copied().unwrap_or(0)
}

fn is_rip_relative(insn: &Insn) -> bool {
    if insn.modrm.got == 0 || insn.displacement.nbytes != 4 {
        return false;
    }
    let modrm = insn.modrm.value as u8;
    ((modrm >> 6) & 0x3) == 0 && (modrm & 0x7) == 5
}

fn read_i8(bytes: &[u8], off: usize) -> Result<i8, i32> {
    bytes.get(off).copied().map(|b| b as i8).ok_or(EFAULT)
}

fn read_i32(bytes: &[u8], off: usize) -> Result<i32, i32> {
    if off + 4 > bytes.len() {
        return Err(EFAULT);
    }
    Ok(i32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()))
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
    fn analyze_rejects_breakpoint_and_classifies_branch() {
        assert_eq!(
            arch_uprobe_analyze_insn(&[0xcc], true).map(|_| ()),
            Err(EINVAL)
        );
        let au = arch_uprobe_analyze_insn(&[0xe9, 5, 0, 0, 0], true).unwrap();
        assert_eq!(au.op, UprobeOp::Branch);
    }

    #[test]
    fn branch_post_xol_recomputes_original_target() {
        let au = arch_uprobe_analyze_insn(&[0xe9, 5, 0, 0, 0], true).unwrap();
        let mut regs = regs();
        arch_uprobe_pre_xol(&au, &mut regs, 0x8000).unwrap();
        arch_uprobe_post_xol(&au, &mut regs, 0x1000).unwrap();
        assert_eq!(regs.rip, 0x100a);
    }

    #[test]
    fn uprobe_syscalls_preserve_fail_closed_errno() {
        assert_eq!(sys_uretprobe(), -(ENOSYS as i64));
        assert_eq!(sys_uprobe(), -(ENOSYS as i64));
        assert_eq!(arch_uretprobe_trampoline()[0], UPROBE_SWBP_INSN);
    }
}
