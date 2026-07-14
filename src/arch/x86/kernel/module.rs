//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/module.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/module.c
//! x86 module relocation and finalization audit helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/module.c
//!
//! The relocation writer delegates to the runtime loader's bounded x86_64
//! implementation. `module_finalize()` mirrors the vendor section scan and
//! finalizes `.smp_locks` for Lupos' current SMP text state. The runtime loader
//! still rejects ITS/FineIBT, return thunk, call thunk, alternatives, IBT
//! sealing, and ORC metadata which need unimplemented patching or registration.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::arch::x86::kernel::alternative::{
    ALT_FLAG_DIRECT_CALL, ALT_FLAGS_SHIFT, AltInstr, CALL_INSN_OPCODE, JMP32_INSN_OPCODE,
    MAX_PATCH_LEN, add_nops, alternatives_smp_module_add, alternatives_smp_module_del,
    text_poke_copy,
};
use crate::arch::x86::kernel::cpu::common::{boot_cpu_has, x86_feature_limit};
use crate::arch::x86::kernel::jump_label::JMP8_INSN_OPCODE;
use crate::arch::x86::kernel::static_call::{
    STATIC_CALL_SITE_FLAGS, STATIC_CALL_SITE_SIZE, STATIC_CALL_SITE_TAIL,
    static_call_fixup_warn_site, warn_trap_addr, warn_trap_trampoline_addr,
};
use crate::arch::x86::lib::insn::Insn;
use crate::kernel::module::loader::{LoadedSection, NameMap};
use crate::kernel::module::relocate::{Rela, RelocType, apply_rela};

#[derive(Debug, Default, Eq, PartialEq)]
pub struct X86ModuleMetadata {
    pub has_jump_entries: bool,
    pub has_orc_unwind: bool,
    pub has_alternatives: bool,
    pub has_smp_locks: bool,
    /// True only after `.altinstructions` has actually been patched.  Merely
    /// finding the section is not equivalent to Linux `apply_alternatives()`.
    pub alternatives_applied: bool,
    /// Membership in Linux's `smp_alt_modules` list. This remains false while
    /// Lupos keeps the SMP-safe lock prefixes and never enters the globally
    /// UP-patched state.
    smp_locks_registered: bool,
    pub num_static_call_sites: usize,
    pub num_extable_entries: usize,
}

