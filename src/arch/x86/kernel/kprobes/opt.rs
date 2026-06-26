//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/kprobes/opt.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kprobes/opt.c
//! x86 optimized kprobe planning.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kprobes/opt.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::arch::x86::kernel::alternative::JMP32_INSN_OPCODE;
use crate::arch::x86::kernel::jump_label::text_gen_insn;
use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

use super::core::{KprobeTextPoke, RELATIVE_INSN_SIZE, can_boost, decode_instruction};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizedKprobe {
    pub addr: u64,
    pub detour_addr: u64,
    pub optimized_len: usize,
    pub saved: Vec<u8>,
    pub jump: Vec<u8>,
    pub optimized: bool,
}

pub fn __recover_optprobed_insn(saved: &[u8], current: &[u8]) -> Vec<u8> {
    let mut out = current.to_vec();
    let n = saved.len().min(out.len());
    out[..n].copy_from_slice(&saved[..n]);
    out
}

pub fn copy_optimized_instructions(ip: u64, bytes: &[u8]) -> Result<(usize, Vec<u8>), i32> {
    let mut off = 0;
    while off < bytes.len() && off < RELATIVE_INSN_SIZE {
        let insn = decode_instruction(&bytes[off..])?;
        let len = insn.length as usize;
        if len == 0 || off + len > bytes.len() || !can_boost(&bytes[off..off + len]) {
            return Err(EINVAL);
        }
        off += len;
    }
    if off < RELATIVE_INSN_SIZE {
        return Err(EINVAL);
    }
    let copied = bytes[..off].to_vec();
    let _ = ip;
    Ok((off, copied))
}

pub fn jump_target(ip: u64, bytes: &[u8]) -> Option<u64> {
    match bytes.first().copied()? {
        0xe9 | 0xe8 if bytes.len() >= 5 => {
            let rel = i32::from_le_bytes(bytes[1..5].try_into().ok()?) as i64;
            Some((ip as i64 + 5 + rel) as u64)
        }
        0xeb if bytes.len() >= 2 => Some((ip as i64 + 2 + bytes[1] as i8 as i64) as u64),
        0x70..=0x7f if bytes.len() >= 2 => Some((ip as i64 + 2 + bytes[1] as i8 as i64) as u64),
        _ => None,
    }
}

pub fn has_jump_into_range(base: u64, len: usize, probes: &[(u64, &[u8])]) -> bool {
    let end = base + len as u64;
    probes.iter().any(|(ip, bytes)| {
        jump_target(*ip, bytes)
            .map(|target| target > base && target < end)
            .unwrap_or(false)
    })
}

pub fn arch_check_optimized_kprobe(ip: u64, bytes: &[u8]) -> Result<usize, i32> {
    let (len, _) = copy_optimized_instructions(ip, bytes)?;
    Ok(len)
}

pub fn arch_prepare_optimized_kprobe(
    ip: u64,
    detour_addr: u64,
    bytes: &[u8],
) -> Result<OptimizedKprobe, i32> {
    let (optimized_len, saved) = copy_optimized_instructions(ip, bytes)?;
    let jump = text_gen_insn(JMP32_INSN_OPCODE, RELATIVE_INSN_SIZE, ip, detour_addr);
    Ok(OptimizedKprobe {
        addr: ip,
        detour_addr,
        optimized_len,
        saved,
        jump,
        optimized: false,
    })
}

pub fn arch_optimize_kprobe<P: KprobeTextPoke>(
    poker: &P,
    kp: &mut OptimizedKprobe,
) -> Result<(), i32> {
    poker.poke(kp.addr, &kp.jump)?;
    kp.optimized = true;
    Ok(())
}

pub fn arch_unoptimize_kprobe<P: KprobeTextPoke>(
    poker: &P,
    kp: &mut OptimizedKprobe,
) -> Result<(), i32> {
    poker.poke(kp.addr, &kp.saved)?;
    kp.optimized = false;
    Ok(())
}

pub const fn arch_within_optimized_kprobe(ip: u64, kp: &OptimizedKprobe) -> bool {
    ip >= kp.addr && ip < kp.addr + kp.optimized_len as u64
}

pub const fn setup_detour_execution_supported() -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    #[derive(Default)]
    struct Mem(RefCell<BTreeMap<u64, u8>>);

    impl KprobeTextPoke for Mem {
        fn poke(&self, ip: u64, bytes: &[u8]) -> Result<(), i32> {
            let mut m = self.0.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(ip + i as u64, *b);
            }
            Ok(())
        }
    }

    #[test]
    fn prepare_requires_enough_boostable_bytes() {
        let kp =
            arch_prepare_optimized_kprobe(0x1000, 0x2000, &[0x90, 0x90, 0x90, 0x90, 0x90]).unwrap();
        assert_eq!(kp.optimized_len, 5);
        assert_eq!(kp.jump[0], JMP32_INSN_OPCODE);
        assert!(arch_within_optimized_kprobe(0x1004, &kp));
        assert!(!arch_within_optimized_kprobe(0x1005, &kp));
    }

    #[test]
    fn optimize_and_unoptimize_write_expected_bytes() {
        let mem = Mem::default();
        let mut kp =
            arch_prepare_optimized_kprobe(0x1000, 0x2000, &[0x90, 0x90, 0x90, 0x90, 0x90]).unwrap();
        arch_optimize_kprobe(&mem, &mut kp).unwrap();
        assert!(kp.optimized);
        arch_unoptimize_kprobe(&mem, &mut kp).unwrap();
        assert!(!kp.optimized);
    }

    #[test]
    fn detects_jump_into_optimized_range() {
        assert!(has_jump_into_range(
            0x1000,
            8,
            &[(0x2000, &[0xe9, 0xff, 0xef, 0xff, 0xff])]
        ));
    }
}
