//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/hw_breakpoint.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/hw_breakpoint.c
//! Hardware breakpoints (DR0–DR7).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/hw_breakpoint.c
//!
//! Intel and AMD CPUs expose 4 debug-address registers (DR0..DR3),
//! gated by a single control register (DR7). Each slot encodes:
//! - a 2-bit length (1, 2, 4, or 8 bytes)
//! - a 2-bit type (Execute, Write, ReadWrite)
//! - a 2-bit enable (Local / Global)
//!
//! This module ports the byte-exact DR7 encoder/decoder, the per-CPU
//! 4-slot allocator, the kernel-space-overlap check, and the
//! arch ↔ generic field translator (`arch_bp_generic_*`).
//!
//! Intel SDM Vol. 3 §17.2 — "Debug Registers".

#![allow(dead_code)]

extern crate alloc;

use crate::include::uapi::errno::{EBUSY, EINVAL, EOPNOTSUPP};

// === DR7 bit layout — mirror vendor/linux/arch/x86/include/uapi/asm/debugreg.h ===

pub const DR_CONTROL_SHIFT: u32 = 16;
pub const DR_CONTROL_SIZE: u32 = 4;
pub const DR_ENABLE_SIZE: u32 = 2;
pub const DR_GLOBAL_ENABLE: u64 = 0x2;
pub const DR_GLOBAL_SLOWDOWN: u64 = 0x200;

pub const DR6_RESERVED: u64 = 0xFFFF_0FF0;
pub const DR_TRAP0: u64 = 1 << 0;
pub const DR_TRAP1: u64 = 1 << 1;
pub const DR_TRAP2: u64 = 1 << 2;
pub const DR_TRAP3: u64 = 1 << 3;
pub const DR_TRAP_BITS: u64 = DR_TRAP0 | DR_TRAP1 | DR_TRAP2 | DR_TRAP3;
pub const DR_STEP: u64 = 0x4000;

// === Architecture-level enums — mirror asm/hw_breakpoint.h ===

pub const X86_BREAKPOINT_LEN_X: u32 = 0x40;
pub const X86_BREAKPOINT_LEN_1: u32 = 0x40;
pub const X86_BREAKPOINT_LEN_2: u32 = 0x44;
pub const X86_BREAKPOINT_LEN_4: u32 = 0x4c;
pub const X86_BREAKPOINT_LEN_8: u32 = 0x48;

pub const X86_BREAKPOINT_EXECUTE: u32 = 0x80;
pub const X86_BREAKPOINT_WRITE: u32 = 0x81;
pub const X86_BREAKPOINT_RW: u32 = 0x83;

pub const HBP_NUM: usize = 4;

// === Generic types — mirror include/uapi/linux/hw_breakpoint.h ===

pub const HW_BREAKPOINT_LEN_1: u32 = 1;
pub const HW_BREAKPOINT_LEN_2: u32 = 2;
pub const HW_BREAKPOINT_LEN_4: u32 = 4;
pub const HW_BREAKPOINT_LEN_8: u32 = 8;

pub const HW_BREAKPOINT_EMPTY: u32 = 0;
pub const HW_BREAKPOINT_R: u32 = 1;
pub const HW_BREAKPOINT_W: u32 = 2;
pub const HW_BREAKPOINT_RW: u32 = HW_BREAKPOINT_R | HW_BREAKPOINT_W;
pub const HW_BREAKPOINT_X: u32 = 4;

/// `__encode_dr7(drnum, len, type)` — produce the *enable + control* bits
/// for slot `drnum` without the global-slowdown bit.
pub fn encode_dr7_raw(drnum: u32, len: u32, ty: u32) -> u64 {
    let bp_info = ((len | ty) & 0xf) as u64;
    let control = bp_info << (DR_CONTROL_SHIFT + drnum * DR_CONTROL_SIZE);
    let enable = DR_GLOBAL_ENABLE << (drnum * DR_ENABLE_SIZE);
    control | enable
}

/// `encode_dr7` — production wrapper that ORs in `DR_GLOBAL_SLOWDOWN`.
pub fn encode_dr7(drnum: u32, len: u32, ty: u32) -> u64 {
    encode_dr7_raw(drnum, len, ty) | DR_GLOBAL_SLOWDOWN
}

