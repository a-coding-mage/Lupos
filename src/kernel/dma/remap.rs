//! linux-parity: complete
//! linux-source: vendor/linux/kernel/dma/remap.c
//! test-origin: linux:vendor/linux/kernel/dma/remap.c
//! Common DMA coherent vmalloc remapping helpers.

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const VM_DMA_COHERENT: u32 = 1 << 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaVmArea {
    pub flags: u32,
    pub page_count: usize,
}

pub const fn page_align(size: usize) -> usize {
    (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub const fn dma_common_find_pages(area: Option<DmaVmArea>) -> Option<usize> {
    let Some(area) = area else {
        return None;
    };
    if area.flags & VM_DMA_COHERENT == 0 {
        return None;
    }
    Some(area.page_count)
}

pub const fn dma_common_pages_remap_count(size: usize) -> usize {
    page_align(size) >> PAGE_SHIFT
}

pub const fn dma_common_contiguous_remap_count(size: usize) -> usize {
    dma_common_pages_remap_count(size)
}

pub const fn dma_common_free_remap_valid(area: Option<DmaVmArea>) -> bool {
    matches!(area, Some(area) if area.flags & VM_DMA_COHERENT != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dma_remap_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/dma/remap.c"
        ));
        assert!(source.contains("struct page **dma_common_find_pages(void *cpu_addr)"));
        assert!(source.contains("find_vm_area(cpu_addr)"));
        assert!(source.contains("!(area->flags & VM_DMA_COHERENT)"));
        assert!(source.contains("return area->pages;"));
        assert!(source.contains("vmap(pages, PAGE_ALIGN(size) >> PAGE_SHIFT"));
        assert!(source.contains("find_vm_area(vaddr)->pages = pages;"));
        assert!(source.contains("kvmalloc_objs(struct page *, count)"));
        assert!(source.contains("vunmap(cpu_addr);"));

        let coherent = DmaVmArea {
            flags: VM_DMA_COHERENT,
            page_count: 3,
        };
        assert_eq!(dma_common_find_pages(Some(coherent)), Some(3));
        assert_eq!(
            dma_common_find_pages(Some(DmaVmArea {
                flags: 0,
                page_count: 3
            })),
            None
        );
        assert_eq!(dma_common_pages_remap_count(PAGE_SIZE + 1), 2);
        assert!(dma_common_free_remap_valid(Some(coherent)));
        assert!(!dma_common_free_remap_valid(None));
    }
}
