//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/lib/insn-eval.c
//! test-origin: linux:vendor/linux/arch/x86/lib/insn-eval.c
//! Operand- and effective-address evaluation for decoded x86 instructions.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/insn-eval.c
//! - vendor/linux/arch/x86/include/asm/insn-eval.h
//!
//! `insn-eval.c` answers two questions about a decoded instruction:
//!   1. Which segment register provides the base for this memory access?
//!   2. What is the linear address the instruction references?
//!
//! Used by the page-fault handler (UMIP emulation, SGX EENTER fault
//! recovery, vsyscall emulation), kprobes, the SEV-ES NPF handler, and
//! the protection-key fault diagnostics. This port covers the call
//! shapes used by the page-fault path — string-insn detection, REP
//! prefix detection, segment override index extraction, and ModRM/SIB
//! effective address evaluation in 64-bit mode.

use super::inat::*;
use super::insn::Insn;

/// Register-class enum mirroring `enum reg_type` in insn-eval.c.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RegType {
    Rm = 0,
    Reg,
    Index,
    Base,
}

/// `is_string_insn(insn)` — true for INS/OUTS/MOVS/CMPS/STOS/LODS/SCAS.
/// Mirrors insn-eval.c lines 39-53.
pub fn is_string_insn(insn: &Insn) -> bool {
    if insn.opcode.nbytes != 1 {
        return false;
    }
    let op = insn.opcode.value as u8;
    matches!(op, 0x6c..=0x6f | 0xa4..=0xa7 | 0xaa..=0xaf)
}

/// `insn_has_rep_prefix(insn)` — true if the instruction carries a
/// REPNE (0xF2) or REPE (0xF3) prefix. Mirrors lines 63-75.
///
/// Linux walks all four legacy prefix slots; the bootstrap decoder
/// records only the *last* legacy prefix in `prefixes.value`, so the
/// equivalent check is a single comparison.
pub fn insn_has_rep_prefix(insn: &Insn) -> bool {
    matches!(insn.prefixes.value as u8, 0xF2 | 0xF3)
}

/// `get_seg_reg_override_idx(insn)` — return the segment-override
/// index (one of INAT_SEG_REG_*) corresponding to any segment
/// prefix on the instruction. Returns INAT_SEG_REG_DEFAULT when no
/// override is present. Mirrors lines 77-110.
pub fn get_seg_reg_override_idx(insn: &Insn) -> u8 {
    match insn.prefixes.value as u8 {
        0x2E => INAT_SEG_REG_CS,
        0x36 => INAT_SEG_REG_SS,
        0x3E => INAT_SEG_REG_DS,
        0x26 => INAT_SEG_REG_ES,
        0x64 => INAT_SEG_REG_FS,
        0x65 => INAT_SEG_REG_GS,
        _ => INAT_SEG_REG_DEFAULT,
    }
}

/// Mirror of `struct pt_regs` slice used by insn-eval — caller supplies
/// the 16 GPRs in Linux's `pt_regs` ordering. Indices 0-15 correspond
/// to RAX, RBX, RCX, RDX, RSI, RDI, RBP, RSP, R8..R15 (the order is
/// not the same as Linux's struct pt_regs in memory; this is the GPR
/// number ordering used by the ModRM encoding, which is what the
/// effective-address code actually needs).
#[derive(Default, Copy, Clone, Debug)]
pub struct Gprs {
    pub gpr: [u64; 16],
}

impl Gprs {
    /// GPR ordering used by ModRM/SIB encoding:
    ///   0=RAX, 1=RCX, 2=RDX, 3=RBX, 4=RSP, 5=RBP, 6=RSI, 7=RDI,
    ///   8=R8 ... 15=R15
    pub const RAX: usize = 0;
    pub const RCX: usize = 1;
    pub const RDX: usize = 2;
    pub const RBX: usize = 3;
    pub const RSP: usize = 4;
    pub const RBP: usize = 5;
    pub const RSI: usize = 6;
    pub const RDI: usize = 7;
}

