//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/tools/relocs_64.c
//! test-origin: linux:vendor/linux/arch/x86/tools/relocs_64.c
//! 64-bit specialization of the x86 `relocs` host tool.

use super::{ELFCLASS64, EM_X86_64, RelEntryKind, RelocsElfConfig, SHT_RELA};

pub const RELOCS_64_CONFIG: RelocsElfConfig = RelocsElfConfig {
    elf_bits: 64,
    elf_machine: EM_X86_64,
    elf_machine_name: "x86_64",
    sht_rel_type: SHT_RELA,
    elf_class: ELFCLASS64,
    rel_entry: RelEntryKind::Rela,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relocs_64_defines_linux_elf_specialization() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/tools/relocs_64.c"
        ));
        assert!(source.contains("#define ELF_BITS 64"));
        assert!(source.contains("#define ELF_MACHINE             EM_X86_64"));
        assert!(source.contains("#define SHT_REL_TYPE            SHT_RELA"));
        assert!(source.contains("#include \"relocs.c\""));

        assert_eq!(
            RELOCS_64_CONFIG,
            RelocsElfConfig {
                elf_bits: 64,
                elf_machine: EM_X86_64,
                elf_machine_name: "x86_64",
                sht_rel_type: SHT_RELA,
                elf_class: ELFCLASS64,
                rel_entry: RelEntryKind::Rela,
            }
        );
    }
}
