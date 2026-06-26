//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/kprobes/core.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kprobes/core.c
//! x86 kprobes instruction preparation and trap handling.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kprobes/core.c
//!
//! Live text mutation remains behind a trait seam, but the x86 rules Linux
//! relies on here are real: probeable instruction decoding, RIP-relative
//! displacement repair, relative branch synthesis, boostability checks, and
//! the INT3 trap bridge into the generic kprobe registry.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::arch::x86::kernel::alternative::{CALL_INSN_OPCODE, JMP32_INSN_OPCODE};
use crate::arch::x86::kernel::idt::ExceptionFrame;
use crate::arch::x86::kernel::jump_label::text_gen_insn;
use crate::arch::x86::lib::insn::{Insn, MAX_INSN_SIZE};
use crate::include::uapi::errno::{EFAULT, EINVAL, EOPNOTSUPP};

pub const INT3_INSN_OPCODE: u8 = 0xcc;
pub const INT3_INSN_SIZE: usize = 1;
pub const RELATIVEJUMP_OPCODE: u8 = JMP32_INSN_OPCODE;
pub const RELATIVECALL_OPCODE: u8 = CALL_INSN_OPCODE;
pub const RELATIVE_ADDR_SIZE: usize = 4;
pub const RELATIVE_INSN_SIZE: usize = 1 + RELATIVE_ADDR_SIZE;
pub const X86_EFLAGS_IF: u64 = 1 << 9;

pub trait KernelText {
    fn read(&self, ip: u64, len: usize) -> Result<Vec<u8>, i32>;
}