/// `get_eff_addr_modrm_64(insn, regs)` — compute the linear (virtual)
/// address an instruction reads/writes, based on its ModRM byte. Only
/// applicable when `mod` ∈ {0,1,2}; `mod=3` denotes a register operand
/// and yields `None`. Mirrors `get_eff_addr_modrm` (insn-eval.c) for
/// the 64-bit subset.
///
/// Caller is responsible for adding the segment base when the
/// override resolves to a non-default segment.
pub fn get_eff_addr_modrm_64(insn: &Insn, regs: &Gprs) -> Option<u64> {
    if insn.modrm.got == 0 {
        return None;
    }
    let modrm = insn.modrm.value as u8;
    let r#mod = (modrm >> 6) & 0x3;
    let mut rm = (modrm & 0x7) as usize;

    if r#mod == 3 {
        // Register operand — no memory address.
        return None;
    }

    // REX.B extends the R/M field for 64-bit GPRs (registers 8..15).
    let rex_b = (insn.rex_prefix.value & 0x1) != 0;
    if rex_b {
        rm += 8;
    }

    // SIB-encoded address: mod != 3 and rm (low 3 bits, ignoring REX.B
    // when checking the encoding switch) == 4.
    if (modrm & 0x7) == 4 {
        return get_eff_addr_sib_64(insn, regs);
    }

    // Mod=00, R/M=5 → RIP-relative. The decoder's `displacement`
    // already captured the disp32; insn-eval.c lets the caller resolve
    // RIP because the eval code has no IP. We mirror that.
    if r#mod == 0 && (modrm & 0x7) == 5 {
        // RIP-relative — caller must add `regs.rip` (not present in
        // our `Gprs`). Return the displacement alone; caller adds RIP.
        return Some(insn.displacement.value as i32 as i64 as u64);
    }

    let base = regs.gpr.get(rm).copied()?;
    let disp = match r#mod {
        0 => 0i64,
        1 => insn.displacement.value as i8 as i64,
        2 => insn.displacement.value as i32 as i64,
        _ => unreachable!(),
    };
    Some(base.wrapping_add(disp as u64))
}

