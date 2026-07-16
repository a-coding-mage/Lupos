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
use crate::kernel::module::{export_symbol, find_symbol};

use super::alternative::{CALL_INSN_OPCODE, JMP32_INSN_OPCODE, x86_nop};
use super::jump_label::text_gen_insn;

/// Linux `MCOUNT_INSN_SIZE` — every ftrace patch site is exactly 5 bytes.
pub const MCOUNT_INSN_SIZE: usize = 5;

/// Linux `MCOUNT_ADDR` — the address ftrace nominally calls when the
/// kernel is built with `-pg`. Patch-site recognition uses this.
pub const MCOUNT_ADDR_DEFAULT: u64 = 0xFFFF_FFFF_8000_0000;

// `__fentry__` is the relocation target emitted into every vendor object.
// Module formation rewrites those calls before the module can execute.  The
// no-op body is still a valid fallback during formation and is also the exact
// symbol address used to verify the original call instruction.
//
// `lupos_ftrace_caller` preserves the complete integer/flags state because an
// fentry call occurs before the compiler prologue.  The return address is the
// instrumented IP plus five; its caller's return address is the next stack
// word.  The Rust dispatcher only runs after all state has been saved.
core::arch::global_asm!(
    ".pushsection .text.lupos.ftrace, \"ax\"",
    ".balign 16",
    ".global __fentry__",
    ".type __fentry__,@function",
    "__fentry__:",
    "endbr64",
    "ret",
    ".size __fentry__,.-__fentry__",
    ".balign 16",
    ".global lupos_ftrace_caller",
    ".type lupos_ftrace_caller,@function",
    "lupos_ftrace_caller:",
    "endbr64",
    "pushfq",
    "push rax",
    "push rcx",
    "push rdx",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "sub rsp, 8",
    "mov rdi, [rsp + 136]",
    "sub rdi, 5",
    "mov rsi, [rsp + 144]",
    // Register-aware ftrace callbacks receive the instrumented function's
    // entry stack pointer and the original BP.  At fentry time that stack
    // pointer addresses the caller return address, which is exactly the
    // initial state expected by the module ORC unwinder.
    "lea rdx, [rsp + 144]",
    "mov rcx, [rsp + 88]",
    "call lupos_ftrace_dispatch",
    "add rsp, 8",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "popfq",
    "ret",
    ".size lupos_ftrace_caller,.-lupos_ftrace_caller",
    ".popsection",
);

unsafe extern "C" {
    pub fn __fentry__();
    pub fn lupos_ftrace_caller();
}

#[unsafe(no_mangle)]
extern "C" fn lupos_ftrace_dispatch(ip: u64, parent_ip: u64, sp: u64, bp: u64) {
    crate::kernel::trace::ftrace::ftrace_function_trace_call_with_regs(
        ip, parent_ip, sp, bp,
    );
}

pub fn mcount_addr() -> u64 {
    __fentry__ as usize as u64
}

pub fn ftrace_caller_addr() -> u64 {
    lupos_ftrace_caller as usize as u64
}

pub fn register_module_exports() {
    if find_symbol("__fentry__").is_none() {
        export_symbol("__fentry__", __fentry__ as usize, false);
    }
}

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

struct ProductionText;

impl KernelMem for ProductionText {
    fn read(&self, ip: u64, len: usize) -> Result<alloc::vec::Vec<u8>, i32> {
        super::alternative::text_poke_read(ip as usize, len)
    }
}

impl FtraceTextPoke for ProductionText {
    fn poke(&self, ip: u64, bytes: &[u8], late: bool) -> Result<(), i32> {
        if late {
            super::alternative::text_poke_live(ip as usize, bytes)
        } else {
            super::alternative::text_poke_early(ip as usize, bytes)
        }
    }
}

fn rel32_reachable(from: u64, to: u64) -> bool {
    let next = from.wrapping_add(MCOUNT_INSN_SIZE as u64);
    let displacement = to.wrapping_sub(next) as u32 as i32;
    next.wrapping_add_signed(displacement as i64) == to
}

/// Convert the compiler-emitted `CALL __fentry__` into the initial disabled
/// NOP form while module text is still writable.
pub fn prepare_module_callsite(ip: usize, trace_active: bool) -> Result<bool, i32> {
    let text = ProductionText;
    ftrace_make_nop(&text, &text, ip as u64, mcount_addr(), mcount_addr(), false)?;
    if trace_active {
        if !rel32_reachable(ip as u64, ftrace_caller_addr()) {
            return Err(EINVAL);
        }
        ftrace_make_call(&text, &text, ip as u64, ftrace_caller_addr(), false)?;
    }
    Ok(trace_active)
}

/// Switch a published callsite between NOP and the Lupos ftrace trampoline.
pub fn set_module_callsite(ip: usize, enabled: bool) -> Result<(), i32> {
    let text = ProductionText;
    if enabled {
        if !rel32_reachable(ip as u64, ftrace_caller_addr()) {
            return Err(EINVAL);
        }
        ftrace_make_call(&text, &text, ip as u64, ftrace_caller_addr(), true)
    } else {
        let old = ftrace_call_replace(ip as u64, ftrace_caller_addr(), false);
        let new = ftrace_nop_replace();
        ftrace_modify_code_direct(&text, &text, ip as u64, &old, &new, true)
    }
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
    fn rel32_reachability_accepts_the_canonical_address_wrap() {
        // Lupos follows Linux's top-of-address-space module window while the
        // boot image also has a low executable alias. x86 adds rel32 modulo
        // 2^64, so this is a valid positive displacement across address zero.
        let module_site = 0xffff_ffff_c000_6014;
        let low_kernel_target = 0x00b6_5260;
        assert!(rel32_reachable(module_site, low_kernel_target));
        let call = ftrace_call_replace(module_site, low_kernel_target, false);
        let displacement = i32::from_le_bytes(call[1..5].try_into().unwrap());
        assert_eq!(
            module_site
                .wrapping_add(MCOUNT_INSN_SIZE as u64)
                .wrapping_add_signed(displacement as i64),
            low_kernel_target
        );
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
