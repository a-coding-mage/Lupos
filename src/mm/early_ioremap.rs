//! linux-parity: complete
//! linux-source: vendor/linux/mm/early_ioremap.c
//! test-origin: linux:vendor/linux/mm/early_ioremap.c
//! Generic early-ioremap slot accounting and copy chunking.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);
pub const NR_FIX_BTMAPS: usize = 64;
pub const FIX_BTMAPS_SLOTS: usize = 8;
pub const MAX_MAP_CHUNK: usize = NR_FIX_BTMAPS << PAGE_SHIFT;
pub const FIX_BTMAP_SLOT_BASE: usize = 0xffff_ff80_0000_0000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EarlyProt {
    Io,
    Normal,
    ReadOnly,
    Custom(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixmapOp {
    EarlySet,
    LateSet,
    EarlyClear,
    LateClear,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EarlyMapping {
    pub slot: usize,
    pub virt: usize,
    pub phys: u64,
    pub aligned_phys: u64,
    pub requested_size: usize,
    pub aligned_size: usize,
    pub nrpages: usize,
    pub prot: EarlyProt,
    pub after_paging_init: bool,
    pub system_running_warning: bool,
    pub fixmap_op: FixmapOp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EarlyCopyChunk {
    pub aligned_src: u64,
    pub slop: usize,
    pub map_size: usize,
    pub copy_len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EarlyUnmap {
    pub mapping: EarlyMapping,
    pub offset: usize,
    pub nrpages: usize,
    pub fixmap_op: FixmapOp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EarlyNoMmuOp {
    Ioremap { virt: usize },
    Memremap { virt: usize },
    MemremapRo { virt: usize },
    Iounmap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EarlyIoremapState {
    prev_map: [Option<EarlyMapping>; FIX_BTMAPS_SLOTS],
    prev_size: [usize; FIX_BTMAPS_SLOTS],
    slot_virt: [usize; FIX_BTMAPS_SLOTS],
    pub after_paging_init: bool,
    pub debug: bool,
    pub system_running: bool,
}

impl Default for EarlyIoremapState {
    fn default() -> Self {
        let mut state = Self {
            prev_map: [None; FIX_BTMAPS_SLOTS],
            prev_size: [0; FIX_BTMAPS_SLOTS],
            slot_virt: [0; FIX_BTMAPS_SLOTS],
            after_paging_init: false,
            debug: false,
            system_running: false,
        };
        state.setup();
        state
    }
}

impl EarlyIoremapState {
    pub fn setup(&mut self) -> usize {
        let mut already_mapped = 0;
        for i in 0..FIX_BTMAPS_SLOTS {
            if self.prev_map[i].is_some() {
                already_mapped += 1;
            }
            self.slot_virt[i] = FIX_BTMAP_SLOT_BASE + NR_FIX_BTMAPS * PAGE_SIZE * i;
        }
        already_mapped
    }

    pub fn debug_setup(&mut self) -> i32 {
        self.debug = true;
        0
    }

    pub fn reset(&mut self) {
        self.after_paging_init = true;
    }

    pub fn early_ioremap(&mut self, phys_addr: u64, size: usize) -> Option<EarlyMapping> {
        self.__early_ioremap(
            phys_addr,
            size,
            early_memremap_pgprot_adjust(phys_addr, size, EarlyProt::Io),
        )
    }

    pub fn early_memremap(&mut self, phys_addr: u64, size: usize) -> Option<EarlyMapping> {
        self.__early_ioremap(
            phys_addr,
            size,
            early_memremap_pgprot_adjust(phys_addr, size, EarlyProt::Normal),
        )
    }

    pub fn early_memremap_ro(&mut self, phys_addr: u64, size: usize) -> Option<EarlyMapping> {
        self.__early_ioremap(
            phys_addr,
            size,
            early_memremap_pgprot_adjust(phys_addr, size, EarlyProt::ReadOnly),
        )
    }

    pub fn early_memremap_prot(
        &mut self,
        phys_addr: u64,
        size: usize,
        prot: usize,
    ) -> Option<EarlyMapping> {
        self.__early_ioremap(phys_addr, size, EarlyProt::Custom(prot))
    }

    pub fn early_iounmap(&mut self, virt: usize, size: usize) -> Result<EarlyUnmap, i32> {
        let slot = self
            .prev_map
            .iter()
            .position(|mapping| mapping.map(|m| m.virt) == Some(virt))
            .ok_or(-EINVAL)?;

        if self.prev_size[slot] != size {
            return Err(-EINVAL);
        }
        if virt < FIX_BTMAP_SLOT_BASE {
            return Err(-EINVAL);
        }

        let mapping = self.prev_map[slot].take().ok_or(-EINVAL)?;
        let offset = offset_in_page(virt as u64);
        let nrpages = page_align(offset + size) >> PAGE_SHIFT;
        Ok(EarlyUnmap {
            mapping,
            offset,
            nrpages,
            fixmap_op: if self.after_paging_init {
                FixmapOp::LateClear
            } else {
                FixmapOp::EarlyClear
            },
        })
    }

    pub fn early_memunmap(&mut self, virt: usize, size: usize) -> Result<EarlyUnmap, i32> {
        self.early_iounmap(virt, size)
    }

    pub fn leak_count(&self) -> usize {
        self.prev_map
            .iter()
            .filter(|mapping| mapping.is_some())
            .count()
    }

    pub fn check_early_ioremap_leak(&self) -> i32 {
        if self.leak_count() == 0 { 0 } else { 1 }
    }

    pub fn prev_size_at(&self, slot: usize) -> Option<usize> {
        self.prev_size.get(slot).copied()
    }

    pub fn slot_virt_at(&self, slot: usize) -> Option<usize> {
        self.slot_virt.get(slot).copied()
    }

    fn __early_ioremap(
        &mut self,
        phys_addr: u64,
        size: usize,
        prot: EarlyProt,
    ) -> Option<EarlyMapping> {
        let slot = self.prev_map.iter().position(Option::is_none)?;
        if size == 0 {
            return None;
        }
        let last_addr = phys_addr.checked_add(size as u64 - 1)?;
        if last_addr < phys_addr {
            return None;
        }

        self.prev_size[slot] = size;
        let offset = phys_addr as usize & !PAGE_MASK;
        let aligned_phys = phys_addr & PAGE_MASK as u64;
        let aligned_size = page_align((last_addr as usize + 1).wrapping_sub(aligned_phys as usize));
        let nrpages = aligned_size >> PAGE_SHIFT;
        if nrpages > NR_FIX_BTMAPS {
            return None;
        }

        let mapping = EarlyMapping {
            slot,
            virt: self.slot_virt[slot] + offset,
            phys: phys_addr,
            aligned_phys,
            requested_size: size,
            aligned_size,
            nrpages,
            prot,
            after_paging_init: self.after_paging_init,
            system_running_warning: self.system_running,
            fixmap_op: if self.after_paging_init {
                FixmapOp::LateSet
            } else {
                FixmapOp::EarlySet
            },
        };
        self.prev_map[slot] = Some(mapping);
        Some(mapping)
    }
}

pub const fn early_memremap_pgprot_adjust(
    _phys_addr: u64,
    _size: usize,
    prot: EarlyProt,
) -> EarlyProt {
    prot
}

pub const fn page_align(size: usize) -> usize {
    (size + PAGE_SIZE - 1) & PAGE_MASK
}

pub const fn offset_in_page(addr: u64) -> usize {
    addr as usize & (PAGE_SIZE - 1)
}

pub fn copy_from_early_mem_plan(src: u64, size: usize) -> Result<Vec<EarlyCopyChunk>, i32> {
    copy_from_early_mem_plan_with_slots(src, size, 1)
}

pub fn copy_from_early_mem_plan_with_slots(
    mut src: u64,
    mut size: usize,
    available_slots: usize,
) -> Result<Vec<EarlyCopyChunk>, i32> {
    if size != 0 && available_slots == 0 {
        return Err(-ENOMEM);
    }
    let mut chunks = Vec::new();

    while size != 0 {
        let slop = offset_in_page(src);
        let mut clen = size;
        if clen > MAX_MAP_CHUNK - slop {
            clen = MAX_MAP_CHUNK - slop;
        }
        if clen == 0 {
            return Err(-ENOMEM);
        }
        chunks.push(EarlyCopyChunk {
            aligned_src: src & PAGE_MASK as u64,
            slop,
            map_size: clen + slop,
            copy_len: clen,
        });
        src += clen as u64;
        size -= clen;
    }

    Ok(chunks)
}

pub const fn early_ioremap_nommu(phys_addr: u64, _size: usize) -> EarlyNoMmuOp {
    EarlyNoMmuOp::Ioremap {
        virt: phys_addr as usize,
    }
}

pub const fn early_memremap_nommu(phys_addr: u64, _size: usize) -> EarlyNoMmuOp {
    EarlyNoMmuOp::Memremap {
        virt: phys_addr as usize,
    }
}

pub const fn early_memremap_ro_nommu(phys_addr: u64, _size: usize) -> EarlyNoMmuOp {
    EarlyNoMmuOp::MemremapRo {
        virt: phys_addr as usize,
    }
}

pub const fn early_iounmap_nommu(_addr: usize, _size: usize) -> EarlyNoMmuOp {
    EarlyNoMmuOp::Iounmap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_ioremap_slot_arithmetic_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/early_ioremap.c"
        ));
        let fixmap = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/fixmap.h"
        ));

        assert!(source.contains("static void __init __iomem *"));
        assert!(
            source.contains("early_param(\"early_ioremap_debug\", early_ioremap_debug_setup);")
        );
        assert!(source.contains("pgprot_t __init __weak early_memremap_pgprot_adjust"));
        assert!(source.contains("void __init early_ioremap_reset(void)"));
        assert!(source.contains("after_paging_init = 1;"));
        assert!(source.contains("void __init early_ioremap_setup(void)"));
        assert!(source.contains("WARN_ON_ONCE(prev_map[i]);"));
        assert!(
            source.contains("slot_virt[i] = __fix_to_virt(FIX_BTMAP_BEGIN - NR_FIX_BTMAPS*i);")
        );
        assert!(source.contains("static int __init check_early_ioremap_leak(void)"));
        assert!(source.contains("slot = -1;"));
        assert!(source.contains("WARN_ON(system_state >= SYSTEM_RUNNING);"));
        assert!(source.contains("last_addr = phys_addr + size - 1;"));
        assert!(source.contains("if (WARN_ON(!size || last_addr < phys_addr))"));
        assert!(source.contains("prev_size[slot] = size;"));
        assert!(source.contains("offset = offset_in_page(phys_addr);"));
        assert!(source.contains("phys_addr &= PAGE_MASK;"));
        assert!(source.contains("nrpages = size >> PAGE_SHIFT;"));
        assert!(source.contains("if (WARN_ON(nrpages > NR_FIX_BTMAPS))"));
        assert!(source.contains("if (after_paging_init)"));
        assert!(source.contains("__late_set_fixmap(idx, phys_addr, prot);"));
        assert!(source.contains("__early_set_fixmap(idx, phys_addr, prot);"));
        assert!(source.contains("prev_map[slot] = (void __iomem *)(offset + slot_virt[slot]);"));
        assert!(
            source.contains("void __init early_iounmap(void __iomem *addr, unsigned long size)")
        );
        assert!(source.contains("if (prev_map[i] == addr)"));
        assert!(source.contains("if (WARN(prev_size[slot] != size,"));
        assert!(source.contains("if (WARN_ON(virt_addr < fix_to_virt(FIX_BTMAP_BEGIN)))"));
        assert!(source.contains("nrpages = PAGE_ALIGN(offset + size) >> PAGE_SHIFT;"));
        assert!(source.contains("__late_clear_fixmap(idx);"));
        assert!(source.contains("__early_set_fixmap(idx, 0, FIXMAP_PAGE_CLEAR);"));
        assert!(source.contains("prev_map[slot] = NULL;"));
        assert!(source.contains("early_memremap_pgprot_adjust(phys_addr, size,"));
        assert!(source.contains("#define MAX_MAP_CHUNK\t(NR_FIX_BTMAPS << PAGE_SHIFT)"));
        assert!(source.contains("if (!p)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("#else /* CONFIG_MMU */"));
        assert!(source.contains("return (__force void __iomem *)phys_addr;"));
        assert!(source.contains("void __init early_memunmap(void *addr, unsigned long size)"));
        assert!(fixmap.contains("#define NR_FIX_BTMAPS\t\t64"));
        assert!(fixmap.contains("#define FIX_BTMAPS_SLOTS\t8"));
        assert!(fixmap.contains("FIX_BTMAP_BEGIN = FIX_BTMAP_END + TOTAL_FIX_BTMAPS - 1"));

        let mut state = EarlyIoremapState::default();
        let mapping = state.early_ioremap(0x1234, 0x2000).unwrap();
        assert_eq!(mapping.slot, 0);
        assert_eq!(mapping.virt, FIX_BTMAP_SLOT_BASE + 0x234);
        assert_eq!(mapping.aligned_phys, 0x1000);
        assert_eq!(mapping.aligned_size, 0x3000);
        assert_eq!(mapping.nrpages, 3);
        assert_eq!(mapping.fixmap_op, FixmapOp::EarlySet);
        assert_eq!(state.leak_count(), 1);
        assert_eq!(state.check_early_ioremap_leak(), 1);
        let unmap = state.early_iounmap(mapping.virt, 0x2000).unwrap();
        assert_eq!(unmap.mapping, mapping);
        assert_eq!(unmap.offset, 0x234);
        assert_eq!(unmap.nrpages, 3);
        assert_eq!(unmap.fixmap_op, FixmapOp::EarlyClear);
        assert_eq!(state.leak_count(), 0);
        assert_eq!(state.check_early_ioremap_leak(), 0);

        assert!(state.early_ioremap(0x1000, 0).is_none());
        let too_large = (NR_FIX_BTMAPS + 1) * PAGE_SIZE;
        assert!(state.early_ioremap(0x1000, too_large).is_none());
        assert_eq!(state.prev_size_at(0), Some(too_large));
    }

    #[test]
    fn setup_slots_late_unmap_and_wrappers_match_linux() {
        let mut state = EarlyIoremapState::default();
        assert_eq!(state.debug_setup(), 0);
        assert!(state.debug);
        assert_eq!(state.slot_virt_at(0), Some(FIX_BTMAP_SLOT_BASE));
        assert_eq!(
            state.slot_virt_at(1),
            Some(FIX_BTMAP_SLOT_BASE + MAX_MAP_CHUNK)
        );
        assert_eq!(state.setup(), 0);

        let first = state.early_ioremap(0x1000, PAGE_SIZE).unwrap();
        let second = state.early_memremap(0x2234, PAGE_SIZE).unwrap();
        assert_eq!(first.slot, 0);
        assert_eq!(second.slot, 1);
        assert_eq!(second.virt, FIX_BTMAP_SLOT_BASE + MAX_MAP_CHUNK + 0x234);
        assert_eq!(second.prot, EarlyProt::Normal);
        assert_eq!(
            early_memremap_pgprot_adjust(0x2000, PAGE_SIZE, EarlyProt::ReadOnly),
            EarlyProt::ReadOnly
        );

        assert_eq!(
            state.early_iounmap(second.virt, PAGE_SIZE + 1),
            Err(-EINVAL)
        );
        state.reset();
        let late = state.early_memunmap(second.virt, PAGE_SIZE).unwrap();
        assert_eq!(late.fixmap_op, FixmapOp::LateClear);
        assert_eq!(late.nrpages, 2);

        state.system_running = true;
        let warned = state.early_memremap_ro(0x3000, PAGE_SIZE).unwrap();
        assert_eq!(warned.fixmap_op, FixmapOp::LateSet);
        assert!(warned.system_running_warning);
    }

    #[test]
    fn copy_from_early_mem_uses_linux_chunk_limit() {
        let chunks = copy_from_early_mem_plan(0x1234, MAX_MAP_CHUNK).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].aligned_src, 0x1000);
        assert_eq!(chunks[0].slop, 0x234);
        assert_eq!(chunks[0].copy_len, MAX_MAP_CHUNK - 0x234);
        assert_eq!(chunks[1].copy_len, 0x234);
        assert_eq!(
            copy_from_early_mem_plan_with_slots(0x1000, PAGE_SIZE, 0),
            Err(-ENOMEM)
        );
        assert!(copy_from_early_mem_plan(0x1000, 0).unwrap().is_empty());
    }

    #[test]
    fn nommu_early_ioremap_returns_physical_addresses() {
        assert_eq!(
            early_ioremap_nommu(0x1234, PAGE_SIZE),
            EarlyNoMmuOp::Ioremap { virt: 0x1234 }
        );
        assert_eq!(
            early_memremap_nommu(0x2234, PAGE_SIZE),
            EarlyNoMmuOp::Memremap { virt: 0x2234 }
        );
        assert_eq!(
            early_memremap_ro_nommu(0x3234, PAGE_SIZE),
            EarlyNoMmuOp::MemremapRo { virt: 0x3234 }
        );
        assert_eq!(
            early_iounmap_nommu(0x1234, PAGE_SIZE),
            EarlyNoMmuOp::Iounmap
        );
    }
}
