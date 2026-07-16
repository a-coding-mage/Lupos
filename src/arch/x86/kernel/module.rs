//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/module.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/module.c
//! x86 module relocation and finalization audit helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/module.c
//!
//! The relocation writer delegates to the runtime loader's bounded x86_64
//! implementation. `module_finalize()` mirrors the ordering in
//! `vendor/linux/arch/x86/kernel/module.c`: retpoline and return sites, call
//! sites, alternatives, IBT sealing, SMP locks, and paired ORC tables.

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86::kernel::alternative::{
    ALT_FLAG_DIRECT_CALL, ALT_FLAGS_SHIFT, AltInstr, CALL_INSN_OPCODE, JMP32_INSN_OPCODE,
    MAX_PATCH_LEN, RetpolinePatchPolicy, add_nops, alternatives_smp_module_add,
    alternatives_smp_module_del, decode_retpoline_site, patch_retpoline, patch_return, seal_endbr,
    text_poke_copy,
};
use crate::arch::x86::kernel::callthunks::{
    CALL_INSN_SIZE as CALLTHUNK_CALL_SIZE, SKL_CALL_THUNK_SIZE, call_get_dest, emit_call,
    install_call_thunk_padding, skl_call_thunk_template,
};
use crate::arch::x86::kernel::cpu::common::{
    X86_FEATURE_CALL_DEPTH, X86_FEATURE_RETHUNK, X86_FEATURE_RETPOLINE,
    X86_FEATURE_RETPOLINE_LFENCE, boot_cpu_has, x86_feature_limit,
};
use crate::arch::x86::kernel::jump_label::{JMP8_INSN_OPCODE, JumpLabelRegistration};
use crate::arch::x86::kernel::retpoline::{
    compiler_return_thunk_addr, retpoline_register, return_thunk_addr,
};
use crate::arch::x86::kernel::static_call::{
    RETINSN, STATIC_CALL_SITE_FLAGS, STATIC_CALL_SITE_SIZE, StaticCallRegistration, TRAMP_UD,
    static_call_fixup_warn_site, warn_trap_addr, warn_trap_trampoline_addr,
};
use crate::arch::x86::kernel::unwind_orc::OrcModuleRegistration;
use crate::arch::x86::lib::insn::Insn;
use crate::kernel::module::loader::{LoadedSection, NameMap};
use crate::kernel::module::relocate::{Rela, RelocType, apply_rela};

#[derive(Debug, Default)]
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
    pub num_retpoline_sites: usize,
    pub num_return_sites: usize,
    pub num_call_sites: usize,
    pub num_call_thunks: usize,
    pub num_cfi_sites: usize,
    pub num_ibt_endbr_seals: usize,
    pub num_orcs: usize,
    pub num_jump_entries: usize,
    orc_registration: Option<OrcModuleRegistration>,
    jump_label_registration: Option<JumpLabelRegistration>,
    static_call_registration: Option<StaticCallRegistration>,
    released: AtomicBool,
}

impl Drop for X86ModuleMetadata {
    fn drop(&mut self) {
        self.release();
    }
}

impl X86ModuleMetadata {
    pub fn orc_unwind_ip(&self) -> Option<usize> {
        self.orc_registration
            .as_ref()
            .map(|registration| registration.orc_unwind_ip)
    }

    pub fn orc_unwind(&self) -> Option<usize> {
        self.orc_registration
            .as_ref()
            .map(|registration| registration.orc_unwind)
    }

