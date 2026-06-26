//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/mem_64.c
//! test-origin: linux:vendor/linux/arch/x86/um/mem_64.c
//! UML x86-64 VMA naming.

pub const UM_VDSO_NAME: &str = "[vdso]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmArea {
    pub has_mm: bool,
    pub vm_start: usize,
}

pub const fn arch_vma_name(vma: VmArea, um_vdso_addr: usize) -> Option<&'static str> {
    if vma.has_mm && vma.vm_start == um_vdso_addr {
        Some(UM_VDSO_NAME)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdso_vma_gets_linux_name() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/mem_64.c"
        ));
        assert!(source.contains("return \"[vdso]\";"));

        let vma = VmArea {
            has_mm: true,
            vm_start: 0x7000,
        };
        assert_eq!(arch_vma_name(vma, 0x7000), Some("[vdso]"));
        assert_eq!(arch_vma_name(vma, 0x8000), None);
        assert_eq!(
            arch_vma_name(
                VmArea {
                    has_mm: false,
                    vm_start: 0x7000
                },
                0x7000
            ),
            None
        );
    }
}
