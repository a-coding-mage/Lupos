//! linux-parity: complete
//! linux-source: vendor/linux/fs/tests/binfmt_elf_kunit.c
//! test-origin: linux:vendor/linux/fs/tests/binfmt_elf_kunit.c
//! Rust mirror of the Linux ELF total_mapping_size KUnit cases.

use crate::fs::binfmt_elf::{PAGE_SIZE, PT_INTERP, PT_LOAD};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ElfPhdr {
    pub p_type: u32,
    pub p_vaddr: u64,
    pub p_memsz: u64,
}

pub fn total_mapping_size_from_phdrs(phdrs: &[ElfPhdr]) -> u64 {
    let mut min_addr = u64::MAX;
    let mut max_addr = 0;
    let mut pt_load = false;

    for phdr in phdrs {
        if phdr.p_type == PT_LOAD {
            min_addr = core::cmp::min(min_addr, phdr.p_vaddr & !(PAGE_SIZE - 1));
            max_addr = core::cmp::max(max_addr, phdr.p_vaddr.saturating_add(phdr.p_memsz));
            pt_load = true;
        }
    }

    if pt_load { max_addr - min_addr } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::binfmt_elf::{
        PT_DYNAMIC, PT_GNU_RELRO, PT_GNU_STACK, PT_NOTE, PT_NULL, PT_PHDR,
    };

    #[test]
    fn binfmt_elf_kunit_source_shape_is_preserved() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/tests/binfmt_elf_kunit.c"
        ));
        assert!(source.contains("#include <kunit/test.h>"));
        assert!(source.contains("static void total_mapping_size_test"));
        assert!(source.contains("struct elf_phdr empty[]"));
        assert!(source.contains("struct elf_phdr mount[]"));
        assert!(source.contains("struct elf_phdr unordered[]"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, total_mapping_size(NULL, 0), 0);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, total_mapping_size(empty, 0), 0);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, total_mapping_size(&empty[1], 1), 0);"));
        assert!(source.contains(
            "KUNIT_EXPECT_EQ(test, total_mapping_size(mount, ARRAY_SIZE(mount)), mount_size);"
        ));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, total_mapping_size(unordered, ARRAY_SIZE(unordered)), mount_size);"));
        assert!(source.contains("KUNIT_CASE(total_mapping_size_test)"));
        assert!(source.contains("kunit_test_suite(binfmt_elf_test_suite);"));
    }

    #[test]
    fn total_mapping_size_cases_match_linux_kunit() {
        let empty = [
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0,
                p_memsz: 0,
            },
            ElfPhdr {
                p_type: PT_INTERP,
                p_vaddr: 10,
                p_memsz: 999_999,
            },
        ];
        let mount = [
            ElfPhdr {
                p_type: PT_PHDR,
                p_vaddr: 0x0000_0040,
                p_memsz: 0x0002_d8,
            },
            ElfPhdr {
                p_type: PT_INTERP,
                p_vaddr: 0x0000_0318,
                p_memsz: 0x0000_1c,
            },
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0x0000_0000,
                p_memsz: 0x0033_a8,
            },
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0x0000_4000,
                p_memsz: 0x005c_91,
            },
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0x0000_a000,
                p_memsz: 0x0022_f8,
            },
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0x0000_d330,
                p_memsz: 0x000d_40,
            },
            ElfPhdr {
                p_type: PT_DYNAMIC,
                p_vaddr: 0x0000_d928,
                p_memsz: 0x0002_00,
            },
            ElfPhdr {
                p_type: PT_NOTE,
                p_vaddr: 0x0000_0338,
                p_memsz: 0x0000_30,
            },
            ElfPhdr {
                p_type: PT_NOTE,
                p_vaddr: 0x0000_0368,
                p_memsz: 0x0000_44,
            },
            ElfPhdr {
                p_type: PT_NULL,
                p_vaddr: 0x0000_0338,
                p_memsz: 0x0000_30,
            },
            ElfPhdr {
                p_type: PT_NULL,
                p_vaddr: 0x0000_b490,
                p_memsz: 0x0001_ec,
            },
            ElfPhdr {
                p_type: PT_GNU_STACK,
                p_vaddr: 0,
                p_memsz: 0,
            },
            ElfPhdr {
                p_type: PT_GNU_RELRO,
                p_vaddr: 0x0000_d330,
                p_memsz: 0x000c_d0,
            },
        ];
        let unordered = [
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0,
                p_memsz: 0x0033_a8,
            },
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0x0000_d330,
                p_memsz: 0x000d_40,
            },
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0x0000_4000,
                p_memsz: 0x005c_91,
            },
            ElfPhdr {
                p_type: PT_LOAD,
                p_vaddr: 0x0000_a000,
                p_memsz: 0x0022_f8,
            },
        ];

        assert_eq!(total_mapping_size_from_phdrs(&[]), 0);
        assert_eq!(total_mapping_size_from_phdrs(&empty[..0]), 0);
        assert_eq!(total_mapping_size_from_phdrs(&empty[..1]), 0);
        assert_eq!(total_mapping_size_from_phdrs(&empty[1..2]), 0);
        assert_eq!(total_mapping_size_from_phdrs(&empty), 0);
        assert_eq!(total_mapping_size_from_phdrs(&mount), 0xe070);
        assert_eq!(total_mapping_size_from_phdrs(&unordered), 0xe070);
    }
}
