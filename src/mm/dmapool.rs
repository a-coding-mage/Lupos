//! linux-parity: partial
//! linux-source: vendor/linux/mm/dmapool.c
//! test-origin: linux:vendor/linux/mm/dmapool.c
//! DMA pool sizing, boundary handling, and free-list accounting.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const PAGE_SIZE: usize = 4096;
pub const DMA_BLOCK_BYTES: usize = 16;
pub const POOL_POISON_FREED: u8 = 0xa7;
pub const POOL_POISON_ALLOCATED: u8 = 0xa9;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaBlockHandle {
    pub page: usize,
    pub offset: usize,
    pub dma: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmaPage {
    pub dma: u64,
    pub allocation: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmaPool {
    pub size: usize,
    pub allocation: usize,
    pub boundary: usize,
    pub node: i32,
    pub nr_blocks: usize,
    pub nr_active: usize,
    pub nr_pages: usize,
    pages: Vec<DmaPage>,
    free_list: Vec<DmaBlockHandle>,
    next_dma: u64,
}

impl DmaPool {
    pub fn create_node(
        dev_present: bool,
        size: usize,
        align: usize,
        boundary: usize,
        node: i32,
    ) -> Option<Self> {
        if !dev_present {
            return None;
        }

        let align = if align == 0 {
            1
        } else if !align.is_power_of_two() {
            return None;
        } else {
            align
        };

        if size == 0 || size > i32::MAX as usize {
            return None;
        }

        let size = align_up(size.max(DMA_BLOCK_BYTES), align)?;
        let allocation = size.max(PAGE_SIZE);
        let boundary = if boundary == 0 {
            allocation
        } else {
            if boundary < size || !boundary.is_power_of_two() {
                return None;
            }
            boundary.min(allocation)
        };

        Some(Self {
            size,
            allocation,
            boundary,
            node,
            nr_blocks: 0,
            nr_active: 0,
            nr_pages: 0,
            pages: Vec::new(),
            free_list: Vec::new(),
            next_dma: 0,
        })
    }

    pub fn alloc(&mut self) -> Option<DmaBlockHandle> {
        if self.free_list.is_empty() {
            self.initialise_page();
        }
        let block = self.free_list.pop()?;
        self.nr_active += 1;
        Some(block)
    }

    pub fn free(&mut self, block: DmaBlockHandle) -> Result<(), i32> {
        if self.pool_find_page(block.dma).is_none() {
            return Err(-EINVAL);
        }
        if self.free_list.iter().any(|free| free.dma == block.dma) {
            return Err(-EINVAL);
        }
        if self.nr_active == 0 {
            return Err(-EINVAL);
        }
        self.free_list.push(block);
        self.nr_active -= 1;
        Ok(())
    }

    pub fn free_blocks(&self) -> usize {
        self.free_list.len()
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn pool_find_page(&self, dma: u64) -> Option<&DmaPage> {
        self.pages
            .iter()
            .find(|page| dma >= page.dma && dma - page.dma < self.allocation as u64)
    }

    fn initialise_page(&mut self) {
        let page_index = self.pages.len();
        let page_dma = self.next_dma;
        self.next_dma = self.next_dma.saturating_add(self.allocation as u64);
        self.pages.push(DmaPage {
            dma: page_dma,
            allocation: self.allocation,
        });
        self.nr_pages += 1;

        let mut blocks = Vec::new();
        let mut next_boundary = self.boundary;
        let mut offset = 0usize;

        while offset + self.size <= self.allocation {
            if offset + self.size > next_boundary {
                offset = next_boundary;
                next_boundary = next_boundary.saturating_add(self.boundary);
                continue;
            }
            blocks.push(DmaBlockHandle {
                page: page_index,
                offset,
                dma: page_dma + offset as u64,
            });
            self.nr_blocks += 1;
            offset += self.size;
        }

        for block in blocks.into_iter().rev() {
            self.free_list.push(block);
        }
    }
}

pub const fn align_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    Some((value + align - 1) & !(align - 1))
}

pub const fn dma_pool_blocks_per_allocation(
    size: usize,
    allocation: usize,
    boundary: usize,
) -> usize {
    let mut offset = 0usize;
    let mut next_boundary = boundary;
    let mut blocks = 0usize;

    while offset + size <= allocation {
        if offset + size > next_boundary {
            offset = next_boundary;
            next_boundary += boundary;
        } else {
            blocks += 1;
            offset += size;
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dma_pool_create_and_block_layout_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/dmapool.c"
        ));
        let poison = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/poison.h"
        ));

        assert!(source.contains("if (!dev)"));
        assert!(source.contains("else if (align & (align - 1))"));
        assert!(source.contains("if (size == 0 || size > INT_MAX)"));
        assert!(source.contains("size = ALIGN(size, align);"));
        assert!(source.contains("allocation = max_t(size_t, size, PAGE_SIZE);"));
        assert!(source.contains("if (offset + pool->size > next_boundary)"));
        assert!(source.contains("pool_block_pop(pool)"));
        assert!(source.contains("pool_block_push(pool, block, dma)"));
        assert!(source.contains("pool->nr_active++"));
        assert!(source.contains("pool->nr_active--"));
        assert!(poison.contains("#define\tPOOL_POISON_FREED\t0xa7"));
        assert!(poison.contains("#define\tPOOL_POISON_ALLOCATED\t0xa9"));

        assert!(DmaPool::create_node(false, 16, 16, 0, 0).is_none());
        assert!(DmaPool::create_node(true, 16, 24, 0, 0).is_none());
        assert!(DmaPool::create_node(true, 0, 16, 0, 0).is_none());
        assert!(DmaPool::create_node(true, 64, 64, 32, 0).is_none());

        let pool = DmaPool::create_node(true, 68, 32, PAGE_SIZE, 0).unwrap();
        assert_eq!(pool.size, 96);
        assert_eq!(pool.allocation, PAGE_SIZE);
        assert_eq!(pool.boundary, PAGE_SIZE);
        assert_eq!(
            dma_pool_blocks_per_allocation(pool.size, pool.allocation, pool.boundary),
            42
        );
    }

    #[test]
    fn dma_pool_alloc_free_tracks_active_and_rejects_double_free() {
        let mut pool = DmaPool::create_node(true, 16, 16, 0, 0).unwrap();
        let first = pool.alloc().unwrap();
        let second = pool.alloc().unwrap();

        assert_eq!(first.offset, 0);
        assert_eq!(second.offset, 16);
        assert_eq!(pool.nr_active, 2);
        assert_eq!(pool.nr_pages, 1);
        assert_eq!(pool.nr_blocks, PAGE_SIZE / 16);

        assert_eq!(pool.free(first), Ok(()));
        assert_eq!(pool.nr_active, 1);
        assert_eq!(pool.free(first), Err(-EINVAL));
        assert_eq!(pool.free(second), Ok(()));
        assert_eq!(pool.nr_active, 0);
    }
}
