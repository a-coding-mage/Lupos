//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/tools/relocs_32.c
//! test-origin: linux:vendor/linux/arch/x86/tools/relocs_32.c
//! 32-bit specialization of the x86 `relocs` host tool.

use super::{ELFCLASS32, EM_386, RelEntryKind, RelocsElfConfig, SHT_REL};

pub const RELOCS_32_CONFIG: RelocsElfConfig = RelocsElfConfig {
    elf_bits: 32,
    elf_machine: EM_386,
    elf_machine_name: "i386",
    sht_rel_type: SHT_REL,
    elf_class: ELFCLASS32,
    rel_entry: RelEntryKind::Rel,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relocs_32_defines_linux_elf_specialization() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/tools/relocs_32.c"
        ));
        assert!(source.contains("#define ELF_BITS 32"));
        assert!(source.contains("#define ELF_MACHINE\t\tEM_386"));
        assert!(source.contains("#define SHT_REL_TYPE\t\tSHT_REL"));
        assert!(source.contains("#include \"relocs.c\""));

        assert_eq!(
            RELOCS_32_CONFIG,
            RelocsElfConfig {
                elf_bits: 32,
                elf_machine: EM_386,
                elf_machine_name: "i386",
                sht_rel_type: SHT_REL,
                elf_class: ELFCLASS32,
                rel_entry: RelEntryKind::Rel,
            }
        );
    }
}
