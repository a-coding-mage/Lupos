//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/amd_gart_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/amd_gart_64.c
//! AMD64 GART aperture and IOMMU table model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/amd_gart_64.c

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::arch::x86::mm::paging::{PAGE_SHIFT, PAGE_SIZE};
use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const GPTE_VALID: u32 = 1 << 0;
pub const GPTE_COHERENT: u32 = 1 << 1;
pub const GART_MAX_PHYS_ADDR: u64 = 1 << 40;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GartIommu {
    pub bus_base: u64,
    pub size: u64,
    pub pages: usize,
    pub gatt: Vec<u32>,
    pub bitmap: Vec<bool>,
    pub next_bit: usize,
    pub need_flush: bool,
    pub fullflush: bool,
    pub unmapped_entry: u32,
}

pub const fn gpte_encode(phys: u64) -> Result<u32, i32> {
    if phys >= GART_MAX_PHYS_ADDR || (phys & (PAGE_SIZE - 1)) != 0 {
        return Err(EINVAL);
    }
    Ok(((phys & 0xffff_f000) as u32) | (((phys >> 32) as u32) << 4) | GPTE_VALID | GPTE_COHERENT)
}

pub const fn gpte_decode(entry: u32) -> Option<u64> {
    if (entry & GPTE_VALID) == 0 {
        None
    } else {
        Some(((entry & 0xffff_f000) as u64) | (((entry as u64) & 0xff0) << 28))
    }
}

impl GartIommu {
    pub fn new(bus_base: u64, pages: usize) -> Self {
        Self {
            bus_base,
            size: pages as u64 * PAGE_SIZE,
            pages,
            gatt: vec![0; pages],
            bitmap: vec![false; pages],
            next_bit: 0,
            need_flush: false,
            fullflush: false,
            unmapped_entry: 0,
        }
    }

    pub fn alloc_iommu(&mut self, size_pages: usize) -> Result<usize, i32> {
        if size_pages == 0 || size_pages > self.pages {
            return Err(ENOMEM);
        }

        for pass in 0..2 {
            let start = if pass == 0 { self.next_bit } else { 0 };
            let end = if pass == 0 { self.pages } else { self.next_bit };
            if let Some(bit) = self.find_free_run(start, end, size_pages) {
                for used in &mut self.bitmap[bit..bit + size_pages] {
                    *used = true;
                }
                self.next_bit = (bit + size_pages) % self.pages;
                return Ok(bit);
            }
        }

        Err(ENOMEM)
    }

    pub fn free_iommu(&mut self, bit: usize, size_pages: usize) -> Result<(), i32> {
        if bit
            .checked_add(size_pages)
            .map_or(true, |end| end > self.pages)
        {
            return Err(EINVAL);
        }
        for idx in bit..bit + size_pages {
            self.bitmap[idx] = false;
            self.gatt[idx] = self.unmapped_entry;
        }
        self.need_flush = true;
        Ok(())
    }

    pub fn map_area(&mut self, phys: u64, size_pages: usize) -> Result<u64, i32> {
        if phys >= GART_MAX_PHYS_ADDR {
            return Err(EINVAL);
        }
        let bit = self.alloc_iommu(size_pages)?;
        for idx in 0..size_pages {
            self.gatt[bit + idx] = gpte_encode(phys + idx as u64 * PAGE_SIZE)?;
        }
        self.need_flush = true;
        Ok(self.bus_base + bit as u64 * PAGE_SIZE)
    }

    pub fn unmap(&mut self, dma_addr: u64, size_pages: usize) -> Result<(), i32> {
        if dma_addr < self.bus_base || dma_addr >= self.bus_base + self.size {
            return Err(EINVAL);
        }
        let bit = ((dma_addr - self.bus_base) / PAGE_SIZE) as usize;
        self.free_iommu(bit, size_pages)
    }

    pub const fn flush_needed(&self) -> bool {
        self.need_flush || self.fullflush
    }

    pub fn flush_gart(&mut self) -> bool {
        let needed = self.flush_needed();
        self.need_flush = false;
        self.fullflush = false;
        needed
    }

    fn find_free_run(&self, start: usize, end: usize, len: usize) -> Option<usize> {
        if start >= end || len > end - start {
            return None;
        }
        let mut run = 0;
        let mut base = start;
        for idx in start..end {
            if self.bitmap[idx] {
                run = 0;
                base = idx + 1;
            } else {
                run += 1;
                if run == len {
                    return Some(base);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpte_round_trips_page_address() {
        let entry = gpte_encode(0x1234_5000).unwrap();
        assert_eq!(
            entry & (GPTE_VALID | GPTE_COHERENT),
            GPTE_VALID | GPTE_COHERENT
        );
        assert_eq!(gpte_decode(entry), Some(0x1234_5000));
        assert_eq!(gpte_encode(GART_MAX_PHYS_ADDR), Err(EINVAL));
    }

    #[test]
    fn allocator_wraps_after_end_of_bitmap() {
        let mut gart = GartIommu::new(0x8000_0000, 4);
        assert_eq!(gart.alloc_iommu(3), Ok(0));
        assert_eq!(gart.alloc_iommu(1), Ok(3));
        gart.free_iommu(1, 1).unwrap();
        assert_eq!(gart.alloc_iommu(1), Ok(1));
    }

    #[test]
    fn map_area_fills_gatt_and_dma_address() {
        let mut gart = GartIommu::new(0x8000_0000, 8);
        let dma = gart.map_area(0x2000, 2).unwrap();
        assert_eq!(dma, 0x8000_0000);
        assert_eq!(gpte_decode(gart.gatt[0]), Some(0x2000));
        assert_eq!(gpte_decode(gart.gatt[1]), Some(0x3000));
        assert!(gart.flush_needed());
    }

    #[test]
    fn unmap_clears_entries_and_marks_flush() {
        let mut gart = GartIommu::new(0x8000_0000, 4);
        let dma = gart.map_area(0x4000, 1).unwrap();
        assert!(gart.flush_gart());
        gart.unmap(dma, 1).unwrap();
        assert_eq!(gart.gatt[0], 0);
        assert!(!gart.bitmap[0]);
        assert!(gart.flush_needed());
    }
}