    /// Linux `module_arch_cleanup()`: withdraw every architecture-owned
    /// registry before the module's section allocations can be freed.  The
    /// module descriptor itself may remain referenced after `delete_module`,
    /// so cleanup cannot be deferred to this object's destructor.
    pub fn release(&self) {
        if self.released.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(registration) = &self.orc_registration {
            registration.unregister();
        }
        if let Some(registration) = &self.jump_label_registration {
            registration.unregister();
        }
        if let Some(registration) = &self.static_call_registration {
            registration.unregister();
        }
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
    let num_cfi_sites = apply_module_cfi_policy(sections)?;
    let num_retpoline_sites = apply_module_retpolines(sections)?;
    let num_return_sites = apply_module_returns(sections)?;
    let (num_call_sites, num_call_thunks) = finalize_module_call_sites(sections)?;
    let alternatives_applied = apply_module_alternatives(sections)?;
    let num_ibt_endbr_seals = apply_module_ibt_seals(sections)?;
    let (num_static_call_sites, static_call_registration) = finalize_static_call_sites(sections)?;
    let (num_jump_entries, jump_label_registration) = finalize_jump_entries(sections)?;
    let num_extable_entries = sort_module_extable(sections)?;
    let orc_registration = finalize_module_orc(sections)?;
    let num_orcs = orc_registration
        .as_ref()
        .map(|registration| registration.num_orcs)
        .unwrap_or(0);
    Ok(X86ModuleMetadata {
        has_jump_entries: sections.contains_key("__jump_table"),
        has_orc_unwind: sections.contains_key(".orc_unwind"),
        has_alternatives: sections.contains_key(".altinstructions"),
        has_smp_locks,
        alternatives_applied,
        smp_locks_registered: has_smp_locks && alternatives_smp_module_add(),
        num_static_call_sites,
        num_extable_entries,
        num_retpoline_sites,
        num_return_sites,
        num_call_sites,
        num_call_thunks,
        num_cfi_sites,
        num_ibt_endbr_seals,
        num_orcs,
        num_jump_entries,
        orc_registration,
        jump_label_registration,
        static_call_registration,
        released: AtomicBool::new(false),
    })
}

pub fn module_arch_cleanup(metadata: &X86ModuleMetadata) {
    metadata.release();
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

fn loaded_tail_at(
    sections: &NameMap<LoadedSection>,
    addr: usize,
    max_size: usize,
) -> Option<Vec<u8>> {
    for section in sections.values() {
        let base = section.as_ptr() as usize;
        let Some(offset) = addr.checked_sub(base) else {
            continue;
        };
        if offset < section.len() {
            let end = section.len().min(offset.saturating_add(max_size));
            return Some(section.as_slice()[offset..end].to_vec());
        }
    }
    None
}

fn parse_prel32_sites(
    sections: &NameMap<LoadedSection>,
    name: &'static str,
) -> Result<Vec<usize>, X86ModuleFinalizeError> {
    let Some(section) = sections.get(name) else {
        return Ok(Vec::new());
    };
    if section.len() % core::mem::size_of::<i32>() != 0 {
        return Err(X86ModuleFinalizeError::BadSection(name));
    }
    let base = section.as_ptr() as usize;
    let mut sites = Vec::with_capacity(section.len() / 4);
    for offset in (0..section.len()).step_by(4) {
        let displacement =
            read_i32(section.as_slice(), offset).ok_or(X86ModuleFinalizeError::BadSection(name))?;
        sites.push(relative_addr(base + offset, displacement));
    }
    Ok(sites)
}

/// `apply_retpolines()` from vendor/linux/arch/x86/kernel/alternative.c.
fn apply_module_retpolines(
    sections: &mut NameMap<LoadedSection>,
) -> Result<usize, X86ModuleFinalizeError> {
    let sites = parse_prel32_sites(sections, ".retpoline_sites")?;
    let policy = RetpolinePatchPolicy {
        retpoline: boot_cpu_has(X86_FEATURE_RETPOLINE),
        retpoline_lfence: boot_cpu_has(X86_FEATURE_RETPOLINE_LFENCE),
        call_depth: boot_cpu_has(X86_FEATURE_CALL_DEPTH),
    };
    for site_addr in sites.iter().copied() {
        let bytes = loaded_tail_at(sections, site_addr, 15)
            .ok_or(X86ModuleFinalizeError::BadSection(".retpoline_sites"))?;
        let site = decode_retpoline_site(site_addr, &bytes, retpoline_register)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".retpoline_sites"))?;
        let Some(patch) = patch_retpoline(site_addr, site, policy)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".retpoline_sites"))?
        else {
            continue;
        };
        let destination = loaded_mut_at(sections, site_addr, patch.len())
            .ok_or(X86ModuleFinalizeError::BadSection(".retpoline_sites"))?;
        text_poke_copy(destination, &patch)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".retpoline_sites"))?;
    }
    Ok(sites.len())
}

