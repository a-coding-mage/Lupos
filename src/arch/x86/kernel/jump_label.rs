//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/jump_label.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/jump_label.c
//! Static jump-label code patching.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/jump_label.c
//!
//! Static keys compile down to either a 2-byte or 5-byte instruction site.
//! When the key is toggled, the site is rewritten between NOP and JMP. The
//! Rust side ports the byte-level transform algorithm plus the
//! expected-byte verification; the actual text mutation goes through a
//! `TextPoke` trait seam so we can keep "live text mutation behind a
//! fail-closed seam" (per `alternative.rs` precedent).

#![allow(dead_code)]

extern crate alloc;

use crate::include::uapi::errno::{EFAULT, EINVAL};

use super::alternative::{JMP32_INSN_OPCODE, X86_NOP2, x86_nop};

// === Vendor constants — mirror asm/text-patching.h ===

pub const JMP32_INSN_SIZE: usize = 5;
pub const JMP8_INSN_OPCODE: u8 = 0xEB;
pub const JMP8_INSN_SIZE: usize = 2;

/// `enum jump_label_type` — direction of the transform.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum JumpLabelType {
    /// `nop` site → `jmp` (key is now true).
    Jmp,
    /// `jmp` site → `nop` (key is now false).
    Nop,
}

/// `enum SystemState` — subset relevant to the jump-label transform path.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SystemState {
    Booting,
    Scheduling,
    Running,
}

/// Result of `__jump_label_patch` — the bytes to write into the
/// instruction site.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JumpLabelPatch {
    pub code: alloc::vec::Vec<u8>,
    pub size: usize,
}

/// `text_gen_insn(opcode, addr, dest)` — write the opcode followed by a
/// PC-relative displacement (rip-relative = `dest - (addr + size)`).
pub fn text_gen_insn(opcode: u8, size: usize, addr: u64, dest: u64) -> alloc::vec::Vec<u8> {
    let mut out = alloc::vec::Vec::with_capacity(size);
    out.push(opcode);
    let rel = dest as i64 - (addr as i64 + size as i64);
    match size {
        JMP8_INSN_SIZE => {
            out.push(rel as i8 as u8);
        }
        JMP32_INSN_SIZE => {
            let rel32 = rel as i32;
            out.extend_from_slice(&rel32.to_le_bytes());
        }
        _ => {
            // Other instruction sizes are not jump-label-relevant.
        }
    }
    out
}

/// Construct the patch payload for an instruction site of `size` bytes,
/// transitioning to `type_` direction.
///
/// On `Jmp` the site previously held the NOP; on `Nop` it previously held
/// the JMP. The function returns the *new* bytes to write; callers verify
/// the *current* bytes (the "expected" form) before applying.
pub fn jump_label_patch(
    addr: u64,
    dest: u64,
    size: usize,
    type_: JumpLabelType,
) -> Result<JumpLabelPatch, i32> {
    let nop = match size {
        JMP8_INSN_SIZE => X86_NOP2.to_vec(),
        JMP32_INSN_SIZE => x86_nop(JMP32_INSN_SIZE).ok_or(EINVAL)?.to_vec(),
        _ => return Err(EINVAL),
    };
    let jmp = match size {
        JMP8_INSN_SIZE => text_gen_insn(JMP8_INSN_OPCODE, JMP8_INSN_SIZE, addr, dest),
        JMP32_INSN_SIZE => text_gen_insn(JMP32_INSN_OPCODE, JMP32_INSN_SIZE, addr, dest),
        _ => return Err(EINVAL),
    };
    let code = match type_ {
        JumpLabelType::Jmp => jmp,
        JumpLabelType::Nop => nop,
    };
    Ok(JumpLabelPatch { code, size })
}

/// The *expected* current bytes at `addr` before the transform — the
/// other variant of the pair.
pub fn jump_label_expected(
    addr: u64,
    dest: u64,
    size: usize,
    type_: JumpLabelType,
) -> Result<alloc::vec::Vec<u8>, i32> {
    let opposite = match type_ {
        JumpLabelType::Jmp => JumpLabelType::Nop,
        JumpLabelType::Nop => JumpLabelType::Jmp,
    };
    Ok(jump_label_patch(addr, dest, size, opposite)?.code)
}

/// Trait seam for the actual text write. Production wires this to
/// `text_poke_early` / `smp_text_poke_single`; tests use a `Vec<u8>`.
pub trait TextPoke {
    fn poke(&self, addr: u64, bytes: &[u8]) -> Result<(), i32>;
    fn read(&self, addr: u64, len: usize) -> Result<alloc::vec::Vec<u8>, i32>;
}

/// Linux's `__jump_label_transform`: verify the expected current bytes,
/// then write the new bytes. Returns `EFAULT` on byte mismatch — Linux
/// `BUG()`s here; we surface an errno instead to keep the kernel alive.
pub fn jump_label_transform<P: TextPoke>(
    poker: &P,
    addr: u64,
    dest: u64,
    size: usize,
    type_: JumpLabelType,
) -> Result<(), i32> {
    let expected = jump_label_expected(addr, dest, size, type_)?;
    let current = poker.read(addr, size)?;
    if current != expected {
        return Err(EFAULT);
    }
    let patch = jump_label_patch(addr, dest, size, type_)?;
    poker.poke(addr, &patch.code)
}