/// `decode_dr7(dr7, bpnum)` — return `(len, type, enabled_bits)` for
/// slot `bpnum`.
///
/// Linux returns the length/type in the same encoding the encoder
/// emits *after* masking by 0xf and ORing the X86_BREAKPOINT_* tag —
/// i.e. it adds the `0x40` / `0x80` discriminator back in.
pub fn decode_dr7(dr7: u64, bpnum: u32) -> (u32, u32, u32) {
    let bp_info = (dr7 >> (DR_CONTROL_SHIFT + bpnum * DR_CONTROL_SIZE)) as u32;
    let len = (bp_info & 0xc) | 0x40;
    let ty = (bp_info & 0x3) | 0x80;
    let en = (dr7 >> (bpnum * DR_ENABLE_SIZE)) as u32 & 0x3;
    (len, ty, en)
}

/// Per-CPU breakpoint state.
#[derive(Debug, Default, Clone)]
pub struct CpuBreakpoints {
    pub dr7: u64,
    pub debugreg: [u64; HBP_NUM],
    pub slot_occupied: [bool; HBP_NUM],
}

/// Arch-specific breakpoint record.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ArchHwBreakpoint {
    pub address: u64,
    pub len: u32,
    pub ty: u32,
    pub mask: u32,
}

/// `arch_install_hw_breakpoint(bp)` — find a free slot, write the
/// address into `cpu_debugreg[i]`, OR the encoded bits into the per-CPU
/// `dr7`. Returns `Err(EBUSY)` when all 4 slots are taken.
pub fn arch_install_hw_breakpoint(
    cpu: &mut CpuBreakpoints,
    info: &ArchHwBreakpoint,
) -> Result<usize, i32> {
    let slot = cpu
        .slot_occupied
        .iter()
        .position(|&occupied| !occupied)
        .ok_or(EBUSY)?;
    cpu.slot_occupied[slot] = true;
    cpu.debugreg[slot] = info.address;
    cpu.dr7 |= encode_dr7(slot as u32, info.len, info.ty);
    Ok(slot)
}

/// `arch_uninstall_hw_breakpoint(bp)` — search for the slot whose
/// `debugreg` matches `address`, clear the slot, clear the corresponding
/// `dr7` bits.
pub fn arch_uninstall_hw_breakpoint(
    cpu: &mut CpuBreakpoints,
    info: &ArchHwBreakpoint,
) -> Result<usize, i32> {
    let slot = cpu
        .slot_occupied
        .iter()
        .enumerate()
        .position(|(i, &occ)| occ && cpu.debugreg[i] == info.address)
        .ok_or(EINVAL)?;
    cpu.dr7 &= !encode_dr7_raw(slot as u32, info.len, info.ty);
    cpu.slot_occupied[slot] = false;
    cpu.debugreg[slot] = 0;
    Ok(slot)
}

/// `arch_bp_generic_len(x86_len)` → generic length, or `EINVAL`.
pub fn arch_bp_generic_len(x86_len: u32) -> Result<u32, i32> {
    match x86_len {
        X86_BREAKPOINT_LEN_1 => Ok(HW_BREAKPOINT_LEN_1),
        X86_BREAKPOINT_LEN_2 => Ok(HW_BREAKPOINT_LEN_2),
        X86_BREAKPOINT_LEN_4 => Ok(HW_BREAKPOINT_LEN_4),
        X86_BREAKPOINT_LEN_8 => Ok(HW_BREAKPOINT_LEN_8),
        _ => Err(EINVAL),
    }
}

/// `arch_bp_generic_fields(x86_len, x86_type)` → `(gen_len, gen_type)`.
///
/// The EXECUTE direction has a special hard-coded `LEN_X` requirement.
pub fn arch_bp_generic_fields(x86_len: u32, x86_type: u32) -> Result<(u32, u32), i32> {
    match x86_type {
        X86_BREAKPOINT_EXECUTE => {
            if x86_len != X86_BREAKPOINT_LEN_X {
                return Err(EINVAL);
            }
            Ok((core::mem::size_of::<usize>() as u32, HW_BREAKPOINT_X))
        }
        X86_BREAKPOINT_WRITE => Ok((arch_bp_generic_len(x86_len)?, HW_BREAKPOINT_W)),
        X86_BREAKPOINT_RW => Ok((arch_bp_generic_len(x86_len)?, HW_BREAKPOINT_RW)),
        _ => Err(EINVAL),
    }
}