pub trait KprobeTextPoke {
    fn poke(&self, ip: u64, bytes: &[u8]) -> Result<(), i32>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CopiedInsn {
    pub original_ip: u64,
    pub slot_ip: u64,
    pub bytes: [u8; MAX_INSN_SIZE],
    pub len: usize,
    pub rip_relative_fixup: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchKprobe {
    pub addr: u64,
    pub opcode: u8,
    pub copied: CopiedInsn,
    pub boostable: bool,
    pub armed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmulatedRegs {
    pub ip: u64,
    pub sp: u64,
    pub flags: u64,
    pub cx: u64,
    pub stack: Vec<u64>,
}

impl EmulatedRegs {
    pub fn new(ip: u64) -> Self {
        Self {
            ip,
            sp: 0,
            flags: 0,
            cx: 0,
            stack: Vec::new(),
        }
    }
}

pub fn synthesize_reljump(from: u64, to: u64) -> [u8; RELATIVE_INSN_SIZE] {
    let v = text_gen_insn(RELATIVEJUMP_OPCODE, RELATIVE_INSN_SIZE, from, to);
    [v[0], v[1], v[2], v[3], v[4]]
}

pub fn synthesize_relcall(from: u64, to: u64) -> [u8; RELATIVE_INSN_SIZE] {
    let v = text_gen_insn(RELATIVECALL_OPCODE, RELATIVE_INSN_SIZE, from, to);
    [v[0], v[1], v[2], v[3], v[4]]
}

pub fn decode_instruction(bytes: &[u8]) -> Result<Insn, i32> {
    let mut insn = Insn::init(bytes, true);
    let len = insn.get_length() as usize;
    if len == 0 || len > bytes.len() {
        Err(EFAULT)
    } else {
        Ok(insn)
    }
}

pub fn can_probe(bytes: &[u8]) -> bool {
    decode_instruction(bytes)
        .map(|insn| insn.length != 0 && bytes.first().copied() != Some(INT3_INSN_OPCODE))
        .unwrap_or(false)
}

pub fn can_boost(bytes: &[u8]) -> bool {
    if !can_probe(bytes) {
        return false;
    }
    let op = first_opcode(bytes);
    !matches!(
        op,
        0xc2 | 0xc3 | 0xca | 0xcb | 0xcf | 0xe0..=0xe3 | 0xe8 | 0xe9 | 0xeb | 0xfa | 0xfb | 0x9d
    ) && !is_jcc(bytes)
}

pub fn first_opcode(bytes: &[u8]) -> u8 {
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

pub fn is_jcc(bytes: &[u8]) -> bool {
    let op = first_opcode(bytes);
    (0x70..=0x7f).contains(&op)
        || (op == 0x0f && bytes.get(1).is_some_and(|b| (0x80..=0x8f).contains(b)))
}

pub fn copy_instruction(
    original_ip: u64,
    slot_ip: u64,
    original: &[u8],
) -> Result<CopiedInsn, i32> {
    let insn = decode_instruction(original)?;
    let len = insn.length as usize;
    let mut bytes = [0u8; MAX_INSN_SIZE];
    bytes[..len].copy_from_slice(&original[..len]);

    let mut rip_relative_fixup = None;
    if insn.modrm.got != 0 {
        let modrm = insn.modrm.value as u8;
        let mode = (modrm >> 6) & 0x3;
        let rm = modrm & 0x7;
        if mode == 0 && rm == 5 && insn.displacement.nbytes == 4 {
            let disp_off = insn.prefixes.nbytes as usize
                + insn.rex_prefix.nbytes as usize
                + insn.opcode.nbytes as usize
                + insn.modrm.nbytes as usize
                + insn.sib.nbytes as usize;
            let old_disp = i32::from_le_bytes(bytes[disp_off..disp_off + 4].try_into().unwrap());
            let target = original_ip
                .wrapping_add(len as u64)
                .wrapping_add(old_disp as i64 as u64);
            let new_disp = target as i64 - (slot_ip + len as u64) as i64;
            if new_disp < i32::MIN as i64 || new_disp > i32::MAX as i64 {
                return Err(EINVAL);
            }
            bytes[disp_off..disp_off + 4].copy_from_slice(&(new_disp as i32).to_le_bytes());
            rip_relative_fixup = Some(new_disp as i32);
        }
    }

    Ok(CopiedInsn {
        original_ip,
        slot_ip,
        bytes,
        len,
        rip_relative_fixup,
    })
}

pub fn recover_probed_instruction(copied: &CopiedInsn) -> Vec<u8> {
    copied.bytes[..copied.len].to_vec()
}

pub fn arch_prepare_kprobe<T: KernelText>(
    text: &T,
    addr: u64,
    slot_ip: u64,
) -> Result<ArchKprobe, i32> {
    let bytes = text.read(addr, MAX_INSN_SIZE)?;
    let copied = copy_instruction(addr, slot_ip, &bytes)?;
    Ok(ArchKprobe {
        addr,
        opcode: bytes[0],
        copied,
        boostable: can_boost(&bytes),
        armed: false,
    })
}

pub fn arch_arm_kprobe<P: KprobeTextPoke>(poker: &P, kp: &mut ArchKprobe) -> Result<(), i32> {
    poker.poke(kp.addr, &[INT3_INSN_OPCODE])?;
    kp.armed = true;
    Ok(())
}

pub fn arch_disarm_kprobe<P: KprobeTextPoke>(poker: &P, kp: &mut ArchKprobe) -> Result<(), i32> {
    poker.poke(kp.addr, &[kp.opcode])?;
    kp.armed = false;
    Ok(())
}

pub fn arch_remove_kprobe<P: KprobeTextPoke>(poker: &P, kp: &mut ArchKprobe) -> Result<(), i32> {
    if kp.armed {
        arch_disarm_kprobe(poker, kp)?;
    }
    Ok(())
}

pub fn emulate_instruction(bytes: &[u8], regs: &mut EmulatedRegs) -> Result<(), i32> {
    let insn = decode_instruction(bytes)?;
    let op = first_opcode(bytes);
    let len = match op {
        0xe8 | 0xe9 => 5,
        0xeb | 0x70..=0x7f | 0xe0..=0xe3 => 2,
        0xc2 => 3,
        0x0f if bytes.get(1).is_some_and(|b| (0x80..=0x8f).contains(b)) => 6,
        _ => insn.length as u64,
    };
    let next = regs.ip.wrapping_add(len);
    match op {
        0xe8 => {
            let rel = read_i32(bytes, 1)? as i64;
            regs.stack.push(next);
            regs.sp = regs.sp.wrapping_sub(8);
            regs.ip = (next as i64 + rel) as u64;
        }
        0xe9 => regs.ip = (next as i64 + read_i32(bytes, 1)? as i64) as u64,
        0xeb => regs.ip = (next as i64 + read_i8(bytes, 1)? as i64) as u64,
        0xc3 => {
            regs.ip = regs.stack.pop().ok_or(EFAULT)?;
            regs.sp = regs.sp.wrapping_add(8);
        }
        0xc2 => {
            regs.ip = regs.stack.pop().ok_or(EFAULT)?;
            let adj = read_u16(bytes, 1)? as u64;
            regs.sp = regs.sp.wrapping_add(8 + adj);
        }
        0xfa => {
            regs.flags &= !X86_EFLAGS_IF;
            regs.ip = next;
        }
        0xfb => {
            regs.flags |= X86_EFLAGS_IF;
            regs.ip = next;
        }
        0x9d => {
            regs.flags = regs.stack.pop().ok_or(EFAULT)?;
            regs.sp = regs.sp.wrapping_add(8);
            regs.ip = next;
        }
        0xe0..=0xe2 => {
            regs.cx = regs.cx.wrapping_sub(1);
            regs.ip = if regs.cx != 0 {
                (next as i64 + read_i8(bytes, 1)? as i64) as u64
            } else {
                next
            };
        }
        0xe3 => {
            regs.ip = if regs.cx == 0 {
                (next as i64 + read_i8(bytes, 1)? as i64) as u64
            } else {
                next
            };
        }
        op if (0x70..=0x7f).contains(&op) => {
            regs.ip = if jcc_taken(op & 0x0f, regs.flags) {
                (next as i64 + read_i8(bytes, 1)? as i64) as u64
            } else {
                next
            };
        }
        0x0f if bytes.get(1).is_some_and(|b| (0x80..=0x8f).contains(b)) => {
            regs.ip = if jcc_taken(bytes[1] & 0x0f, regs.flags) {
                (next as i64 + read_i32(bytes, 2)? as i64) as u64
            } else {
                next
            };
        }
        _ => regs.ip = next,
    }
    Ok(())
}

pub fn kprobe_int3_handler(frame: &ExceptionFrame) -> bool {
    let probe_ip = frame.rip.wrapping_sub(INT3_INSN_SIZE as u64);
    if !crate::kernel::trace::kprobe::fire_kprobe(probe_ip) {
        return false;
    }
    unsafe {
        let frame_mut = frame as *const ExceptionFrame as *mut ExceptionFrame;
        (*frame_mut).rip = probe_ip;
    }
    true
}

pub const fn arch_populate_kprobe_blacklist() -> &'static [&'static str] {
    &["__switch_to", "do_int3", "kprobe_int3_handler"]
}

pub const fn arch_init_kprobes() -> Result<(), i32> {
    Err(EOPNOTSUPP)
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

fn read_u16(bytes: &[u8], off: usize) -> Result<u16, i32> {
    if off + 2 > bytes.len() {
        return Err(EFAULT);
    }
    Ok(u16::from_le_bytes(bytes[off..off + 2].try_into().unwrap()))
}

fn jcc_taken(cc: u8, flags: u64) -> bool {
    const CF: u64 = 1 << 0;
    const ZF: u64 = 1 << 6;
    const SF: u64 = 1 << 7;
    const OF: u64 = 1 << 11;
    match cc {
        0 => flags & OF != 0,
        1 => flags & OF == 0,
        2 => flags & CF != 0,
        3 => flags & CF == 0,
        4 => flags & ZF != 0,
        5 => flags & ZF == 0,
        6 => flags & (CF | ZF) != 0,
        7 => flags & (CF | ZF) == 0,
        8 => flags & SF != 0,
        9 => flags & SF == 0,
        0xc => (flags & SF != 0) != (flags & OF != 0),
        0xd => (flags & SF != 0) == (flags & OF != 0),
        0xe => (flags & ZF != 0) || ((flags & SF != 0) != (flags & OF != 0)),
        0xf => (flags & ZF == 0) && ((flags & SF != 0) == (flags & OF != 0)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    #[derive(Default)]
    struct Mem {
        bytes: RefCell<BTreeMap<u64, u8>>,
    }

    impl Mem {
        fn seed(&self, ip: u64, bytes: &[u8]) {
            let mut m = self.bytes.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(ip + i as u64, *b);
            }
        }
    }

    impl KernelText for Mem {
        fn read(&self, ip: u64, len: usize) -> Result<Vec<u8>, i32> {
            let m = self.bytes.borrow();
            Ok((0..len)
                .map(|i| *m.get(&(ip + i as u64)).unwrap_or(&0x90))
                .collect())
        }
    }

    impl KprobeTextPoke for Mem {
        fn poke(&self, ip: u64, bytes: &[u8]) -> Result<(), i32> {
            self.seed(ip, bytes);
            Ok(())
        }
    }

    #[test]
    fn reljump_and_relcall_encode_rel32() {
        let j = synthesize_reljump(0x1000, 0x2000);
        assert_eq!(j[0], JMP32_INSN_OPCODE);
        assert_eq!(i32::from_le_bytes(j[1..5].try_into().unwrap()), 0xffb);
        let c = synthesize_relcall(0x1000, 0x0ff0);
        assert_eq!(c[0], CALL_INSN_OPCODE);
        assert_eq!(i32::from_le_bytes(c[1..5].try_into().unwrap()), -0x15);
    }

    #[test]
    fn rip_relative_copy_rewrites_displacement_for_slot() {
        let copied =
            copy_instruction(0x1000, 0x2000, &[0x48, 0x8b, 0x05, 0x34, 0x12, 0, 0]).expect("copy");
        assert_eq!(copied.len, 7);
        let new_disp = i32::from_le_bytes(copied.bytes[3..7].try_into().unwrap());
        assert_eq!(new_disp, 0x234);
    }

    #[test]
    fn boost_rejects_control_flow_and_if_modifiers() {
        assert!(can_boost(&[0x90]));
        assert!(!can_boost(&[0xe8, 0, 0, 0, 0]));
        assert!(!can_boost(&[0xeb, 0]));
        assert!(!can_boost(&[0xc3]));
        assert!(!can_boost(&[0xfa]));
    }

    #[test]
    fn emulate_call_ret_jcc_loop_and_if() {
        let mut regs = EmulatedRegs::new(0x1000);
        regs.sp = 0x8000;
        emulate_instruction(&[0xe8, 0x05, 0, 0, 0], &mut regs).unwrap();
        assert_eq!(regs.ip, 0x100a);
        assert_eq!(regs.stack.last().copied(), Some(0x1005));
        emulate_instruction(&[0xc3], &mut regs).unwrap();
        assert_eq!(regs.ip, 0x1005);

        regs.ip = 0x2000;
        regs.flags = 1 << 6;
        emulate_instruction(&[0x74, 0x7e], &mut regs).unwrap();
        assert_eq!(regs.ip, 0x2080);

        regs.ip = 0x3000;
        regs.cx = 2;
        emulate_instruction(&[0xe2, 0xfc], &mut regs).unwrap();
        assert_eq!(regs.cx, 1);
        assert_eq!(regs.ip, 0x2ffe);

        emulate_instruction(&[0xfa], &mut regs).unwrap();
        assert_eq!(regs.flags & X86_EFLAGS_IF, 0);
        emulate_instruction(&[0xfb], &mut regs).unwrap();
        assert_ne!(regs.flags & X86_EFLAGS_IF, 0);
    }

    #[test]
    fn arm_and_disarm_patch_first_byte() {
        let mem = Mem::default();
        mem.seed(0x1000, &[0x90; MAX_INSN_SIZE]);
        let mut kp = arch_prepare_kprobe(&mem, 0x1000, 0x2000).unwrap();
        arch_arm_kprobe(&mem, &mut kp).unwrap();
        assert!(kp.armed);
        assert_eq!(mem.read(0x1000, 1).unwrap(), &[INT3_INSN_OPCODE]);
        arch_disarm_kprobe(&mem, &mut kp).unwrap();
        assert!(!kp.armed);
        assert_eq!(mem.read(0x1000, 1).unwrap(), &[0x90]);
    }
}
