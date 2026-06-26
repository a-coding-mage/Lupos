//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/ftrace.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/ftrace.c
//! Dynamic function tracing — patch site preparation.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/ftrace.c
//!
//! `ftrace` instruments compiler-emitted call sites: each function's
//! prologue contains an `mcount` / `__fentry__` stub that ftrace flips
//! between `CALL <trampoline>` and a 5-byte `NOP`. Live text mutation
//! goes behind a fail-closed seam, but the byte-level encode /
//! verification / NOP-replace algorithm is testable here.

#![allow(dead_code)]

extern crate alloc;

use crate::include::uapi::errno::{EFAULT, EINVAL};

use super::alternative::{CALL_INSN_OPCODE, JMP32_INSN_OPCODE, x86_nop};
use super::jump_label::text_gen_insn;

/// Linux `MCOUNT_INSN_SIZE` — every ftrace patch site is exactly 5 bytes.
pub const MCOUNT_INSN_SIZE: usize = 5;

/// Linux `MCOUNT_ADDR` — the address ftrace nominally calls when the
/// kernel is built with `-pg`. Patch-site recognition uses this.
pub const MCOUNT_ADDR_DEFAULT: u64 = 0xFFFF_FFFF_8000_0000;

/// `ftrace_nop_replace()` — 5-byte NOP.
pub fn ftrace_nop_replace() -> alloc::vec::Vec<u8> {
    x86_nop(MCOUNT_INSN_SIZE)
        .map(|n| n.to_vec())
        .unwrap_or_default()
}

/// `ftrace_call_replace(ip, addr)` — generate a `CALL rel32` to `addr`.
/// When `addr` is a tail-jump (Linux's `ftrace_is_jmp` flag), the
/// opcode is `JMP rel32` instead. We mirror this by accepting a flag.
pub fn ftrace_call_replace(ip: u64, addr: u64, jmp: bool) -> alloc::vec::Vec<u8> {
    let opcode = if jmp {
        JMP32_INSN_OPCODE
    } else {
        CALL_INSN_OPCODE
    };
    text_gen_insn(opcode, MCOUNT_INSN_SIZE, ip, addr)
}

/// Trait seam for `copy_from_kernel_nofault` (used by `ftrace_verify_code`).
pub trait KernelMem {
    fn read(&self, ip: u64, len: usize) -> Result<alloc::vec::Vec<u8>, i32>;
}

/// Trait seam for `text_poke_early` / `smp_text_poke_batch_add` — the
/// production write goes through a fail-closed implementation that the
/// patcher subsystem owns.
pub trait FtraceTextPoke {
    fn poke(&self, ip: u64, bytes: &[u8], late: bool) -> Result<(), i32>;
}

/// `ftrace_verify_code(ip, old_code)` — read 5 bytes from `ip` and
/// compare against the expected pattern. Returns `EFAULT` if the read
/// fails or `EINVAL` if the bytes don't match.
pub fn ftrace_verify_code<M: KernelMem>(mem: &M, ip: u64, old_code: &[u8]) -> Result<(), i32> {
    if old_code.len() != MCOUNT_INSN_SIZE {
        return Err(EINVAL);
    }
    let cur = mem.read(ip, MCOUNT_INSN_SIZE).map_err(|_| EFAULT)?;
    if cur != old_code {
        return Err(EINVAL);
    }
    Ok(())
}

/// `ftrace_modify_code_direct(ip, old, new)` — verify then write.
pub fn ftrace_modify_code_direct<M: KernelMem, P: FtraceTextPoke>(
    mem: &M,
    poker: &P,
    ip: u64,
    old_code: &[u8],
    new_code: &[u8],
    late: bool,
) -> Result<(), i32> {
    ftrace_verify_code(mem, ip, old_code)?;
    if new_code.len() != MCOUNT_INSN_SIZE {
        return Err(EINVAL);
    }
    poker.poke(ip, new_code, late)
}

/// `ftrace_make_nop(rec, addr)` — flip a `CALL addr` site to `NOP`.
/// Only valid when `addr == MCOUNT_ADDR` (mirrors Linux's invariant —
/// every other addr would have gone through the breakpoint path).
pub fn ftrace_make_nop<M: KernelMem, P: FtraceTextPoke>(
    mem: &M,
    poker: &P,
    ip: u64,
    addr: u64,
    mcount_addr: u64,
    late: bool,
) -> Result<(), i32> {
    if addr != mcount_addr {
        return Err(EINVAL);
    }
    let old = ftrace_call_replace(ip, addr, false);
    let new = ftrace_nop_replace();
    ftrace_modify_code_direct(mem, poker, ip, &old, &new, late)
}

