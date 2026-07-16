//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/static_call.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/static_call.c
//! x86 static-call patch byte generation.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/static_call.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::arch::x86::kernel::alternative::{
    CALL_INSN_OPCODE, JMP32_INSN_OPCODE, text_poke_copy, x86_nop,
};
use crate::arch::x86::kernel::jump_label::text_gen_insn;
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

pub const CALL_INSN_SIZE: usize = 5;
pub const RET_INSN_OPCODE: u8 = 0xc3;
pub const TRAMP_UD: [u8; 3] = [0x0f, 0xb9, 0xcc];
pub const XOR5RAX: [u8; 5] = [0x2e, 0x2e, 0x2e, 0x31, 0xc0];
pub const RETINSN: [u8; 5] = [RET_INSN_OPCODE, 0xcc, 0xcc, 0xcc, 0xcc];
pub const WARNINSN: [u8; 5] = [0x67, 0x48, 0x0f, 0xb9, 0x3a];
pub const STATIC_CALL_SITE_SIZE: usize = 8;
pub const STATIC_CALL_SITE_TAIL: usize = 1;
pub const STATIC_CALL_SITE_FLAGS: usize = 3;

// The real `__WARN_trap` body and its module-exported static-call trampoline,
// mirroring vendor/linux/arch/x86/entry/entry.S::__WARN_trap and
// vendor/linux/arch/x86/kernel/traps.c::EXPORT_STATIC_CALL_TRAMP(WARN_trap).
// The `ud1 (%edx), %rdi` byte sequence is exactly WARNINSN, the pattern
// decode_bug() classifies as BUG_UD1_WARN with the bug_entry pointer in
// pt_regs->di; the #UD handler reports the warning and resumes after the
// 5-byte insn, so the RET returns to the module caller.
//
// The 16-byte alignment is load-bearing: relocated `.static_call_sites` keys
// resolve to these exported addresses and are masked with
// STATIC_CALL_SITE_FLAGS before comparison
// (vendor/linux/kernel/static_call_inline.c::static_call_add_module uses the
// low key bits as INIT/TAIL flags).  A plain Rust fn has no alignment
// guarantee, so an unluckily placed stub made the masked comparison fail and
// every vendor module carrying WARN static-call sites (scsi_mod, libata,
// drm, snd, libphy, ...) was rejected at load.
core::arch::global_asm!(
    ".pushsection .text.lupos.warn_trap, \"ax\"",
    ".balign 16",
    ".global __WARN_trap",
    "__WARN_trap:",
    ".byte 0x67, 0x48, 0x0f, 0xb9, 0x3a", // ud1 (%edx), %rdi == WARNINSN
    "ret",
    ".balign 16",
    ".global __SCT__WARN_trap",
    "__SCT__WARN_trap:",
    "jmp __WARN_trap",
    ".popsection",
);