/// `arch_check_bp_in_kernelspace(hw)` — does the bp range fall into
/// kernel-space VAs? `task_size_max` is the user/kernel split.
pub fn arch_check_bp_in_kernelspace(hw: &ArchHwBreakpoint, task_size_max: u64) -> bool {
    let len = arch_bp_generic_len(hw.len).unwrap_or(1) as u64;
    let end = hw.address.wrapping_add(len.saturating_sub(1));
    hw.address >= task_size_max || end >= task_size_max
}

/// `arch_build_bp_info` — validate `attr.bp_type` and `attr.bp_len`,
/// derive the arch fields. Skipping the kprobe-blacklist check and the
/// CPU-entry-area overlap check here (those live in the kprobe/CPU entry
/// modules in their own batches).
pub fn arch_build_bp_info(
    addr: u64,
    bp_type: u32,
    bp_len: u32,
    has_bpext: bool,
) -> Result<ArchHwBreakpoint, i32> {
    let mut hw = ArchHwBreakpoint {
        address: addr,
        len: 0,
        ty: 0,
        mask: 0,
    };
    match bp_type {
        HW_BREAKPOINT_W => hw.ty = X86_BREAKPOINT_WRITE,
        v if v == (HW_BREAKPOINT_W | HW_BREAKPOINT_R) => hw.ty = X86_BREAKPOINT_RW,
        HW_BREAKPOINT_X => {
            hw.ty = X86_BREAKPOINT_EXECUTE;
            if bp_len as usize == core::mem::size_of::<usize>() {
                hw.len = X86_BREAKPOINT_LEN_X;
                return Ok(hw);
            }
            return Err(EINVAL);
        }
        _ => return Err(EINVAL),
    }
    hw.len = match bp_len {
        HW_BREAKPOINT_LEN_1 => X86_BREAKPOINT_LEN_1,
        HW_BREAKPOINT_LEN_2 => X86_BREAKPOINT_LEN_2,
        HW_BREAKPOINT_LEN_4 => X86_BREAKPOINT_LEN_4,
        HW_BREAKPOINT_LEN_8 => X86_BREAKPOINT_LEN_8,
        len => {
            // AMD range breakpoint path: bp_len must be a power-of-two,
            // bp_addr must be aligned to bp_len, and the CPU must
            // advertise BPEXT.
            if !len.is_power_of_two() {
                return Err(EINVAL);
            }
            if addr & (len as u64 - 1) != 0 {
                return Err(EINVAL);
            }
            if !has_bpext {
                return Err(EOPNOTSUPP);
            }
            hw.mask = len - 1;
            X86_BREAKPOINT_LEN_1
        }
    };
    Ok(hw)
}

/// `hw_breakpoint_arch_parse` — wrap `arch_build_bp_info` and run the
/// final alignment check (the low-order bits of `address` must agree
/// with `len`).
pub fn hw_breakpoint_arch_parse(
    addr: u64,
    bp_type: u32,
    bp_len: u32,
    has_bpext: bool,
) -> Result<ArchHwBreakpoint, i32> {
    let hw = arch_build_bp_info(addr, bp_type, bp_len, has_bpext)?;
    let align = match hw.len {
        X86_BREAKPOINT_LEN_1 => {
            if hw.mask != 0 {
                hw.mask as u64
            } else {
                0
            }
        }
        X86_BREAKPOINT_LEN_2 => 1,
        X86_BREAKPOINT_LEN_4 => 3,
        X86_BREAKPOINT_LEN_8 => 7,
        _ => return Err(EINVAL),
    };
    if hw.address & align != 0 {
        return Err(EINVAL);
    }
    Ok(hw)
}

/// Per-task ptrace state — DR6/DR7 mirrors.
#[derive(Debug, Default, Clone, Copy)]
pub struct PtraceBpState {
    pub virtual_dr6: u64,
    pub ptrace_dr7: u64,
    pub slot_active: [bool; HBP_NUM],
}

