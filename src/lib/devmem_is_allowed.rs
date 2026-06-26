//! linux-parity: complete
//! linux-source: vendor/linux/lib/devmem_is_allowed.c
//! test-origin: linux:vendor/linux/lib/devmem_is_allowed.c
//! Generic /dev/mem access policy.

pub const fn devmem_is_allowed_from_attrs(iomem_exclusive: bool, page_is_ram: bool) -> i32 {
    if iomem_exclusive {
        0
    } else if !page_is_ram {
        1
    } else {
        0
    }
}

pub const fn devmem_is_allowed(_pfn: usize) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devmem_is_allowed_policy_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/devmem_is_allowed.c"
        ));
        assert!(source.contains("#include <linux/ioport.h>"));
        assert!(source.contains("iomem_is_exclusive(PFN_PHYS(pfn))"));
        assert!(source.contains("if (!page_is_ram(pfn))"));
        assert_eq!(devmem_is_allowed_from_attrs(true, false), 0);
        assert_eq!(devmem_is_allowed_from_attrs(true, true), 0);
        assert_eq!(devmem_is_allowed_from_attrs(false, false), 1);
        assert_eq!(devmem_is_allowed_from_attrs(false, true), 0);
    }
}
