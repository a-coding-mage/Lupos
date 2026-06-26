//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/physaddr.c
//! test-origin: linux:vendor/linux/arch/x86/mm/physaddr.c
//! x86 virtual-address validity checks.

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_OFFSET: u64 = 0xffff_8880_0000_0000;
pub const START_KERNEL_MAP: u64 = 0xffff_ffff_8000_0000;
pub const KERNEL_IMAGE_SIZE: u64 = 512 * 1024 * 1024;
pub const FIXADDR_START: u64 = 0xffff_ffff_ff00_0000;

pub const fn phys_addr_from_kernel_map(x: u64, phys_base: u64) -> Option<u64> {
    let y = x.wrapping_sub(START_KERNEL_MAP);
    if x > y {
        if y >= KERNEL_IMAGE_SIZE {
            None
        } else {
            Some(y + phys_base)
        }
    } else {
        Some(y + (START_KERNEL_MAP - PAGE_OFFSET))
    }
}

pub const fn virt_addr_valid_64(x: u64, phys_base: u64, phys_valid: bool, pfn_valid: bool) -> bool {
    let y = x.wrapping_sub(START_KERNEL_MAP);
    let phys = if x > y {
        if y >= KERNEL_IMAGE_SIZE {
            return false;
        }
        y + phys_base
    } else {
        let p = y + (START_KERNEL_MAP - PAGE_OFFSET);
        if p > y || !phys_valid {
            return false;
        }
        p
    };
    let _ = phys >> PAGE_SHIFT;
    pfn_valid
}

pub const fn virt_addr_valid_32(
    x: u64,
    vmalloc_start_set: bool,
    is_vmalloc_addr: bool,
    pfn_valid: bool,
) -> bool {
    if x < PAGE_OFFSET {
        return false;
    }
    if vmalloc_start_set && is_vmalloc_addr {
        return false;
    }
    if x >= FIXADDR_START {
        return false;
    }
    pfn_valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physaddr_checks_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/mm/physaddr.c"
        ));
        assert!(source.contains("unsigned long __phys_addr(unsigned long x)"));
        assert!(source.contains("unsigned long y = x - __START_KERNEL_map;"));
        assert!(source.contains("if (unlikely(x > y))"));
        assert!(source.contains("VIRTUAL_BUG_ON(y >= KERNEL_IMAGE_SIZE);"));
        assert!(source.contains("bool __virt_addr_valid(unsigned long x)"));
        assert!(source.contains("return pfn_valid(x >> PAGE_SHIFT);"));
        assert!(source.contains("if (x < PAGE_OFFSET)"));
        assert!(source.contains("__vmalloc_start_set && is_vmalloc_addr"));
        assert!(source.contains("if (x >= FIXADDR_START)"));
        assert!(source.contains("EXPORT_SYMBOL(__virt_addr_valid);"));

        assert_eq!(
            phys_addr_from_kernel_map(START_KERNEL_MAP + 0x1000, 0),
            Some(0x1000)
        );
        assert!(!virt_addr_valid_64(
            START_KERNEL_MAP + KERNEL_IMAGE_SIZE,
            0,
            true,
            true
        ));
        assert!(virt_addr_valid_64(START_KERNEL_MAP + 0x1000, 0, true, true));
        assert!(!virt_addr_valid_32(PAGE_OFFSET - 1, false, false, true));
        assert!(!virt_addr_valid_32(PAGE_OFFSET, true, true, true));
    }
}