/// `apply_returns()` from vendor/linux/arch/x86/kernel/alternative.c.
fn apply_module_returns(
    sections: &mut NameMap<LoadedSection>,
) -> Result<usize, X86ModuleFinalizeError> {
    let sites = parse_prel32_sites(sections, ".return_sites")?;
    let compiler_return_thunk = compiler_return_thunk_addr();
    let selected_return_thunk = return_thunk_addr();
    let wants_rethunk = boot_cpu_has(X86_FEATURE_RETHUNK);
    for site_addr in sites.iter().copied() {
        let bytes = loaded_tail_at(sections, site_addr, 8)
            .ok_or(X86ModuleFinalizeError::BadSection(".return_sites"))?;
        if bytes.len() >= 8
            && bytes[5..8] == TRAMP_UD
            && (bytes[0] == crate::arch::x86::kernel::alternative::RET_INSN_OPCODE
                || (bytes[0] == JMP32_INSN_OPCODE
                    && patch_return(
                        site_addr,
                        &bytes,
                        compiler_return_thunk,
                        selected_return_thunk,
                        true,
                    )
                    .is_ok()))
        {
            // __static_call_fixup(): a NULL static-call return trampoline is
            // normalized to RETINSN before ordinary return-site handling.
            let destination = loaded_mut_at(sections, site_addr, RETINSN.len())
                .ok_or(X86ModuleFinalizeError::BadSection(".return_sites"))?;
            text_poke_copy(destination, &RETINSN)
                .map_err(|_| X86ModuleFinalizeError::BadSection(".return_sites"))?;
            continue;
        }
        let patch = patch_return(
            site_addr,
            &bytes,
            compiler_return_thunk,
            selected_return_thunk,
            wants_rethunk,
        )
        .map_err(|_| X86ModuleFinalizeError::BadSection(".return_sites"))?;
        let destination = loaded_mut_at(sections, site_addr, patch.len())
            .ok_or(X86ModuleFinalizeError::BadSection(".return_sites"))?;
        text_poke_copy(destination, &patch)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".return_sites"))?;
    }
    Ok(sites.len())
}

fn is_core_text_name(name: &str) -> bool {
    !name.starts_with(".init")
        && !name.starts_with(".exit")
        && (name == ".text" || name.starts_with(".text.") || name.ends_with(".text"))
}

fn is_module_core_text(sections: &NameMap<LoadedSection>, address: usize, size: usize) -> bool {
    sections.iter().any(|(name, section)| {
        if !is_core_text_name(name) {
            return false;
        }
        let base = section.as_ptr() as usize;
        address
            .checked_sub(base)
            .and_then(|offset| offset.checked_add(size))
            .is_some_and(|end| end <= section.len())
    })
}

/// `callthunks_patch_module_calls()`. The metadata is always consumed; Linux
/// deliberately performs no mutation until its software CALL_DEPTH feature
/// bit is selected for an affected CPU.
fn finalize_module_call_sites(
    sections: &mut NameMap<LoadedSection>,
) -> Result<(usize, usize), X86ModuleFinalizeError> {
    finalize_module_call_sites_with_policy(
        sections,
        boot_cpu_has(X86_FEATURE_CALL_DEPTH),
        crate::arch::x86::kernel::setup_percpu::x86_call_depth_symbol() as u64,
    )
}

fn finalize_module_call_sites_with_policy(
    sections: &mut NameMap<LoadedSection>,
    call_depth_enabled: bool,
    percpu_symbol: u64,
) -> Result<(usize, usize), X86ModuleFinalizeError> {
    let sites = parse_prel32_sites(sections, ".call_sites")?;
    if !call_depth_enabled {
        return Ok((sites.len(), 0));
    }

    let template = skl_call_thunk_template(percpu_symbol)
        .map_err(|_| X86ModuleFinalizeError::BadSection(".call_sites"))?;
    let mut patched = 0usize;
    for site in sites.iter().copied() {
        // Linux ignores metadata outside MOD_TEXT (notably init text).
        if !is_module_core_text(sections, site, CALLTHUNK_CALL_SIZE) {
            continue;
        }
        let instruction = loaded_bytes_at(sections, site, CALLTHUNK_CALL_SIZE)
            .ok_or(X86ModuleFinalizeError::BadSection(".call_sites"))?;
        let Some(destination) = call_get_dest(site as u64, &instruction)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".call_sites"))?
        else {
            // An earlier alternative may have removed this call.
            continue;
        };
        let destination = destination as usize;
        if !is_module_core_text(sections, destination, 1) {
            // Lupos core text is not compiled with Linux function padding.
            // As Linux does for non-core destinations, retain the direct call.
            continue;
        }
        let padding_addr = destination
            .checked_sub(SKL_CALL_THUNK_SIZE)
            .ok_or(X86ModuleFinalizeError::BadSection(".call_sites"))?;
        if !is_module_core_text(sections, padding_addr, SKL_CALL_THUNK_SIZE) {
            continue;
        }
        let padding = loaded_mut_at(sections, padding_addr, SKL_CALL_THUNK_SIZE)
            .ok_or(X86ModuleFinalizeError::BadSection(".call_sites"))?;
        if install_call_thunk_padding(padding, &template).is_err() {
            // `patch_dest()` warns and leaves this call unchanged when a
            // function lacks the compiler-reserved NOP prefix.
            continue;
        }
        let call = emit_call(site as u64, padding_addr as u64)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".call_sites"))?;
        let destination_bytes = loaded_mut_at(sections, site, CALLTHUNK_CALL_SIZE)
            .ok_or(X86ModuleFinalizeError::BadSection(".call_sites"))?;
        text_poke_copy(destination_bytes, &call)
            .map_err(|_| X86ModuleFinalizeError::BadSection(".call_sites"))?;
        patched += 1;
    }
    Ok((sites.len(), patched))
}