unsafe extern "C" {
    pub fn __WARN_trap(bug: *mut core::ffi::c_void, ...);
    pub fn __SCT__WARN_trap(bug: *mut core::ffi::c_void, ...);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticCallInsn {
    Call,
    Nop,
    Jmp,
    Ret,
    Jcc(u8),
}

pub fn is_jcc(insn: &[u8]) -> Option<u8> {
    if insn.len() >= 2 && insn[0] == 0x0f && (insn[1] & 0xf0) == 0x80 {
        Some(insn[1])
    } else {
        None
    }
}

pub const fn sc_insn(null: bool, tail: bool) -> StaticCallInsn {
    match (tail, null) {
        (false, false) => StaticCallInsn::Call,
        (false, true) => StaticCallInsn::Nop,
        (true, false) => StaticCallInsn::Jmp,
        (true, true) => StaticCallInsn::Ret,
    }
}

pub fn static_call_transform_bytes(
    site: u64,
    current: &[u8],
    kind: StaticCallInsn,
    func: u64,
) -> Result<Vec<u8>, i32> {
    if let StaticCallInsn::Jmp | StaticCallInsn::Ret = kind {
        if let Some(op) = is_jcc(current) {
            return Ok(static_call_jcc(site, op, func));
        }
    }
    match kind {
        StaticCallInsn::Call => Ok(text_gen_insn(CALL_INSN_OPCODE, CALL_INSN_SIZE, site, func)),
        StaticCallInsn::Nop => x86_nop(CALL_INSN_SIZE)
            .map(|bytes| bytes.to_vec())
            .ok_or(EINVAL),
        StaticCallInsn::Jmp => Ok(text_gen_insn(JMP32_INSN_OPCODE, CALL_INSN_SIZE, site, func)),
        StaticCallInsn::Ret => Ok(RETINSN.to_vec()),
        StaticCallInsn::Jcc(op) => Ok(static_call_jcc(site, op, func)),
    }
}

pub fn warn_trap_addr() -> usize {
    __WARN_trap as usize
}

pub fn warn_trap_trampoline_addr() -> usize {
    __SCT__WARN_trap as usize
}

pub fn register_module_exports() {
    if find_symbol("__WARN_trap").is_none() {
        export_symbol("__WARN_trap", warn_trap_addr(), false);
    }
    if find_symbol("__SCT__WARN_trap").is_none() {
        export_symbol("__SCT__WARN_trap", warn_trap_trampoline_addr(), false);
    }
}

pub fn static_call_fixup_warn_site(site: &mut [u8]) -> Result<(), i32> {
    static_call_validate(site, false, false)?;
    text_poke_copy(&mut site[..WARNINSN.len()], &WARNINSN)
}

pub fn static_call_jcc(site: u64, op: u8, func: u64) -> Vec<u8> {
    let mut out = alloc::vec![0x0f, op, 0, 0, 0, 0];
    let rel = (func as i64).wrapping_sub((site + 6) as i64) as i32;
    out[2..6].copy_from_slice(&rel.to_le_bytes());
    out
}

pub fn static_call_validate(insn: &[u8], tail: bool, tramp: bool) -> Result<(), i32> {
    if tramp && (insn.len() < 8 || insn[5..8] != TRAMP_UD) {
        return Err(EINVAL);
    }
    let op = insn.first().copied().ok_or(EINVAL)?;
    if tail {
        if op == JMP32_INSN_OPCODE || op == RET_INSN_OPCODE || is_jcc(insn).is_some() {
            return Ok(());
        }
    } else if op == CALL_INSN_OPCODE
        || insn.get(..5) == Some(x86_nop(5).unwrap_or(&[]))
        || insn.get(..5) == Some(&XOR5RAX)
        || insn.get(..5) == Some(&WARNINSN)
    {
        return Ok(());
    }
    Err(EINVAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_match_linux_static_call_shapes() {
        let call =
            static_call_transform_bytes(0x1000, &[0xe8, 0, 0, 0, 0], StaticCallInsn::Call, 0x2000)
                .unwrap();
        assert_eq!(call[0], CALL_INSN_OPCODE);
        let nop = static_call_transform_bytes(0x1000, &call, StaticCallInsn::Nop, 0).unwrap();
        assert_eq!(nop.len(), 5);
        let ret = static_call_transform_bytes(0x1000, &[RET_INSN_OPCODE], StaticCallInsn::Ret, 0)
            .unwrap();
        assert_eq!(ret, RETINSN);
    }

    #[test]
    fn validates_tail_and_trampoline_signatures() {
        assert!(static_call_validate(&[JMP32_INSN_OPCODE, 0, 0, 0, 0], true, false).is_ok());
        assert!(static_call_validate(&[0xe8, 0, 0, 0, 0, 0x0f, 0xb9, 0xcc], false, true).is_ok());
        assert_eq!(static_call_validate(&[0x90; 8], false, true), Err(EINVAL));
    }
}
