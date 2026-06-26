//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/tools
//! x86 host-side build tools mirrored for layout and generator parity.

pub mod relocs_32;
pub mod relocs_64;
pub mod relocs_common;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelEntryKind {
    Rel,
    Rela,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelocsElfConfig {
    pub elf_bits: u8,
    pub elf_machine: u16,
    pub elf_machine_name: &'static str,
    pub sht_rel_type: u32,
    pub elf_class: u8,
    pub rel_entry: RelEntryKind,
}

pub const EM_386: u16 = 3;
pub const EM_X86_64: u16 = 62;
pub const SHT_RELA: u32 = 4;
pub const SHT_REL: u32 = 9;
pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;