/// `ftrace_make_call(rec, addr)` — flip a `NOP` site to `CALL addr`.
pub fn ftrace_make_call<M: KernelMem, P: FtraceTextPoke>(
    mem: &M,
    poker: &P,
    ip: u64,
    addr: u64,
    late: bool,
) -> Result<(), i32> {
    let old = ftrace_nop_replace();
    let new = ftrace_call_replace(ip, addr, false);
    ftrace_modify_code_direct(mem, poker, ip, &old, &new, late)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    #[derive(Default)]
    struct MemMap {
        bytes: RefCell<BTreeMap<u64, u8>>,
    }

    impl MemMap {
        fn seed(&self, ip: u64, bytes: &[u8]) {
            let mut m = self.bytes.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(ip + i as u64, *b);
            }
        }
        fn dump(&self, ip: u64, len: usize) -> alloc::vec::Vec<u8> {
            let m = self.bytes.borrow();
            (0..len)
                .map(|i| *m.get(&(ip + i as u64)).unwrap_or(&0))
                .collect()
        }
    }

    impl KernelMem for MemMap {
        fn read(&self, ip: u64, len: usize) -> Result<alloc::vec::Vec<u8>, i32> {
            let m = self.bytes.borrow();
            (0..len)
                .map(|i| m.get(&(ip + i as u64)).copied().ok_or(EFAULT))
                .collect()
        }
    }

    impl FtraceTextPoke for MemMap {
        fn poke(&self, ip: u64, bytes: &[u8], _late: bool) -> Result<(), i32> {
            let mut m = self.bytes.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(ip + i as u64, *b);
            }
            Ok(())
        }
    }

    #[test]
    fn mcount_insn_size_is_5() {
        assert_eq!(MCOUNT_INSN_SIZE, 5);
    }

    #[test]
    fn nop_replace_returns_5_bytes() {
        let nop = ftrace_nop_replace();
        assert_eq!(nop.len(), 5);
    }

    #[test]
    fn call_replace_emits_e8_with_rel32() {
        let bytes = ftrace_call_replace(0x1000, 0x2000, false);
        assert_eq!(bytes.len(), 5);
        assert_eq!(bytes[0], CALL_INSN_OPCODE);
    }

    #[test]
    fn call_replace_with_jmp_emits_e9_opcode() {
        let bytes = ftrace_call_replace(0x1000, 0x2000, true);
        assert_eq!(bytes[0], JMP32_INSN_OPCODE);
    }

    #[test]
    fn verify_code_accepts_matching_bytes() {
        let mem = MemMap::default();
        let bytes = ftrace_call_replace(0x1000, 0x2000, false);
        mem.seed(0x1000, &bytes);
        assert!(ftrace_verify_code(&mem, 0x1000, &bytes).is_ok());
    }

    #[test]
    fn verify_code_rejects_mismatching_bytes() {
        let mem = MemMap::default();
        mem.seed(0x1000, &[0xCC, 0xCC, 0xCC, 0xCC, 0xCC]);
        let expected = ftrace_call_replace(0x1000, 0x2000, false);
        assert_eq!(ftrace_verify_code(&mem, 0x1000, &expected), Err(EINVAL));
    }

    #[test]
    fn verify_code_rejects_wrong_length_pattern() {
        let mem = MemMap::default();
        assert_eq!(ftrace_verify_code(&mem, 0x1000, &[0xCC; 4]), Err(EINVAL));
    }

    #[test]
    fn modify_code_direct_writes_new_bytes() {
        let mem = MemMap::default();
        let old = ftrace_nop_replace();
        let new = ftrace_call_replace(0x1000, 0x2000, false);
        mem.seed(0x1000, &old);
        ftrace_modify_code_direct(&mem, &mem, 0x1000, &old, &new, false).unwrap();
        assert_eq!(mem.dump(0x1000, 5), new);
    }

    #[test]
    fn make_nop_requires_mcount_addr_match() {
        let mem = MemMap::default();
        let r = ftrace_make_nop(&mem, &mem, 0x1000, 0xDEAD, MCOUNT_ADDR_DEFAULT, false);
        assert_eq!(r, Err(EINVAL));
    }

    #[test]
    fn make_nop_flips_call_site_to_nop() {
        let mem = MemMap::default();
        let mcount = 0xFFFF_FFFF_8000_1000;
        let call = ftrace_call_replace(0x2000, mcount, false);
        mem.seed(0x2000, &call);
        ftrace_make_nop(&mem, &mem, 0x2000, mcount, mcount, false).unwrap();
        let nop = ftrace_nop_replace();
        assert_eq!(mem.dump(0x2000, 5), nop);
    }

    #[test]
    fn make_call_flips_nop_site_to_call() {
        let mem = MemMap::default();
        let nop = ftrace_nop_replace();
        mem.seed(0x3000, &nop);
        ftrace_make_call(&mem, &mem, 0x3000, 0x4000, false).unwrap();
        let call = ftrace_call_replace(0x3000, 0x4000, false);
        assert_eq!(mem.dump(0x3000, 5), call);
    }
}