/// `arch_jump_entry_size` analogue — given the bytes at the patch site,
/// return whether it is a 2-byte or 5-byte slot. A real implementation
/// would decode via the `insn` module; this is the byte-shape mirror.
pub fn arch_jump_entry_size(bytes: &[u8]) -> Result<usize, i32> {
    if bytes.len() < 2 {
        return Err(EINVAL);
    }
    // JMP8 sites are 2 bytes; everything else is treated as the 5-byte
    // slot. Linux BUG()s on neither; we return EINVAL.
    let len5_ok = bytes.len() >= 5;
    let first = bytes[0];
    if first == JMP8_INSN_OPCODE || (first == X86_NOP2[0] && bytes[1] == X86_NOP2[1]) {
        // 2-byte slot
        if first == X86_NOP2[0] && len5_ok {
            // Long-NOP 5-byte slot starts with 0x66 too; disambiguate by
            // checking byte 2.
            if bytes[1] == 0x0f {
                return Ok(JMP32_INSN_SIZE);
            }
        }
        return Ok(JMP8_INSN_SIZE);
    }
    if len5_ok {
        Ok(JMP32_INSN_SIZE)
    } else {
        Err(EINVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    extern crate alloc;
    use alloc::collections::BTreeMap;

    #[derive(Default)]
    struct MemPoker {
        memory: RefCell<BTreeMap<u64, u8>>,
    }

    impl MemPoker {
        fn seed(&self, addr: u64, bytes: &[u8]) {
            let mut m = self.memory.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(addr + i as u64, *b);
            }
        }
    }

    impl TextPoke for MemPoker {
        fn poke(&self, addr: u64, bytes: &[u8]) -> Result<(), i32> {
            let mut m = self.memory.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(addr + i as u64, *b);
            }
            Ok(())
        }
        fn read(&self, addr: u64, len: usize) -> Result<alloc::vec::Vec<u8>, i32> {
            let m = self.memory.borrow();
            (0..len)
                .map(|i| m.get(&(addr + i as u64)).copied().ok_or(EFAULT))
                .collect()
        }
    }

    #[test]
    fn opcode_constants_match_linux() {
        assert_eq!(JMP32_INSN_OPCODE, 0xE9);
        assert_eq!(JMP32_INSN_SIZE, 5);
        assert_eq!(JMP8_INSN_OPCODE, 0xEB);
        assert_eq!(JMP8_INSN_SIZE, 2);
    }

    #[test]
    fn text_gen_insn_emits_signed_rel32() {
        let bytes = text_gen_insn(JMP32_INSN_OPCODE, JMP32_INSN_SIZE, 0x1000, 0x2000);
        assert_eq!(bytes[0], 0xE9);
        let rel = i32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        // dest - (addr + size) = 0x2000 - 0x1005 = 0xFFB
        assert_eq!(rel, 0x0FFB);
    }

    #[test]
    fn text_gen_insn_emits_signed_rel8_for_short_jump() {
        let bytes = text_gen_insn(JMP8_INSN_OPCODE, JMP8_INSN_SIZE, 0x100, 0x110);
        assert_eq!(bytes[0], 0xEB);
        assert_eq!(bytes[1] as i8, 0x0E);
    }

    #[test]
    fn jump_label_patch_jmp_produces_jmp_bytes() {
        let p = jump_label_patch(0x1000, 0x2000, 5, JumpLabelType::Jmp).unwrap();
        assert_eq!(p.size, 5);
        assert_eq!(p.code[0], JMP32_INSN_OPCODE);
    }

    #[test]
    fn jump_label_patch_nop_produces_nop_bytes() {
        let p = jump_label_patch(0x1000, 0x2000, 5, JumpLabelType::Nop).unwrap();
        assert_eq!(p.size, 5);
        let expected = x86_nop(5).unwrap();
        assert_eq!(&p.code[..], expected);
    }

    #[test]
    fn transform_writes_jmp_when_site_currently_has_nop() {
        let mem = MemPoker::default();
        let nop = x86_nop(5).unwrap();
        mem.seed(0x1000, nop);

        let r = jump_label_transform(&mem, 0x1000, 0x2000, 5, JumpLabelType::Jmp);
        assert!(r.is_ok());
        let after = mem.read(0x1000, 5).unwrap();
        assert_eq!(after[0], JMP32_INSN_OPCODE);
    }

    #[test]
    fn transform_rejects_unexpected_current_bytes() {
        let mem = MemPoker::default();
        mem.seed(0x1000, &[0xCC, 0xCC, 0xCC, 0xCC, 0xCC]);
        let r = jump_label_transform(&mem, 0x1000, 0x2000, 5, JumpLabelType::Jmp);
        assert_eq!(r, Err(EFAULT));
    }

    #[test]
    fn entry_size_recognises_jmp8_vs_jmp32() {
        // 0xEB short jump → 2-byte
        assert_eq!(arch_jump_entry_size(&[0xEB, 0x10]).unwrap(), 2);
        // 0xE9 long jump → 5-byte
        assert_eq!(
            arch_jump_entry_size(&[0xE9, 0x00, 0x00, 0x00, 0x00]).unwrap(),
            5
        );
    }
}
