//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/vdso/vma.c
//! test-origin: linux:vendor/linux/arch/x86/um/vdso/vma.c
//! UML vDSO page allocation and special mapping.

use crate::include::uapi::errno::EINTR;

pub const PAGE_SIZE: usize = crate::mm::frame::PAGE_SIZE;
pub const VDSO_MAPPING_NAME: &str = "[vdso]";
pub const VM_READ: u32 = 1 << 0;
pub const VM_EXEC: u32 = 1 << 2;
pub const VM_MAYREAD: u32 = 1 << 3;
pub const VM_MAYWRITE: u32 = 1 << 4;
pub const VM_MAYEXEC: u32 = 1 << 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UmlVdsoMapping {
    pub addr: usize,
    pub size: usize,
    pub flags: u32,
    pub name: &'static str,
}

pub const fn init_vdso_addr(task_size: usize, vdso_len: usize) -> Option<usize> {
    if vdso_len > PAGE_SIZE {
        None
    } else {
        Some(task_size - PAGE_SIZE)
    }
}

pub const fn arch_setup_additional_pages(
    mmap_lock_killable_failed: bool,
    um_vdso_addr: usize,
) -> Result<UmlVdsoMapping, i32> {
    if mmap_lock_killable_failed {
        return Err(-EINTR);
    }
    Ok(UmlVdsoMapping {
        addr: um_vdso_addr,
        size: PAGE_SIZE,
        flags: VM_READ | VM_EXEC | VM_MAYREAD | VM_MAYWRITE | VM_MAYEXEC,
        name: VDSO_MAPPING_NAME,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_vdso_mapping_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/vdso/vma.c"
        ));
        assert!(source.contains("unsigned long um_vdso_addr;"));
        assert!(source.contains("BUG_ON(vdso_end - vdso_start > PAGE_SIZE);"));
        assert!(source.contains("um_vdso_addr = task_size - PAGE_SIZE;"));
        assert!(source.contains("alloc_page(GFP_KERNEL);"));
        assert!(source.contains("copy_page(page_address(um_vdso), vdso_start);"));
        assert!(source.contains("subsys_initcall(init_vdso);"));
        assert!(source.contains(".name = \"[vdso]\""));
        assert!(source.contains("mmap_write_lock_killable(mm)"));
        assert!(source.contains("_install_special_mapping"));
        assert!(source.contains("VM_READ|VM_EXEC|"));

        assert_eq!(init_vdso_addr(0x8000_0000, PAGE_SIZE), Some(0x7fff_f000));
        assert_eq!(init_vdso_addr(0x8000_0000, PAGE_SIZE + 1), None);
        let mapping = arch_setup_additional_pages(false, 0x7000).unwrap();
        assert_eq!(mapping.name, "[vdso]");
        assert_eq!(mapping.flags & VM_EXEC, VM_EXEC);
        assert_eq!(arch_setup_additional_pages(true, 0x7000), Err(-EINTR));
    }
}
