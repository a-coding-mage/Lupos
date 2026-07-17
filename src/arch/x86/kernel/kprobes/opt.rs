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

use crate::arch::x86::kernel::alternative::ENDBR64;
use crate::arch::x86::kernel::alternative::JMP32_INSN_OPCODE;
use crate::arch::x86::kernel::jump_label::text_gen_insn;
use crate::arch::x86::lib::insn::MAX_INSN_SIZE;
use crate::include::uapi::errno::{EINVAL, ENOMEM};

use super::core::{
    KprobeTextPoke, RELATIVE_INSN_SIZE, can_boost, copy_instruction, decode_instruction,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimizedKprobe {
    pub addr: u64,
    pub detour_addr: u64,
    pub optimized_len: usize,
    pub saved: Vec<u8>,
    pub jump: Vec<u8>,
    pub optimized: bool,
}

pub struct LiveOptimizedKprobe {
    pub arch: OptimizedKprobe,
    slot: usize,
}

unsafe impl Send for LiveOptimizedKprobe {}

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

fn copy_relocated_instructions(
    ip: u64,
    detour_ip: u64,
    bytes: &[u8],
) -> Result<(usize, Vec<u8>), i32> {
    let mut offset = 0usize;
    let mut relocated = Vec::new();
    while offset < RELATIVE_INSN_SIZE {
        let instruction = decode_instruction(&bytes[offset..])?;
        let len = instruction.length as usize;
        if len == 0 || offset + len > bytes.len() || !can_boost(&bytes[offset..offset + len]) {
            return Err(EINVAL);
        }
        let copied = copy_instruction(
            ip + offset as u64,
            detour_ip + offset as u64,
            &bytes[offset..offset + len],
        )?;
        relocated.extend_from_slice(&copied.bytes[..copied.len]);
        offset += len;
    }
    Ok((offset, relocated))
}

pub fn jump_target(ip: u64, bytes: &[u8]) -> Option<u64> {
    match bytes.first().copied()? {
        0xe9 | 0xe8 if bytes.len() >= 5 => {
            let rel = i32::from_le_bytes(bytes[1..5].try_into().ok()?) as i64;
            Some((ip as i64 + 5 + rel) as u64)
        }
        0xeb | 0x70..=0x7f | 0xe0..=0xe3 if bytes.len() >= 2 => {
            Some((ip as i64 + 2 + bytes[1] as i8 as i64) as u64)
        }
        0x0f if bytes.len() >= 6
            && bytes
                .get(1)
                .is_some_and(|opcode| (0x80..=0x8f).contains(opcode)) =>
        {
            let rel = i32::from_le_bytes(bytes[2..6].try_into().ok()?) as i64;
            Some((ip as i64 + 6 + rel) as u64)
        }
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

fn function_has_jump_into_range(
    function_ip: u64,
    bytes: &[u8],
    optimized_ip: u64,
    optimized_len: usize,
) -> Result<bool, i32> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        let instruction = decode_instruction(&bytes[offset..])?;
        let len = instruction.length as usize;
        if len == 0 || offset.checked_add(len).is_none_or(|end| end > bytes.len()) {
            return Err(EINVAL);
        }
        let ip = function_ip.wrapping_add(offset as u64);
        if jump_target(ip, &bytes[offset..offset + len]).is_some_and(|target| {
            target > optimized_ip && target < optimized_ip + optimized_len as u64
        }) {
            return Ok(true);
        }
        offset += len;
    }
    Ok(false)
}

fn optimized_region_has_exception_entry(
    optimized_ip: u64,
    bytes: &[u8],
    optimized_len: usize,
    mut lookup: impl FnMut(u64) -> bool,
) -> Result<bool, i32> {
    let mut offset = 0usize;
    while offset < optimized_len {
        if lookup(optimized_ip.wrapping_add(offset as u64)) {
            return Ok(true);
        }
        let instruction = decode_instruction(bytes.get(offset..).ok_or(EINVAL)?)?;
        let len = instruction.length as usize;
        if len == 0 || offset.checked_add(len).is_none_or(|end| end > bytes.len()) {
            return Err(EINVAL);
        }
        offset += len;
    }
    Ok(false)
}

#[cfg(not(test))]
fn validate_module_branch_targets(optimized_ip: u64, optimized_len: usize) -> Result<(), i32> {
    let Some(symbol) = crate::kernel::module::kallsyms::lookup_address(optimized_ip as usize)
    else {
        // Built-in text does not currently have a runtime symbol-size table;
        // its local relocatability checks still apply below. Loaded modules
        // do have exact post-relocation kallsyms bounds and take this stronger
        // path before any text is changed.
        return Ok(());
    };
    if symbol.size == 0 || !matches!(symbol.symbol_type, b't' | b'T') {
        return Err(EINVAL);
    }
    let bytes = crate::arch::x86::kernel::alternative::text_poke_read(symbol.address, symbol.size)?;
    if function_has_jump_into_range(symbol.address as u64, &bytes, optimized_ip, optimized_len)? {
        Err(EINVAL)
    } else {
        Ok(())
    }
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
    // Optimization replaces only the five-byte JMP32 footprint. The decoded
    // instruction span can be longer (for example one six-byte RIP-relative
    // instruction); rewriting that untouched tail can race with another text
    // patcher and is not part of undoing this detour.
    let original_jump_bytes = kp.saved.get(..RELATIVE_INSN_SIZE).ok_or(EINVAL)?;
    poker.poke(kp.addr, original_jump_bytes)?;
    kp.optimized = false;
    Ok(())
}

pub const fn arch_within_optimized_kprobe(ip: u64, kp: &OptimizedKprobe) -> bool {
    ip >= kp.addr && ip < kp.addr + kp.optimized_len as u64
}

pub const fn setup_detour_execution_supported() -> Result<(), i32> {
    Ok(())
}

fn append_detour_prologue(code: &mut Vec<u8>, probe_addr: u64) {
    code.extend_from_slice(&ENDBR64);
    code.push(0x9c); // pushfq
    code.extend_from_slice(&[0x50, 0x51, 0x52, 0x53, 0x55, 0x56, 0x57]);
    for register in 0..8u8 {
        code.extend_from_slice(&[0x41, 0x50 + register]); // push r8..r15
    }
    code.extend_from_slice(&[0x49, 0x89, 0xe4]); // mov r12, rsp
    code.extend_from_slice(&[0x48, 0x83, 0xe4, 0xf0]); // and rsp, -16
    code.extend_from_slice(&[0x48, 0xbf]); // movabs rdi, probe_addr
    code.extend_from_slice(&probe_addr.to_le_bytes());
    code.extend_from_slice(&[0x48, 0xb8]); // movabs rax, callback
    code.extend_from_slice(&(lupos_optimized_kprobe_dispatch as usize as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0]); // call rax
    code.extend_from_slice(&[0x4c, 0x89, 0xe4]); // mov rsp, r12
    for register in (0..8u8).rev() {
        code.extend_from_slice(&[0x41, 0x58 + register]); // pop r15..r8
    }
    code.extend_from_slice(&[0x5f, 0x5e, 0x5d, 0x5b, 0x5a, 0x59, 0x58]);
    code.push(0x9d); // popfq
}

fn rel32_reachable(from_next: u64, target: u64) -> bool {
    let displacement = target.wrapping_sub(from_next) as u32 as i32;
    from_next.wrapping_add_signed(displacement as i64) == target
}

#[unsafe(no_mangle)]
extern "C" fn lupos_optimized_kprobe_dispatch(addr: u64) {
    crate::kernel::trace::kprobe::fire_optimized_kprobe(addr);
}

#[cfg(not(test))]
pub fn prepare_live_optimized_kprobe(addr: u64) -> Result<LiveOptimizedKprobe, i32> {
    let adjusted = super::core::adjust_live_kprobe_addr(addr)?;
    let original =
        crate::arch::x86::kernel::alternative::text_poke_read(adjusted as usize, MAX_INSN_SIZE)?;
    let slot = crate::arch::x86::mm::init::execmem_alloc_rw(
        crate::arch::x86::mm::paging::PAGE_SIZE as usize,
    );
    if slot.is_null() {
        return Err(ENOMEM);
    }
    let result = (|| {
        let mut code = Vec::new();
        append_detour_prologue(&mut code, adjusted);
        let copied_ip = slot as u64 + code.len() as u64;
        let (optimized_len, copied) = copy_relocated_instructions(adjusted, copied_ip, &original)?;
        // A fault in relocated detour text cannot use an exception-table
        // entry keyed to the original instruction address. Linux rejects
        // such optimized regions rather than silently losing their fixup.
        if optimized_region_has_exception_entry(adjusted, &original, optimized_len, |ip| {
            crate::arch::x86::kernel::extable::search_extable(ip).is_some()
        })? {
            return Err(EINVAL);
        }
        validate_module_branch_targets(adjusted, optimized_len)?;
        code.extend_from_slice(&copied);
        let jump_ip = slot as u64 + code.len() as u64;
        let resume = adjusted + optimized_len as u64;
        if !rel32_reachable(jump_ip + RELATIVE_INSN_SIZE as u64, resume)
            || !rel32_reachable(adjusted + RELATIVE_INSN_SIZE as u64, slot as u64)
        {
            return Err(EINVAL);
        }
        code.extend_from_slice(&text_gen_insn(
            JMP32_INSN_OPCODE,
            RELATIVE_INSN_SIZE,
            jump_ip,
            resume,
        ));
        if code.len() > crate::arch::x86::mm::paging::PAGE_SIZE as usize {
            return Err(ENOMEM);
        }
        unsafe { core::ptr::copy_nonoverlapping(code.as_ptr(), slot, code.len()) };
        crate::arch::x86::mm::init::execmem_set_final_permissions(
            slot,
            crate::arch::x86::mm::paging::PAGE_SIZE as usize,
            false,
            true,
        )?;
        let jump = text_gen_insn(JMP32_INSN_OPCODE, RELATIVE_INSN_SIZE, adjusted, slot as u64);
        Ok(LiveOptimizedKprobe {
            arch: OptimizedKprobe {
                addr: adjusted,
                detour_addr: slot as u64,
                optimized_len,
                saved: original[..optimized_len].to_vec(),
                jump,
                optimized: false,
            },
            slot: slot as usize,
        })
    })();
    if result.is_err() {
        crate::arch::x86::mm::init::execmem_free(slot);
    }
    result
}

pub fn arm_live_optimized_kprobe(kprobe: &mut LiveOptimizedKprobe) -> Result<(), i32> {
    if kprobe.arch.optimized {
        return Ok(());
    }
    arch_optimize_kprobe(&super::core::ProductionText, &mut kprobe.arch)
}

pub fn disarm_live_optimized_kprobe(kprobe: &mut LiveOptimizedKprobe) -> Result<(), i32> {
    if !kprobe.arch.optimized {
        return Ok(());
    }
    arch_unoptimize_kprobe(&super::core::ProductionText, &mut kprobe.arch)
}

pub fn free_live_optimized_kprobe(mut kprobe: LiveOptimizedKprobe) {
    if kprobe.arch.optimized {
        let _ = disarm_live_optimized_kprobe(&mut kprobe);
    }
    #[cfg(not(test))]
    crate::arch::x86::mm::init::execmem_free(kprobe.slot as *mut u8);
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
    fn unoptimize_restores_only_the_jump_footprint() {
        // One six-byte RIP-relative instruction covers the five-byte detour.
        let original = [0x8b, 0x05, 0x10, 0, 0, 0];
        let mem = Mem::default();
        let mut kp = arch_prepare_optimized_kprobe(0x1000, 0x2000, &original).unwrap();
        assert_eq!(kp.optimized_len, 6);

        arch_optimize_kprobe(&mem, &mut kp).unwrap();
        mem.0.borrow_mut().clear();
        arch_unoptimize_kprobe(&mem, &mut kp).unwrap();

        let writes = mem.0.borrow();
        assert_eq!(writes.len(), RELATIVE_INSN_SIZE);
        assert_eq!(writes.get(&0x1004), Some(&original[4]));
        assert!(!writes.contains_key(&0x1005));
    }

    #[test]
    fn detects_jump_into_optimized_range() {
        assert!(has_jump_into_range(
            0x1000,
            8,
            &[(0x2000, &[0xe9, 0xff, 0xef, 0xff, 0xff])]
        ));
    }

    #[test]
    fn scans_a_complete_function_for_short_and_near_branches_into_detour() {
        // jmp +2 targets byte 4 of the five-byte replacement.
        assert_eq!(
            function_has_jump_into_range(0x1000, &[0xeb, 0x02, 0x90, 0x90, 0x90], 0x1000, 5),
            Ok(true)
        );

        // 0f 84 rel32 at 0x2000 targets 0x1003.
        let rel = (0x1003i64 - 0x2006i64) as i32;
        let mut near = [0u8; 6];
        near[..2].copy_from_slice(&[0x0f, 0x84]);
        near[2..].copy_from_slice(&rel.to_le_bytes());
        assert_eq!(
            function_has_jump_into_range(0x2000, &near, 0x1000, 5),
            Ok(true)
        );

        assert_eq!(
            function_has_jump_into_range(0x3000, &[0xeb, 0x03, 0x90, 0x90, 0x90], 0x3000, 5),
            Ok(false),
            "a branch to the first byte after the replacement is safe"
        );
    }

    #[test]
    fn rejects_an_exception_table_entry_in_any_relocated_instruction() {
        let instructions = [0x90, 0x90, 0x90, 0x90, 0x90];
        assert_eq!(
            optimized_region_has_exception_entry(0x1000, &instructions, 5, |ip| ip == 0x1003),
            Ok(true)
        );
        assert_eq!(
            optimized_region_has_exception_entry(0x1000, &instructions, 5, |_| false),
            Ok(false)
        );
    }

    #[test]
    fn optimized_copy_relocates_rip_relative_operands() {
        // mov eax, [rip + 0x10] is six bytes and therefore covers JMP32.
        let original = [0x8b, 0x05, 0x10, 0, 0, 0];
        let (len, relocated) = copy_relocated_instructions(0x1000, 0x2000, &original).unwrap();
        assert_eq!(len, 6);
        let displacement = i32::from_le_bytes(relocated[2..6].try_into().unwrap());
        assert_eq!(0x2006u64.wrapping_add_signed(displacement as i64), 0x1016);
    }

    #[test]
    fn detour_prologue_preserves_state_and_calls_the_probe_dispatcher() {
        let mut code = Vec::new();
        append_detour_prologue(&mut code, 0x1234);
        assert!(code.starts_with(&ENDBR64));
        assert_eq!(code[4], 0x9c); // pushfq
        assert_eq!(code.last().copied(), Some(0x9d)); // popfq
        assert!(code.windows(2).any(|bytes| bytes == [0xff, 0xd0]));
    }
}