const FINEIBT_CALLER_SIZE: usize = 14;
const FINEIBT_CALLER_JUMP: u8 = 12;

/// Apply the vendor kernel's `cfi=off` module policy. Lupos is not compiled
/// with Clang KCFI, so accepting `.cfi_sites` while leaving caller-side type
/// checks active would route failures into an ABI which does not exist here.
/// Linux first converts each typed caller prefix into `jmp +12`, preserving
/// the hash bytes for a later mode change, then leaves the callee preambles
/// untouched. This is still full metadata consumption, not silent ignoring.
fn apply_module_cfi_policy(
    sections: &mut NameMap<LoadedSection>,
) -> Result<usize, X86ModuleFinalizeError> {
    let cfi_sites = parse_prel32_sites(sections, ".cfi_sites")?;
    if cfi_sites.is_empty() {
        return Ok(0);
    }
    for address in cfi_sites.iter().copied() {
        let preamble = loaded_bytes_at(sections, address, 5)
            .ok_or(X86ModuleFinalizeError::BadSection(".cfi_sites"))?;
        if !(0xb8..=0xbf).contains(&preamble[0]) {
            return Err(X86ModuleFinalizeError::BadSection(".cfi_sites"));
        }
    }
    let retpolines = parse_prel32_sites(sections, ".retpoline_sites")?;
    for site in retpolines {
        let Some(prefix_addr) = site.checked_sub(FINEIBT_CALLER_SIZE) else {
            return Err(X86ModuleFinalizeError::BadSection(".retpoline_sites"));
        };
        let Some(prefix) = loaded_bytes_at(sections, prefix_addr, 6) else {
            continue;
        };
        let typed = prefix[0..2] == [0x41, 0xba];
        let already_disabled = prefix[0..2] == [JMP8_INSN_OPCODE, FINEIBT_CALLER_JUMP];
        if !typed && !already_disabled {
            // `nocfi` callers do not carry a decodable hash prefix.
            continue;
        }
        if typed {
            let destination = loaded_mut_at(sections, prefix_addr, 2)
                .ok_or(X86ModuleFinalizeError::BadSection(".retpoline_sites"))?;
            text_poke_copy(destination, &[JMP8_INSN_OPCODE, FINEIBT_CALLER_JUMP])
                .map_err(|_| X86ModuleFinalizeError::BadSection(".retpoline_sites"))?;
        }
    }
    Ok(cfi_sites.len())
}

/// `apply_seal_endbr()` from vendor/linux/arch/x86/kernel/alternative.c.
fn apply_module_ibt_seals(
    sections: &mut NameMap<LoadedSection>,
) -> Result<usize, X86ModuleFinalizeError> {
    let sites = parse_prel32_sites(sections, ".ibt_endbr_seal")?;
    for site_addr in sites.iter().copied() {
        let site = loaded_mut_at(
            sections,
            site_addr,
            crate::arch::x86::kernel::alternative::ENDBR_INSN_SIZE,
        )
        .ok_or(X86ModuleFinalizeError::BadSection(".ibt_endbr_seal"))?;
        seal_endbr(site).map_err(|_| X86ModuleFinalizeError::BadSection(".ibt_endbr_seal"))?;
    }
    Ok(sites.len())
}

