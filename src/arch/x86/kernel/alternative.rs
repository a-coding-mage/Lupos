//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/alternative.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/alternative.c
//! x86 alternative instruction patching helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/alternative.c
//!
//! Linux rewrites instruction sites during early boot and module load. Lupos
//! keeps live text mutation behind a fail-closed seam, but the byte-level
//! NOP, relocation, and patch-site preparation rules are kept testable here.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

pub const MAX_PATCH_LEN: usize = 254;

pub const DA_ALT: u32 = 0x01;
pub const DA_RET: u32 = 0x02;
pub const DA_RETPOLINE: u32 = 0x04;
pub const DA_ENDBR: u32 = 0x08;
pub const DA_SMP: u32 = 0x10;

pub const ALT_FLAGS_SHIFT: u32 = 16;
pub const ALT_FLAG_NOT: u16 = 1 << 0;
pub const ALT_FLAG_DIRECT_CALL: u16 = 1 << 1;

pub const CALL_INSN_OPCODE: u8 = 0xe8;
pub const JMP32_INSN_OPCODE: u8 = 0xe9;
pub const RET_INSN_OPCODE: u8 = 0xc3;

pub static ALTERNATIVES_PATCHED: AtomicBool = AtomicBool::new(false);

pub const X86_NOP1: &[u8] = &[0x90];
pub const X86_NOP2: &[u8] = &[0x66, 0x90];
pub const X86_NOP3: &[u8] = &[0x0f, 0x1f, 0x00];
pub const X86_NOP4: &[u8] = &[0x0f, 0x1f, 0x40, 0x00];
pub const X86_NOP5: &[u8] = &[0x0f, 0x1f, 0x44, 0x00, 0x00];
pub const X86_NOP6: &[u8] = &[0x66, 0x0f, 0x1f, 0x44, 0x00, 0x00];
pub const X86_NOP7: &[u8] = &[0x0f, 0x1f, 0x80, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP8: &[u8] = &[0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP9: &[u8] = &[0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP10: &[u8] = &[0x66, 0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP11: &[u8] = &[
    0x66, 0x66, 0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub const ASM_NOP_MAX: usize = 11;

pub fn x86_nop(len: usize) -> Option<&'static [u8]> {
    match len {
        1 => Some(X86_NOP1),
        2 => Some(X86_NOP2),
        3 => Some(X86_NOP3),
        4 => Some(X86_NOP4),
        5 => Some(X86_NOP5),
        6 => Some(X86_NOP6),
        7 => Some(X86_NOP7),
        8 => Some(X86_NOP8),
        9 => Some(X86_NOP9),
        10 => Some(X86_NOP10),
        11 => Some(X86_NOP11),
        _ => None,
    }
}

pub fn add_nops(out: &mut [u8]) {
    let mut off = 0;
    while off < out.len() {
        let chunk = (out.len() - off).min(ASM_NOP_MAX);
        let nop = x86_nop(chunk).expect("chunk is <= ASM_NOP_MAX and non-zero");
        out[off..off + chunk].copy_from_slice(nop);
        off += chunk;
    }
}

pub fn is_nop_at(bytes: &[u8], offset: usize) -> Option<usize> {
    if offset >= bytes.len() {
        return None;
    }
    for len in (1..=ASM_NOP_MAX).rev() {
        if offset + len <= bytes.len() && x86_nop(len) == Some(&bytes[offset..offset + len]) {
            return Some(len);
        }
    }
    if bytes[offset] == 0x90 { Some(1) } else { None }
}

pub fn skip_nops(bytes: &[u8], mut offset: usize) -> usize {
    while let Some(len) = is_nop_at(bytes, offset) {
        offset += len;
    }
    offset
}

pub fn apply_reloc(width: usize, value: u64, diff: i64) -> Result<u64, i32> {
    let mask = match width {
        1 => 0xff,
        2 => 0xffff,
        4 => 0xffff_ffff,
        8 => u64::MAX,
        _ => return Err(EINVAL),
    };
    Ok(value.wrapping_add(diff as u64) & mask)
}

pub const fn need_reloc(offset: usize, src_len: usize) -> bool {
    offset < src_len
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AltInstr {
    pub cpuid: u16,
    pub instrlen: u8,
    pub replacementlen: u8,
    pub flags: u16,
}

impl AltInstr {
    pub const fn should_patch(self, feature_present: bool) -> bool {
        let patch_when_not = (self.flags & ALT_FLAG_NOT) != 0;
        feature_present != patch_when_not
    }
}

pub fn prepare_patch_site(
    original: &[u8],
    replacement: Option<&[u8]>,
    feature_present: bool,
    alt: AltInstr,
) -> Result<Vec<u8>, i32> {
    if original.len() > MAX_PATCH_LEN {
        return Err(EINVAL);
    }
    if !alt.should_patch(feature_present) {
        return Ok(original.to_vec());
    }

    let repl = replacement.ok_or(EINVAL)?;
    if repl.len() > original.len() {
        return Err(EINVAL);
    }

    let mut out = vec![0u8; original.len()];
    out[..repl.len()].copy_from_slice(repl);
    add_nops(&mut out[repl.len()..]);
    Ok(out)
}

pub fn text_poke_copy(dst: &mut [u8], opcode: &[u8]) -> Result<(), i32> {
    if dst.len() != opcode.len() {
        return Err(EINVAL);
    }
    dst.copy_from_slice(opcode);
    Ok(())
}

pub fn text_poke_set(dst: &mut [u8], byte: u8) {
    dst.fill(byte);
}

pub const fn live_text_poke_supported() -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

pub fn mark_alternatives_patched() {
    ALTERNATIVES_PATCHED.store(true, Ordering::Release);
}

pub fn alternatives_patched() -> bool {
    ALTERNATIVES_PATCHED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop_sequences_match_linux_64bit_table() {
        assert_eq!(ASM_NOP_MAX, 11);
        assert_eq!(x86_nop(1), Some(&[0x90][..]));
        assert_eq!(x86_nop(5), Some(&[0x0f, 0x1f, 0x44, 0x00, 0x00][..]));
        assert_eq!(
            x86_nop(11),
            Some(&[0x66, 0x66, 0x2e, 0x0f, 0x1f, 0x84, 0, 0, 0, 0, 0][..])
        );
    }

    #[test]
    fn add_nops_uses_longest_chunks() {
        let mut bytes = [0xcc; 13];
        add_nops(&mut bytes);
        assert_eq!(&bytes[..11], X86_NOP11);
        assert_eq!(&bytes[11..], X86_NOP2);
    }

    #[test]
    fn skip_nops_walks_mixed_linux_nops() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(X86_NOP5);
        bytes.extend_from_slice(X86_NOP2);
        bytes.push(0xcc);
        assert_eq!(skip_nops(&bytes, 0), 7);
    }

    #[test]
    fn alt_flag_not_inverts_feature_predicate() {
        let alt = AltInstr {
            cpuid: 7,
            instrlen: 4,
            replacementlen: 1,
            flags: ALT_FLAG_NOT,
        };
        assert!(!alt.should_patch(true));
        assert!(alt.should_patch(false));
    }

    #[test]
    fn prepare_patch_site_pads_replacement_with_linux_nops() {
        let alt = AltInstr {
            cpuid: 1,
            instrlen: 5,
            replacementlen: 1,
            flags: 0,
        };
        let out = prepare_patch_site(&[0xcc; 5], Some(&[RET_INSN_OPCODE]), true, alt).unwrap();
        assert_eq!(out[0], RET_INSN_OPCODE);
        assert_eq!(&out[1..], X86_NOP4);
    }

    #[test]
    fn apply_reloc_wraps_to_requested_width() {
        assert_eq!(apply_reloc(1, 0xff, 2), Ok(1));
        assert_eq!(apply_reloc(4, 0xffff_fffe, 3), Ok(1));
        assert_eq!(apply_reloc(3, 0, 1), Err(EINVAL));
    }

    #[test]
    fn live_text_poke_fails_closed() {
        assert_eq!(live_text_poke_supported(), Err(EOPNOTSUPP));
    }
}
