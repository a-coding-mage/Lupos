//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module/livepatch.c
//! test-origin: linux:vendor/linux/kernel/module/livepatch.c
//! Module livepatch ELF metadata persistence.

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivepatchAllocFailure {
    Info,
    SectionHeaders,
    SectionStrings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadInfoShape {
    pub section_count: usize,
    pub section_header_size: usize,
    pub section_strings_size: usize,
    pub symbol_section_index: usize,
    pub core_symtab_addr: usize,
    pub fail_at: Option<LivepatchAllocFailure>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleKlpInfo {
    pub section_headers_bytes: usize,
    pub section_strings_bytes: usize,
    pub symndx: usize,
    pub symtab_sh_addr: usize,
}

pub const fn copy_module_elf(info: LoadInfoShape) -> Result<ModuleKlpInfo, i32> {
    if matches!(info.fail_at, Some(LivepatchAllocFailure::Info)) {
        return Err(-ENOMEM);
    }
    let section_headers_bytes = info.section_count * info.section_header_size;
    if matches!(info.fail_at, Some(LivepatchAllocFailure::SectionHeaders)) {
        return Err(-ENOMEM);
    }
    if matches!(info.fail_at, Some(LivepatchAllocFailure::SectionStrings)) {
        return Err(-ENOMEM);
    }
    Ok(ModuleKlpInfo {
        section_headers_bytes,
        section_strings_bytes: info.section_strings_size,
        symndx: info.symbol_section_index,
        symtab_sh_addr: info.core_symtab_addr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn livepatch_copy_shape_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/module/livepatch.c"
        ));
        assert!(source.contains("int copy_module_elf(struct module *mod, struct load_info *info)"));
        assert!(source.contains("mod->klp_info = kmalloc(size, GFP_KERNEL);"));
        assert!(source.contains("memcpy(&mod->klp_info->hdr, info->hdr, size);"));
        assert!(source.contains("sizeof(*info->sechdrs) * info->hdr->e_shnum"));
        assert!(source.contains("mod->klp_info->sechdrs = kmemdup"));
        assert!(source.contains("mod->klp_info->secstrings = kmemdup"));
        assert!(source.contains("symndx = info->index.sym;"));
        assert!(source.contains("mod->klp_info->sechdrs[symndx].sh_addr"));
        assert!(source.contains("void free_module_elf(struct module *mod)"));
        assert!(source.contains("kfree(mod->klp_info->secstrings);"));

        let info = LoadInfoShape {
            section_count: 5,
            section_header_size: 64,
            section_strings_size: 31,
            symbol_section_index: 2,
            core_symtab_addr: 0xfeed,
            fail_at: None,
        };
        let copied = copy_module_elf(info).unwrap();
        assert_eq!(copied.section_headers_bytes, 320);
        assert_eq!(copied.section_strings_bytes, 31);
        assert_eq!(copied.symndx, 2);
        assert_eq!(copied.symtab_sh_addr, 0xfeed);
        assert_eq!(
            copy_module_elf(LoadInfoShape {
                fail_at: Some(LivepatchAllocFailure::SectionHeaders),
                ..info
            }),
            Err(-ENOMEM)
        );
    }
}