fn finalize_module_orc(
    sections: &mut NameMap<LoadedSection>,
) -> Result<Option<OrcModuleRegistration>, X86ModuleFinalizeError> {
    if let Some(header) = sections.get(".orc_header")
        && header.as_slice() != crate::arch::x86::kernel::unwind_orc::VENDOR_ORC_HASH
    {
        return Err(X86ModuleFinalizeError::BadSection(".orc_header"));
    }
    let (Some(ip_section), Some(orc_section)) =
        (sections.get(".orc_unwind_ip"), sections.get(".orc_unwind"))
    else {
        // This is the exact `if (orc && orc_ip)` condition in module.c.
        return Ok(None);
    };
    let ip_base = ip_section.as_ptr() as usize;
    let orc_base = orc_section.as_ptr() as usize;
    let mut text_ranges = Vec::new();
    for (name, section) in sections.iter() {
        if name == ".text" || name.starts_with(".text.") || name.ends_with(".text") {
            let start = section.as_ptr() as usize;
            let end = start.saturating_add(section.len());
            if start < end {
                text_ranges.push((start, end));
            }
        }
    }
    let mut ip_bytes = ip_section.as_slice().to_vec();
    let mut orc_bytes = orc_section.as_slice().to_vec();
    let num_entries = crate::arch::x86::kernel::unwind_orc::sort_module_orc_tables(
        ip_base,
        &mut ip_bytes,
        &mut orc_bytes,
    )
    .map_err(|_| X86ModuleFinalizeError::BadSection(".orc_unwind"))?;
    sections
        .get_mut(".orc_unwind_ip")
        .ok_or(X86ModuleFinalizeError::BadSection(".orc_unwind_ip"))?
        .as_mut_slice()
        .copy_from_slice(&ip_bytes);
    sections
        .get_mut(".orc_unwind")
        .ok_or(X86ModuleFinalizeError::BadSection(".orc_unwind"))?
        .as_mut_slice()
        .copy_from_slice(&orc_bytes);
    crate::arch::x86::kernel::unwind_orc::register_sorted_module_orc_tables_for_ranges(
        ip_base,
        &ip_bytes,
        orc_base,
        &orc_bytes,
        num_entries,
        &text_ranges,
    )
    .map_err(|_| X86ModuleFinalizeError::BadSection(".orc_unwind"))
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
        let repl_len = entry.replacementlen as usize;
        if repl_len > patch_len {
            return Err(X86ModuleFinalizeError::BadSection(".altinstructions"));
        }
        let replacement = loaded_bytes_at(sections, entry.repl_addr, repl_len)
            .ok_or(X86ModuleFinalizeError::BadSection(".altinstructions"))?;
        let mut patch = alloc::vec![0x90u8; patch_len];
        if entry.flags & ALT_FLAG_DIRECT_CALL != 0 {
            // alt_replace_call(): vendor/linux/arch/x86/kernel/alternative.c.
            // A five-byte `call BUG_func` replacement inherits the target of
            // the original six-byte `call *disp32(%rip)` pv_ops instruction.
            if repl_len != 5
                || replacement.first().copied() != Some(CALL_INSN_OPCODE)
                || entry.instrlen != 6
            {
                return Err(X86ModuleFinalizeError::BadSection(
                    ".altinstructions ALT_FLAG_DIRECT_CALL",
                ));
            }
            let original = loaded_bytes_at(sections, instr_addr, 6).ok_or(
                X86ModuleFinalizeError::BadSection(".altinstructions ALT_FLAG_DIRECT_CALL"),
            )?;
            if original[..2] != [0xff, 0x15] {
                return Err(X86ModuleFinalizeError::BadSection(
                    ".altinstructions ALT_FLAG_DIRECT_CALL",
                ));
            }
            let pointer_disp = i32::from_le_bytes(original[2..6].try_into().map_err(|_| {
                X86ModuleFinalizeError::BadSection(".altinstructions ALT_FLAG_DIRECT_CALL")
            })?);
            let pointer_addr = instr_addr
                .wrapping_add(6)
                .wrapping_add_signed(pointer_disp as isize);
            let mut target_bytes = [0u8; core::mem::size_of::<usize>()];
            unsafe {
                crate::arch::x86::mm::maccess::copy_from_kernel_nofault(
                    target_bytes.as_mut_ptr(),
                    pointer_addr as *const u8,
                    target_bytes.len(),
                )
            }
            .map_err(|_| {
                X86ModuleFinalizeError::BadSection(".altinstructions ALT_FLAG_DIRECT_CALL")
            })?;
            let target = match usize::from_le_bytes(target_bytes) {
                0 => crate::arch::x86::kernel::alternative::BUG_func as usize,
                target => target,
            };
            if target == crate::arch::x86::kernel::alternative::nop_func as usize {
                add_nops(&mut patch);
                let dst = loaded_mut_at(sections, instr_addr, patch_len)
                    .ok_or(X86ModuleFinalizeError::BadSection(".altinstructions"))?;
                text_poke_copy(dst, &patch)
                    .map_err(|_| X86ModuleFinalizeError::BadSection(".altinstructions"))?;
                applied = true;
                continue;
            }
            let next = instr_addr.wrapping_add(5);
            let relative = target as i128 - next as i128;
            if !(i32::MIN as i128..=i32::MAX as i128).contains(&relative) {
                return Err(X86ModuleFinalizeError::BadSection(
                    ".altinstructions ALT_FLAG_DIRECT_CALL",
                ));
            }
            patch[0] = CALL_INSN_OPCODE;
            patch[1..5].copy_from_slice(&(relative as i32).to_le_bytes());
            add_nops(&mut patch[5..]);
        } else {
            patch[..repl_len].copy_from_slice(&replacement);
            add_nops(&mut patch[repl_len..]);
            apply_alt_relocation(&mut patch, instr_addr, entry.repl_addr, repl_len)?;
        }
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
) -> Result<(usize, Option<StaticCallRegistration>), X86ModuleFinalizeError> {
    if let Some(section) = sections.get_mut(".static_call_sites") {
        let base = section.as_ptr() as usize;
        crate::arch::x86::kernel::static_call::sort_module_static_call_sites(
            base,
            section.as_mut_slice(),
        )
        .map_err(|_| X86ModuleFinalizeError::BadSection(".static_call_sites"))?;
    }
    let Some(section) = sections.get(".static_call_sites") else {
        return Ok((0, None));
    };
    if section.len() % STATIC_CALL_SITE_SIZE != 0 {
        return Err(X86ModuleFinalizeError::BadSection(".static_call_sites"));
    }
    let base = section.as_ptr() as usize;
    let count = section.len() / STATIC_CALL_SITE_SIZE;
    let registration = crate::arch::x86::kernel::static_call::register_module_static_call_sites(
        base,
        section.as_slice(),
    )
    .map_err(|_| X86ModuleFinalizeError::BadSection(".static_call_sites"))?;
    Ok((count, registration))
}

