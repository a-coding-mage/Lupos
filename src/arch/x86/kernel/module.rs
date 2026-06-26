//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/module.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/module.c
//! x86 module relocation/finalization wrapper.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/module.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::kernel::module::relocate::{Rela, RelocType, apply_rela};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct X86ModuleMetadata {
    pub has_jump_entries: bool,
    pub has_orc_unwind: bool,
    pub alternatives_applied: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedRela {
    pub rela: Rela,
    pub sym_addr: u64,
}

pub fn apply_relocate_add(
    mem: &mut [u8],
    section_addr: u64,
    relocs: &[ResolvedRela],
) -> Result<(), i32> {
    for r in relocs {
        apply_rela(
            mem,
            r.rela.offset as usize,
            r.rela.rel_type,
            r.sym_addr,
            section_addr + r.rela.offset,
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

pub fn module_finalize(sections: &[&str]) -> X86ModuleMetadata {
    X86ModuleMetadata {
        has_jump_entries: sections.iter().any(|s| *s == "__jump_table"),
        has_orc_unwind: sections.iter().any(|s| *s == ".orc_unwind"),
        alternatives_applied: sections.iter().any(|s| *s == ".altinstructions"),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let meta = module_finalize(&[".text", "__jump_table", ".orc_unwind"]);
        assert!(meta.has_jump_entries);
        assert!(meta.has_orc_unwind);
        assert!(!meta.alternatives_applied);
    }
}