/// `get_eff_addr_sib_64()` — SIB-encoded address. Mirrors the SIB
/// branch of insn-eval.c::get_eff_addr_modrm.
pub fn get_eff_addr_sib_64(insn: &Insn, regs: &Gprs) -> Option<u64> {
    if insn.sib.got == 0 {
        return None;
    }
    let sib = insn.sib.value as u8;
    let modrm = insn.modrm.value as u8;
    let r#mod = (modrm >> 6) & 0x3;
    let scale = (sib >> 6) & 0x3;
    let mut index = ((sib >> 3) & 0x7) as usize;
    let mut base = (sib & 0x7) as usize;

    let rex_x = (insn.rex_prefix.value & 0x2) != 0;
    let rex_b = (insn.rex_prefix.value & 0x1) != 0;
    if rex_x {
        index += 8;
    }
    if rex_b {
        base += 8;
    }

    // index=4 means "no index" (RSP is not encodable as an index — that
    // slot signals the absence of an index register).
    let idx_val: u64 = if (sib >> 3) & 0x7 == 4 {
        0
    } else {
        regs.gpr.get(index).copied()?.wrapping_shl(scale as u32)
    };

    // base=5 with mod=0 is "no base, disp32".
    let base_val: u64 = if r#mod == 0 && (sib & 0x7) == 5 {
        0
    } else {
        regs.gpr.get(base).copied()?
    };

    let disp: i64 = match r#mod {
        0 if (sib & 0x7) == 5 => insn.displacement.value as i32 as i64,
        0 => 0,
        1 => insn.displacement.value as i8 as i64,
        2 => insn.displacement.value as i32 as i64,
        _ => 0,
    };

    Some(base_val.wrapping_add(idx_val).wrapping_add(disp as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_insn_recognises_movs_stos_etc() {
        let mut i = Insn::init(&[0xA4], true); // MOVSB
        i.get_length();
        assert!(is_string_insn(&i));
        let mut i = Insn::init(&[0x6c], true); // INSB
        i.get_length();
        assert!(is_string_insn(&i));
        let mut i = Insn::init(&[0xAF], true); // SCASD
        i.get_length();
        assert!(is_string_insn(&i));
        let mut i = Insn::init(&[0x90], true); // NOP — not a string insn
        i.get_length();
        assert!(!is_string_insn(&i));
    }

    #[test]
    fn rep_prefix_detection_handles_both_f2_and_f3() {
        let mut i = Insn::init(&[0xF3, 0xA4], true); // REP MOVSB
        i.get_length();
        assert!(insn_has_rep_prefix(&i));
        let mut i = Insn::init(&[0xF2, 0xAE], true); // REPNE SCASB
        i.get_length();
        assert!(insn_has_rep_prefix(&i));
        let mut i = Insn::init(&[0xA4], true); // No prefix
        i.get_length();
        assert!(!insn_has_rep_prefix(&i));
    }

    #[test]
    fn seg_reg_override_returns_correct_index_per_prefix() {
        let pairs: &[(u8, u8)] = &[
            (0x2E, INAT_SEG_REG_CS),
            (0x36, INAT_SEG_REG_SS),
            (0x3E, INAT_SEG_REG_DS),
            (0x26, INAT_SEG_REG_ES),
            (0x64, INAT_SEG_REG_FS),
            (0x65, INAT_SEG_REG_GS),
        ];
        for (prefix, expected) in pairs.iter() {
            let mut i = Insn::init(&[*prefix, 0x89, 0xD8], true);
            i.get_length();
            assert_eq!(get_seg_reg_override_idx(&i), *expected);
        }
        // No prefix → default.
        let mut i = Insn::init(&[0x89, 0xD8], true);
        i.get_length();
        assert_eq!(get_seg_reg_override_idx(&i), INAT_SEG_REG_DEFAULT);
    }

    #[test]
    fn eff_addr_modrm_64_disp32_via_rip_relative() {
        // 8B 05 11 22 33 44 — MOV eax, [rip+0x44332211].
        let mut i = Insn::init(&[0x8B, 0x05, 0x11, 0x22, 0x33, 0x44], true);
        i.get_length();
        let regs = Gprs::default();
        // RIP-relative: caller adds RIP; the helper returns the disp32.
        let addr = get_eff_addr_modrm_64(&i, &regs).expect("disp32 RIP-rel");
        assert_eq!(addr, 0x4433_2211);
    }

    #[test]
    fn eff_addr_modrm_64_register_indirect() {
        // 8B 00 — MOV eax, [rax]. With rax = 0xCAFEBABE.
        let mut i = Insn::init(&[0x8B, 0x00], true);
        i.get_length();
        let mut regs = Gprs::default();
        regs.gpr[Gprs::RAX] = 0xCAFEBABE;
        assert_eq!(get_eff_addr_modrm_64(&i, &regs), Some(0xCAFEBABE));
    }

    #[test]
    fn eff_addr_modrm_64_register_with_disp8() {
        // 8B 41 10 — MOV eax, [rcx+0x10]. rcx = 0x1000.
        let mut i = Insn::init(&[0x8B, 0x41, 0x10], true);
        i.get_length();
        let mut regs = Gprs::default();
        regs.gpr[Gprs::RCX] = 0x1000;
        assert_eq!(get_eff_addr_modrm_64(&i, &regs), Some(0x1010));
    }

    #[test]
    fn eff_addr_modrm_64_returns_none_for_register_operand() {
        // 89 D8 — MOV eax, ebx. Mod=11 → register operand, no memory.
        let mut i = Insn::init(&[0x89, 0xD8], true);
        i.get_length();
        let regs = Gprs::default();
        assert_eq!(get_eff_addr_modrm_64(&i, &regs), None);
    }
}