fn finalize_jump_entries(
    sections: &mut NameMap<LoadedSection>,
) -> Result<(usize, Option<JumpLabelRegistration>), X86ModuleFinalizeError> {
    let Some(section) = sections.get_mut("__jump_table") else {
        return Ok((0, None));
    };
    let base = section.as_ptr() as usize;
    let count = crate::arch::x86::kernel::jump_label::sort_module_jump_entries(
        base,
        section.as_mut_slice(),
    )
    .map_err(|_| X86ModuleFinalizeError::BadSection("__jump_table"))?;
    let registration = crate::arch::x86::kernel::jump_label::register_module_jump_entries(
        base,
        section.as_slice(),
    )
    .map_err(|_| X86ModuleFinalizeError::BadSection("__jump_table"))?;
    Ok((count, registration))
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

    fn one_borrowed_prel32_section(bytes: &mut [u8], target: usize) -> LoadedSection {
        let mut section = LoadedSection::borrowed_for_prel32_test(bytes);
        let base = section.as_ptr() as usize;
        let displacement = target as i128 - base as i128;
        assert!((i32::MIN as i128..=i32::MAX as i128).contains(&displacement));
        section
            .as_mut_slice()
            .copy_from_slice(&(displacement as i32).to_le_bytes());
        section
    }

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

    /// Every staged vendor module with WARN static calls (scsi_mod, libata,
    /// drm, i915, snd*, libphy, ...) relocates its `.static_call_sites` key
    /// against the exported `__SCT__WARN_trap`
    /// (vendor/linux/arch/x86/kernel/traps.c::EXPORT_STATIC_CALL_TRAMP).
    /// static_call_add_module() masks the low key bits as INIT/TAIL flags
    /// (vendor/linux/kernel/static_call_inline.c), so the exported addresses
    /// must be flag-bit clean: an unaligned trampoline made the masked
    /// comparison fail build-dependently, every such module was rejected with
    /// ENOEXEC, and `systemd-modules-load` stranded the guest without any
    /// network/storage/graphics driver.
    #[test]
    fn warn_trap_exports_are_flag_bit_clean_and_registered() {
        crate::arch::x86::kernel::static_call::register_module_exports();
        let tramp =
            crate::kernel::module::find_symbol("__SCT__WARN_trap").expect("trampoline exported");
        let func = crate::kernel::module::find_symbol("__WARN_trap").expect("function exported");
        assert_eq!(tramp, warn_trap_trampoline_addr());
        assert_eq!(func, warn_trap_addr());
        assert_eq!(
            tramp & STATIC_CALL_SITE_FLAGS,
            0,
            "trampoline address must not alias static-call site flag bits"
        );
        assert_eq!(
            func & STATIC_CALL_SITE_FLAGS,
            0,
            "__WARN_trap address must not alias static-call site flag bits"
        );
    }

    /// A relocated `static_call_mod(WARN_trap)` site is `call __SCT__WARN_trap`;
    /// the inline transform rewrites it to the vendor WARNINSN
    /// (`ud1 (%edx), %rdi`, vendor/linux/arch/x86/entry/entry.S::__WARN_trap)
    /// that handle_bug() reports and skips.
    #[test]
    fn warn_site_fixup_rewrites_call_to_warninsn() {
        let mut site = [0xe8u8, 0, 0, 0, 0];
        static_call_fixup_warn_site(&mut site).expect("call site must be patchable");
        assert_eq!(site, crate::arch::x86::kernel::static_call::WARNINSN);
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

    #[test]
    fn enabled_call_depth_patches_padding_and_direct_call() {
        let mut backing = [0x90; 68];
        backing[64..].fill(0);
        let mut text = LoadedSection::borrowed_for_prel32_test(&mut backing[..64]);
        let text_base = text.as_ptr() as usize;
        let function = text_base + 16;
        let callsite = text_base + 32;
        let original = emit_call(callsite as u64, function as u64).unwrap();
        text.as_mut_slice()[32..37].copy_from_slice(&original);

        let mut sections = NameMap::new();
        sections.insert(String::from(".text"), text);
        sections.insert(
            String::from(".call_sites"),
            one_borrowed_prel32_section(&mut backing[64..68], callsite),
        );
        assert_eq!(
            finalize_module_call_sites_with_policy(&mut sections, true, 0x1234),
            Ok((1, 1))
        );

        let text = sections.get(".text").unwrap().as_slice();
        let expected = skl_call_thunk_template(0x1234).unwrap();
        assert_eq!(&text[6..16], &expected);
        assert_eq!(
            call_get_dest(callsite as u64, &text[32..37]),
            Ok(Some((text_base + 6) as u64))
        );
    }

    #[test]
    fn cfi_off_policy_disables_typed_callers_and_consumes_sites() {
        let mut backing = [0x90; 72];
        backing[64..].fill(0);
        let mut text = LoadedSection::borrowed_for_prel32_test(&mut backing[..64]);
        let text_base = text.as_ptr() as usize;
        text.as_mut_slice()[0..6].copy_from_slice(&[0x41, 0xba, 0x78, 0x56, 0x34, 0x12]);
        text.as_mut_slice()[32..37].copy_from_slice(&[0xb8, 0x78, 0x56, 0x34, 0x12]);
        let mut sections = NameMap::new();
        sections.insert(String::from(".text"), text);
        sections.insert(
            String::from(".retpoline_sites"),
            one_borrowed_prel32_section(&mut backing[64..68], text_base + FINEIBT_CALLER_SIZE),
        );
        sections.insert(
            String::from(".cfi_sites"),
            one_borrowed_prel32_section(&mut backing[68..72], text_base + 32),
        );
        assert_eq!(apply_module_cfi_policy(&mut sections), Ok(1));
        assert_eq!(
            &sections.get(".text").unwrap().as_slice()[0..2],
            &[JMP8_INSN_OPCODE, FINEIBT_CALLER_JUMP]
        );
    }

    #[test]
    fn orc_header_rejects_a_different_packed_entry_abi() {
        let mut sections = NameMap::new();
        sections.insert(
            String::from(".orc_header"),
            LoadedSection::from_bytes(&[0u8; 20]).unwrap(),
        );
        assert!(matches!(
            finalize_module_orc(&mut sections),
            Err(X86ModuleFinalizeError::BadSection(".orc_header"))
        ));
    }
}