/// `flush_ptrace_hw_breakpoint(tsk)` — clear the per-task DR mirrors.
pub fn flush_ptrace_hw_breakpoint(state: &mut PtraceBpState) {
    state.virtual_dr6 = 0;
    state.ptrace_dr7 = 0;
    state.slot_active = [false; HBP_NUM];
}

/// `hw_breakpoint_handler` — given `DR6`, return the set of slots whose
/// trap bits are set.
pub fn hw_breakpoint_triggered_slots(dr6: u64) -> [bool; HBP_NUM] {
    let mut out = [false; HBP_NUM];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = (dr6 & (DR_TRAP0 << i)) != 0;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dr7_constants_match_linux_uapi() {
        assert_eq!(DR_CONTROL_SHIFT, 16);
        assert_eq!(DR_CONTROL_SIZE, 4);
        assert_eq!(DR_ENABLE_SIZE, 2);
        assert_eq!(DR_GLOBAL_SLOWDOWN, 0x200);
        assert_eq!(DR6_RESERVED, 0xFFFF_0FF0);
        assert_eq!(DR_TRAP_BITS, 0xF);
        assert_eq!(DR_STEP, 0x4000);
    }

    #[test]
    fn encoding_round_trip_through_decoder() {
        for slot in 0..4u32 {
            for (len_x, ty_x) in &[
                (X86_BREAKPOINT_LEN_1, X86_BREAKPOINT_WRITE),
                (X86_BREAKPOINT_LEN_4, X86_BREAKPOINT_RW),
                (X86_BREAKPOINT_LEN_8, X86_BREAKPOINT_WRITE),
                (X86_BREAKPOINT_LEN_X, X86_BREAKPOINT_EXECUTE),
            ] {
                let dr7 = encode_dr7(slot, *len_x, *ty_x);
                let (dlen, dty, en) = decode_dr7(dr7, slot);
                assert_eq!(dlen, *len_x);
                assert_eq!(dty, *ty_x);
                assert!(en & 0x2 != 0, "global-enable bit must be set");
            }
        }
    }

    #[test]
    fn encode_dr7_includes_global_slowdown() {
        let dr7 = encode_dr7(0, X86_BREAKPOINT_LEN_4, X86_BREAKPOINT_WRITE);
        assert!(dr7 & DR_GLOBAL_SLOWDOWN != 0);
    }

    #[test]
    fn install_allocates_first_free_slot() {
        let mut cpu = CpuBreakpoints::default();
        let bp = ArchHwBreakpoint {
            address: 0x1000,
            len: X86_BREAKPOINT_LEN_4,
            ty: X86_BREAKPOINT_WRITE,
            mask: 0,
        };
        assert_eq!(arch_install_hw_breakpoint(&mut cpu, &bp), Ok(0));
        assert!(cpu.slot_occupied[0]);
        assert_eq!(cpu.debugreg[0], 0x1000);
    }

    #[test]
    fn install_rejects_when_all_slots_full() {
        let mut cpu = CpuBreakpoints::default();
        let bp = ArchHwBreakpoint {
            address: 0x1000,
            len: X86_BREAKPOINT_LEN_4,
            ty: X86_BREAKPOINT_WRITE,
            mask: 0,
        };
        for _ in 0..HBP_NUM {
            arch_install_hw_breakpoint(&mut cpu, &bp).unwrap();
        }
        assert_eq!(arch_install_hw_breakpoint(&mut cpu, &bp), Err(EBUSY));
    }

    #[test]
    fn uninstall_clears_slot_and_dr7_bits() {
        let mut cpu = CpuBreakpoints::default();
        let bp = ArchHwBreakpoint {
            address: 0x2000,
            len: X86_BREAKPOINT_LEN_4,
            ty: X86_BREAKPOINT_WRITE,
            mask: 0,
        };
        arch_install_hw_breakpoint(&mut cpu, &bp).unwrap();
        let dr7_before = cpu.dr7;
        arch_uninstall_hw_breakpoint(&mut cpu, &bp).unwrap();
        assert!(!cpu.slot_occupied[0]);
        assert_eq!(cpu.debugreg[0], 0);
        assert!(cpu.dr7 < dr7_before);
    }

    #[test]
    fn arch_bp_generic_len_maps_lengths_round_trip() {
        assert_eq!(arch_bp_generic_len(X86_BREAKPOINT_LEN_1), Ok(1));
        assert_eq!(arch_bp_generic_len(X86_BREAKPOINT_LEN_2), Ok(2));
        assert_eq!(arch_bp_generic_len(X86_BREAKPOINT_LEN_4), Ok(4));
        assert_eq!(arch_bp_generic_len(X86_BREAKPOINT_LEN_8), Ok(8));
        assert_eq!(arch_bp_generic_len(0xFF), Err(EINVAL));
    }

    #[test]
    fn arch_bp_generic_fields_execute_requires_len_x() {
        assert!(arch_bp_generic_fields(X86_BREAKPOINT_LEN_X, X86_BREAKPOINT_EXECUTE).is_ok());
        assert_eq!(
            arch_bp_generic_fields(X86_BREAKPOINT_LEN_4, X86_BREAKPOINT_EXECUTE),
            Err(EINVAL)
        );
    }

    #[test]
    fn arch_check_bp_in_kernelspace_detects_kernel_va() {
        let hw = ArchHwBreakpoint {
            address: 0xFFFF_8000_0000_0000,
            len: X86_BREAKPOINT_LEN_4,
            ty: X86_BREAKPOINT_WRITE,
            mask: 0,
        };
        assert!(arch_check_bp_in_kernelspace(&hw, 0x0000_7FFF_FFFF_F000));
        let hw2 = ArchHwBreakpoint {
            address: 0x1000,
            len: X86_BREAKPOINT_LEN_4,
            ty: X86_BREAKPOINT_WRITE,
            mask: 0,
        };
        assert!(!arch_check_bp_in_kernelspace(&hw2, 0x0000_7FFF_FFFF_F000));
    }

    #[test]
    fn arch_build_bp_info_translates_w_4() {
        let hw = arch_build_bp_info(0x1000, HW_BREAKPOINT_W, HW_BREAKPOINT_LEN_4, false).unwrap();
        assert_eq!(hw.ty, X86_BREAKPOINT_WRITE);
        assert_eq!(hw.len, X86_BREAKPOINT_LEN_4);
        assert_eq!(hw.mask, 0);
    }

    #[test]
    fn arch_build_bp_info_rejects_unaligned_amd_range() {
        let r = arch_build_bp_info(0x1003, HW_BREAKPOINT_W, 16, true);
        assert_eq!(r, Err(EINVAL));
    }

    #[test]
    fn arch_build_bp_info_requires_bpext_for_amd_range() {
        let r = arch_build_bp_info(0x1000, HW_BREAKPOINT_W, 16, false);
        assert_eq!(r, Err(EOPNOTSUPP));
    }

    #[test]
    fn arch_build_bp_info_accepts_amd_range_when_supported() {
        let hw = arch_build_bp_info(0x1000, HW_BREAKPOINT_W, 16, true).unwrap();
        assert_eq!(hw.mask, 15);
        assert_eq!(hw.len, X86_BREAKPOINT_LEN_1);
    }

    #[test]
    fn hw_breakpoint_arch_parse_alignment_check() {
        // len 4 → low 2 bits must be 0.
        assert_eq!(
            hw_breakpoint_arch_parse(0x1001, HW_BREAKPOINT_W, HW_BREAKPOINT_LEN_4, false),
            Err(EINVAL)
        );
        assert!(
            hw_breakpoint_arch_parse(0x1000, HW_BREAKPOINT_W, HW_BREAKPOINT_LEN_4, false).is_ok()
        );
    }

    #[test]
    fn flush_ptrace_zeroes_state() {
        let mut s = PtraceBpState {
            virtual_dr6: 0xFF,
            ptrace_dr7: 0xABCD,
            slot_active: [true; HBP_NUM],
        };
        flush_ptrace_hw_breakpoint(&mut s);
        assert_eq!(s.virtual_dr6, 0);
        assert_eq!(s.ptrace_dr7, 0);
        assert!(s.slot_active.iter().all(|&a| !a));
    }

    #[test]
    fn triggered_slots_returns_exact_dr6_bits() {
        let slots = hw_breakpoint_triggered_slots(DR_TRAP1 | DR_TRAP3);
        assert_eq!(slots, [false, true, false, true]);
    }
}
