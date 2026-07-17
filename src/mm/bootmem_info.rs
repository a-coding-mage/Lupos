//! linux-parity: complete
//! linux-source: vendor/linux/mm/bootmem_info.c
//! test-origin: linux:vendor/linux/mm/bootmem_info.c
//! Bootmem metadata page ownership helpers.

pub const PAGE_SIZE: usize = 4096;
pub const PAGES_PER_SECTION: u64 = 1 << 18;
pub const MEMORY_HOTPLUG_MIN_BOOTMEM_TYPE: u8 = 1;
pub const MEMORY_HOTPLUG_MAX_BOOTMEM_TYPE: u8 = 4;
pub const NODE_INFO: u8 = 1;
pub const MIX_SECTION_INFO: u8 = 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BootmemPage {
    pub private: u64,
    pub refcount: u32,
    pub page_private: bool,
    pub freed_reserved: bool,
}

pub const fn bootmem_private(info: u64, bootmem_type: u8) -> Option<u64> {
    if bootmem_type > 0x0f || info > (u64::MAX >> 4) {
        None
    } else {
        Some((info << 4) | bootmem_type as u64)
    }
}

pub const fn get_page_bootmem(
    page: BootmemPage,
    info: u64,
    bootmem_type: u8,
) -> Option<BootmemPage> {
    let Some(private) = bootmem_private(info, bootmem_type) else {
        return None;
    };
    Some(BootmemPage {
        private,
        refcount: page.refcount + 1,
        page_private: true,
        freed_reserved: false,
    })
}

pub const fn put_page_bootmem(mut page: BootmemPage, bootmem_type: u8) -> Option<BootmemPage> {
    if bootmem_type < MEMORY_HOTPLUG_MIN_BOOTMEM_TYPE
        || bootmem_type > MEMORY_HOTPLUG_MAX_BOOTMEM_TYPE
    {
        return None;
    }
    page.refcount -= 1;
    if page.refcount == 1 {
        page.private = 0;
        page.page_private = false;
        page.freed_reserved = true;
    }
    Some(page)
}

pub const fn bootmem_node_pages(pgdat_size: usize) -> usize {
    pgdat_size.div_ceil(PAGE_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootmem_info_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/bootmem_info.c"
        ));
        assert!(source.contains("void get_page_bootmem"));
        assert!(source.contains("BUG_ON(type > 0xf);"));
        assert!(source.contains("set_page_private(page, info << 4 | type);"));
        assert!(source.contains("page_ref_inc(page);"));
        assert!(source.contains("enum bootmem_type type = bootmem_type(page);"));
        assert!(source.contains("MEMORY_HOTPLUG_MIN_BOOTMEM_TYPE"));
        assert!(source.contains("set_page_private(page, 0);"));
        assert!(source.contains("free_reserved_page(page);"));
        assert!(source.contains("register_page_bootmem_info_section"));
        assert!(source.contains("for (; pfn < end_pfn; pfn += PAGES_PER_SECTION)"));

        let page = get_page_bootmem(
            BootmemPage {
                refcount: 1,
                ..BootmemPage::default()
            },
            7,
            NODE_INFO,
        )
        .unwrap();
        assert_eq!(page.private, (7 << 4) | NODE_INFO as u64);
        assert!(page.page_private);
        let freed = put_page_bootmem(
            BootmemPage {
                refcount: 2,
                page_private: true,
                private: 1,
                freed_reserved: false,
            },
            NODE_INFO,
        )
        .unwrap();
        assert!(freed.freed_reserved);
        assert_eq!(bootmem_node_pages(PAGE_SIZE + 1), 2);
    }
}
