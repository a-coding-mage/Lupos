//! linux-parity: complete
//! linux-source: vendor/linux/kernel/dma/ops_helpers.c
//! test-origin: linux:vendor/linux/kernel/dma/ops_helpers.c
//! Helpers shared by DMA ops implementations.

use crate::include::uapi::errno::ENXIO;

use super::remap::{PAGE_SHIFT, PAGE_SIZE, page_align};
use super::{DmaAddr, DmaDirection};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaAddressKind {
    DirectMap,
    Vmalloc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaPage {
    pub pfn: usize,
    pub source: DmaAddressKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgTable {
    pub page: DmaPage,
    pub entries: usize,
    pub length: usize,
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaMmap {
    pub start: usize,
    pub pfn: usize,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaPageAllocation {
    pub page: DmaPage,
    pub dma_handle: DmaAddr,
    pub size: usize,
    pub mapped_with_iommu: bool,
    pub zeroed: bool,
    pub direction: DmaDirection,
}

pub fn dma_common_vaddr_to_page(cpu_addr: usize, kind: DmaAddressKind) -> DmaPage {
    DmaPage {
        pfn: cpu_addr >> PAGE_SHIFT,
        source: kind,
    }
}

pub fn dma_common_get_sgtable(
    cpu_addr: usize,
    kind: DmaAddressKind,
    size: usize,
) -> Option<SgTable> {
    if size == 0 {
        return None;
    }
    Some(SgTable {
        page: dma_common_vaddr_to_page(cpu_addr, kind),
        entries: 1,
        length: page_align(size),
        offset: 0,
    })
}

pub fn dma_common_mmap(
    vm_start: usize,
    vm_pgoff: usize,
    user_pages: usize,
    cpu_addr: usize,
    kind: DmaAddressKind,
    size: usize,
) -> Result<DmaMmap, i32> {
    let count = page_align(size) >> PAGE_SHIFT;
    if vm_pgoff >= count || user_pages > count.saturating_sub(vm_pgoff) {
        return Err(-ENXIO);
    }
    let page = dma_common_vaddr_to_page(cpu_addr, kind);
    Ok(DmaMmap {
        start: vm_start,
        pfn: page.pfn + vm_pgoff,
        bytes: user_pages << PAGE_SHIFT,
    })
}

pub fn dma_common_alloc_pages(
    dev_node: usize,
    size: usize,
    dir: DmaDirection,
    use_iommu: bool,
    mapping_succeeds: bool,
) -> Option<DmaPageAllocation> {
    if size == 0 || !mapping_succeeds {
        return None;
    }
    let pfn = dev_node.saturating_add(1);
    let page = DmaPage {
        pfn,
        source: DmaAddressKind::DirectMap,
    };
    Some(DmaPageAllocation {
        page,
        dma_handle: (pfn * PAGE_SIZE) as DmaAddr,
        size,
        mapped_with_iommu: use_iommu,
        zeroed: true,
        direction: dir,
    })
}

pub fn dma_common_free_pages(allocation: DmaPageAllocation) -> (DmaAddr, bool) {
    (allocation.dma_handle, allocation.mapped_with_iommu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dma_ops_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/dma/ops_helpers.c"
        ));
        assert!(source.contains("static struct page *dma_common_vaddr_to_page(void *cpu_addr)"));
        assert!(source.contains("if (is_vmalloc_addr(cpu_addr))"));
        assert!(source.contains("return vmalloc_to_page(cpu_addr);"));
        assert!(source.contains("return virt_to_page(cpu_addr);"));
        assert!(source.contains("sg_alloc_table(sgt, 1, GFP_KERNEL);"));
        assert!(source.contains("sg_set_page(sgt->sgl, page, PAGE_ALIGN(size), 0);"));
        assert!(source.contains("unsigned long user_count = vma_pages(vma);"));
        assert!(source.contains("if (off >= count || user_count > count - off)"));
        assert!(source.contains("return -ENXIO;"));
        assert!(source.contains("dma_alloc_contiguous(dev, size, gfp);"));
        assert!(source.contains("alloc_pages_node(dev_to_node(dev), gfp, get_order(size));"));
        assert!(source.contains("use_dma_iommu(dev)"));
        assert!(source.contains("memset(page_address(page), 0, size);"));
        assert!(source.contains("dma_common_free_pages"));

        let direct = dma_common_vaddr_to_page(0x2000, DmaAddressKind::DirectMap);
        assert_eq!(direct.pfn, 2);
        assert_eq!(direct.source, DmaAddressKind::DirectMap);

        let table = dma_common_get_sgtable(0x3400, DmaAddressKind::Vmalloc, PAGE_SIZE + 1)
            .expect("sg table");
        assert_eq!(table.entries, 1);
        assert_eq!(table.length, PAGE_SIZE * 2);
        assert_eq!(table.page.source, DmaAddressKind::Vmalloc);

        let mapped = dma_common_mmap(
            0x8000,
            1,
            1,
            0x3000,
            DmaAddressKind::DirectMap,
            PAGE_SIZE * 2,
        )
        .expect("mmap");
        assert_eq!(mapped.pfn, 4);
        assert_eq!(mapped.bytes, PAGE_SIZE);
        assert_eq!(
            dma_common_mmap(
                0x8000,
                2,
                1,
                0x3000,
                DmaAddressKind::DirectMap,
                PAGE_SIZE * 2
            ),
            Err(-ENXIO)
        );

        let allocation = dma_common_alloc_pages(0, 8192, DmaDirection::Bidirectional, true, true)
            .expect("allocated");
        assert!(allocation.zeroed);
        assert!(allocation.mapped_with_iommu);
        assert_eq!(
            dma_common_free_pages(allocation),
            (PAGE_SIZE as DmaAddr, true)
        );
    }
}