impl Drop for X86ModuleMetadata {
    fn drop(&mut self) {
        alternatives_smp_module_del(self.smp_locks_registered);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedRela {
    pub rela: Rela,
    pub sym_addr: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub enum X86ModuleFinalizeError {
    BadSection(&'static str),
    Unsupported(&'static str),
}

pub fn apply_relocate_add(
    mem: &mut [u8],
    section_addr: u64,
    relocs: &[ResolvedRela],
) -> Result<(), i32> {
    for r in relocs {
        let patch_addr = section_addr
            .checked_add(r.rela.offset)
            .ok_or(crate::include::uapi::errno::ENOEXEC)?;
        apply_rela(
            mem,
            r.rela.offset as usize,
            r.rela.rel_type,
            r.sym_addr,
            patch_addr,
            r.rela.addend,
        )?;
    }
    Ok(())
}

pub fn clear_relocate_add(mem: &mut [u8], relocs: &[ResolvedRela]) {
    for r in relocs {
        let width = match r.rela.rel_type {
            RelocType::Abs64 | RelocType::Pc64 => 8,
            RelocType::None => 0,
            _ => 4,
        };
        let off = r.rela.offset as usize;
        if width != 0 && off + width <= mem.len() {
            mem[off..off + width].fill(0);
        }
    }
}

pub fn module_finalize(
    sections: &mut NameMap<LoadedSection>,
) -> Result<X86ModuleMetadata, X86ModuleFinalizeError> {
    let has_smp_locks = sections.contains_key(".smp_locks");
    let alternatives_applied = apply_module_alternatives(sections)?;
    let num_static_call_sites = finalize_static_call_sites(sections)?;
    let num_extable_entries = sort_module_extable(sections)?;
    Ok(X86ModuleMetadata {
        has_jump_entries: sections.contains_key("__jump_table"),
        has_orc_unwind: sections.contains_key(".orc_unwind"),
        has_alternatives: sections.contains_key(".altinstructions"),
        has_smp_locks,
        alternatives_applied,
        smp_locks_registered: has_smp_locks && alternatives_smp_module_add(),
        num_static_call_sites,
        num_extable_entries,
    })
}

pub fn module_arch_cleanup(metadata: &mut X86ModuleMetadata) {
    *metadata = X86ModuleMetadata::default();
}

pub fn decode_rela_entries(data: &[u8]) -> Vec<Rela> {
    let mut out = Vec::new();
    let mut off = 0;
    while let Some(rela) = Rela::from_bytes(data, off) {
        out.push(rela);
        off += 24;
    }
    out
}

fn read_i32(data: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn relative_addr(field_addr: usize, displacement: i32) -> usize {
    (field_addr as isize).wrapping_add(displacement as isize) as usize
}

fn loaded_bytes_at(sections: &NameMap<LoadedSection>, addr: usize, size: usize) -> Option<Vec<u8>> {
    for section in sections.values() {
        let base = section.as_ptr() as usize;
        let Some(offset) = addr.checked_sub(base) else {
            continue;
        };
        let Some(end) = offset.checked_add(size) else {
            continue;
        };
        if end <= section.len() {
            return Some(section.as_slice()[offset..end].to_vec());
        }
    }
    None
}

fn loaded_mut_at(
    sections: &mut NameMap<LoadedSection>,
    addr: usize,
    size: usize,
) -> Option<&mut [u8]> {
    for section in sections.values_mut() {
        let base = section.as_ptr() as usize;
        let Some(offset) = addr.checked_sub(base) else {
            continue;
        };
        let Some(end) = offset.checked_add(size) else {
            continue;
        };
        if end <= section.len() {
            return Some(&mut section.as_mut_slice()[offset..end]);
        }
    }
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ModuleAltEntry {
    instr_addr: usize,
    repl_addr: usize,
    cpuid: u16,
    flags: u16,
    instrlen: u8,
    replacementlen: u8,
}

fn parse_alt_entries(
    sections: &NameMap<LoadedSection>,
) -> Result<Vec<ModuleAltEntry>, X86ModuleFinalizeError> {
    let Some(section) = sections.get(".altinstructions") else {
        return Ok(Vec::new());
    };
    if section.len() % 14 != 0 {
        return Err(X86ModuleFinalizeError::BadSection(".altinstructions"));
    }
    let base = section.as_ptr() as usize;
    let data = section.as_slice();
    let mut entries = Vec::with_capacity(section.len() / 14);
    for offset in (0..section.len()).step_by(14) {
        let entry_addr = base + offset;
        let instr_off =
            read_i32(data, offset).ok_or(X86ModuleFinalizeError::BadSection(".altinstructions"))?;
        let repl_off = read_i32(data, offset + 4)
            .ok_or(X86ModuleFinalizeError::BadSection(".altinstructions"))?;
        let ft_flags = read_u32(data, offset + 8)
            .ok_or(X86ModuleFinalizeError::BadSection(".altinstructions"))?;
        entries.push(ModuleAltEntry {
            instr_addr: relative_addr(entry_addr, instr_off),
            repl_addr: relative_addr(entry_addr + 4, repl_off),
            cpuid: (ft_flags & 0xffff) as u16,
            flags: (ft_flags >> ALT_FLAGS_SHIFT) as u16,
            instrlen: data[offset + 12],
            replacementlen: data[offset + 13],
        });
    }
    Ok(entries)
}

fn apply_module_alternatives(
    sections: &mut NameMap<LoadedSection>,
) -> Result<bool, X86ModuleFinalizeError> {
    let entries = parse_alt_entries(sections)?;
    let mut applied = false;
    let mut index = 0usize;
    while index < entries.len() {
        let instr_addr = entries[index].instr_addr;
        let mut patch_len = 0usize;
        let mut selected = None;
        while index < entries.len() && entries[index].instr_addr == instr_addr {
            let entry = entries[index];
            if entry.cpuid as u32 >= x86_feature_limit() {
                return Err(X86ModuleFinalizeError::BadSection(".altinstructions"));
            }
            patch_len = patch_len.max(entry.instrlen as usize);
            let alt = AltInstr {
                cpuid: entry.cpuid,
                instrlen: entry.instrlen,
                replacementlen: entry.replacementlen,
                flags: entry.flags,
            };
            if alt.should_patch(boot_cpu_has(entry.cpuid as u32)) {
                selected = Some(entry);
            }
            index += 1;
        }

        if patch_len > MAX_PATCH_LEN {
            return Err(X86ModuleFinalizeError::BadSection(".altinstructions"));
        }
        let Some(entry) = selected else {
            continue;
        };
        if entry.flags & ALT_FLAG_DIRECT_CALL != 0 {
            return Err(X86ModuleFinalizeError::Unsupported(
                ".altinstructions ALT_FLAG_DIRECT_CALL",
            ));
        }
        let repl_len = entry.replacementlen as usize;
        if repl_len > patch_len {
            return Err(X86ModuleFinalizeError::BadSection(".altinstructions"));
        }
        let replacement = loaded_bytes_at(sections, entry.repl_addr, repl_len)
            .ok_or(X86ModuleFinalizeError::BadSection(".altinstructions"))?;
        let mut patch = alloc::vec![0x90u8; patch_len];
        patch[..repl_len].copy_from_slice(&replacement);
        add_nops(&mut patch[repl_len..]);
        apply_alt_relocation(&mut patch, instr_addr, entry.repl_addr, repl_len)?;
        let dst = loaded_mut_at(sections, instr_addr, patch_len)
            .ok_or(X86ModuleFinalizeError::BadSection(".altinstructions"))?;
        text_poke_copy(dst, &patch)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".altinstructions"))?;
        applied = true;
    }
    Ok(applied)
}

fn sign_extend(value: u32, nbytes: u8) -> i64 {
    match nbytes {
        1 => value as u8 as i8 as i64,
        2 => value as u16 as i16 as i64,
        4 => value as i32 as i64,
        _ => value as i64,
    }
}

fn write_signed(bytes: &mut [u8], offset: usize, nbytes: u8, value: i64) -> Result<(), i32> {
    match nbytes {
        1 if (i8::MIN as i64..=i8::MAX as i64).contains(&value) => {
            bytes[offset] = value as i8 as u8;
            Ok(())
        }
        2 if (i16::MIN as i64..=i16::MAX as i64).contains(&value) => {
            bytes[offset..offset + 2].copy_from_slice(&(value as i16).to_le_bytes());
            Ok(())
        }
        4 if (i32::MIN as i64..=i32::MAX as i64).contains(&value) => {
            bytes[offset..offset + 4].copy_from_slice(&(value as i32).to_le_bytes());
            Ok(())
        }
        _ => Err(crate::include::uapi::errno::EINVAL),
    }
}

fn need_reloc(target_offset: i64, repl_len: usize) -> bool {
    target_offset < 0 || target_offset > repl_len as i64
}

fn is_rip_relative(insn: &Insn) -> bool {
    if insn.modrm.got == 0 || insn.displacement.nbytes == 0 {
        return false;
    }
    let modrm = insn.modrm.value as u8;
    let mode = (modrm >> 6) & 0x3;
    let rm = modrm & 0x7;
    mode == 0 && (rm == 5 || (rm == 4 && insn.sib.got != 0 && (insn.sib.value as u8 & 0x7) == 5))
}

fn apply_alt_relocation(
    patch: &mut [u8],
    instr_addr: usize,
    repl_addr: usize,
    repl_len: usize,
) -> Result<(), X86ModuleFinalizeError> {
    let diff = (repl_addr as isize).wrapping_sub(instr_addr as isize) as i64;
    let mut offset = 0usize;
    while offset < patch.len() {
        let mut insn = Insn::init(&patch[offset..], true);
        let len = insn.get_length() as usize;
        if len == 0 || offset + len > patch.len() {
            return Err(X86ModuleFinalizeError::BadSection(".altinstructions"));
        }

        let opcode_offset =
            offset + insn.prefixes.nbytes as usize + insn.rex_prefix.nbytes as usize;
        let opcode0 = patch[opcode_offset];
        let opcode1 = patch.get(opcode_offset + 1).copied().unwrap_or(0);
        let is_branch = matches!(
            opcode0,
            CALL_INSN_OPCODE | JMP32_INSN_OPCODE | JMP8_INSN_OPCODE | 0x70..=0x7f
        ) || (opcode0 == 0x0f && (0x80..=0x8f).contains(&opcode1));

        if is_branch && insn.immediate.nbytes != 0 {
            let imm = sign_extend(insn.immediate.value, insn.immediate.nbytes);
            let next = offset as i64 + len as i64;
            if need_reloc(next + imm, repl_len) {
                let imm_offset = offset
                    + insn.prefixes.nbytes as usize
                    + insn.rex_prefix.nbytes as usize
                    + insn.opcode.nbytes as usize
                    + insn.modrm.nbytes as usize
                    + insn.sib.nbytes as usize
                    + insn.displacement.nbytes as usize;
                write_signed(patch, imm_offset, insn.immediate.nbytes, imm + diff)
                    .map_err(|_| X86ModuleFinalizeError::BadSection(".altinstructions"))?;
            }
        }

        if is_rip_relative(&insn) {
            let disp = sign_extend(insn.displacement.value, insn.displacement.nbytes);
            let next = offset as i64 + len as i64;
            if need_reloc(next + disp, repl_len) {
                let disp_offset = offset
                    + insn.prefixes.nbytes as usize
                    + insn.rex_prefix.nbytes as usize
                    + insn.opcode.nbytes as usize
                    + insn.modrm.nbytes as usize
                    + insn.sib.nbytes as usize;
                write_signed(patch, disp_offset, insn.displacement.nbytes, disp + diff)
                    .map_err(|_| X86ModuleFinalizeError::BadSection(".altinstructions"))?;
            }
        }

        offset += len;
    }
    Ok(())
}

fn finalize_static_call_sites(
    sections: &mut NameMap<LoadedSection>,
) -> Result<usize, X86ModuleFinalizeError> {
    let Some(section) = sections.get(".static_call_sites") else {
        return Ok(0);
    };
    if section.len() % STATIC_CALL_SITE_SIZE != 0 {
        return Err(X86ModuleFinalizeError::BadSection(".static_call_sites"));
    }
    let base = section.as_ptr() as usize;
    let data = section.as_slice().to_vec();
    let mut sites = Vec::with_capacity(data.len() / STATIC_CALL_SITE_SIZE);
    for offset in (0..data.len()).step_by(STATIC_CALL_SITE_SIZE) {
        let site_disp = read_i32(&data, offset)
            .ok_or(X86ModuleFinalizeError::BadSection(".static_call_sites"))?;
        let key_disp = read_i32(&data, offset + 4)
            .ok_or(X86ModuleFinalizeError::BadSection(".static_call_sites"))?;
        let site_addr = relative_addr(base + offset, site_disp);
        let key_value = relative_addr(base + offset + 4, key_disp);
        sites.push((site_addr, key_value));
    }

    let warn_tramp = warn_trap_trampoline_addr();
    let warn_func = warn_trap_addr();
    for (site_addr, key_value) in sites.iter().copied() {
        if key_value & STATIC_CALL_SITE_TAIL != 0 {
            return Err(X86ModuleFinalizeError::Unsupported(
                ".static_call_sites tail",
            ));
        }
        let key_addr = key_value & !STATIC_CALL_SITE_FLAGS;
        if key_addr != warn_tramp && key_addr != warn_func {
            return Err(X86ModuleFinalizeError::Unsupported(".static_call_sites"));
        }
        let site = loaded_mut_at(
            sections,
            site_addr,
            crate::arch::x86::kernel::static_call::CALL_INSN_SIZE,
        )
        .ok_or(X86ModuleFinalizeError::BadSection(".static_call_sites"))?;
        static_call_fixup_warn_site(site)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".static_call_sites"))?;
    }
    Ok(sites.len())
}

fn sort_module_extable(
    sections: &mut NameMap<LoadedSection>,
) -> Result<usize, X86ModuleFinalizeError> {
    let Some(section) = sections.get_mut("__ex_table") else {
        return Ok(0);
    };
    crate::arch::x86::kernel::extable::sort_extable_bytes(section.as_mut_slice())
        .map_err(|_| X86ModuleFinalizeError::BadSection("__ex_table"))?;
    Ok(section.len() / crate::arch::x86::kernel::extable::EXTABLE_ENTRY_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn apply_and_clear_relocation_batch() {
        let mut mem = [0u8; 8];
        let rel = ResolvedRela {
            rela: Rela {
                offset: 0,
                sym: 1,
                rel_type: RelocType::Plt32,
                addend: -4,
            },
            sym_addr: 0x1010,
        };
        apply_relocate_add(&mut mem, 0x1000, &[rel]).unwrap();
        assert_eq!(i32::from_le_bytes(mem[0..4].try_into().unwrap()), 0xc);
        clear_relocate_add(&mut mem, &[rel]);
        assert_eq!(&mem[0..4], &[0; 4]);
    }

    #[test]
    fn finalize_detects_x86_metadata_sections() {
        let mut sections = NameMap::new();
        sections.insert(
            String::from(".text"),
            LoadedSection::from_bytes(&[0xcc]).unwrap(),
        );
        sections.insert(
            String::from("__jump_table"),
            LoadedSection::from_bytes(&[]).unwrap(),
        );
        sections.insert(
            String::from(".orc_unwind"),
            LoadedSection::from_bytes(&[]).unwrap(),
        );
        let meta = module_finalize(&mut sections).unwrap();
        assert!(meta.has_jump_entries);
        assert!(meta.has_orc_unwind);
        assert!(!meta.alternatives_applied);
    }
}
