//! linux-parity: complete
//! linux-source: vendor/linux/mm/cma_debug.c
//! test-origin: linux:vendor/linux/mm/cma_debug.c
//! CMA debugfs counters and debug allocation bookkeeping.

extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::include::uapi::errno::ENOMEM;

pub const CMA_DEBUGFS_ROOT: &str = "cma";
pub const CMA_MAX_RANGES: usize = 8;
pub const BITS_PER_BYTE: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmaMemRange {
    pub base_pfn: usize,
    pub count: usize,
    pub bitmap: Vec<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmaMem {
    pub page_index: usize,
    pub n: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmaDebugArea {
    pub name: String,
    pub count: usize,
    pub available_count: usize,
    pub order_per_bit: u32,
    pub ranges: Vec<CmaMemRange>,
    pub mem_head: Vec<CmaMem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmaDebugfsEntry {
    pub path: String,
    pub mode: u16,
}

impl CmaDebugArea {
    pub fn new(name: &str, count: usize) -> Self {
        Self {
            name: String::from(name),
            count,
            available_count: count,
            order_per_bit: 0,
            ranges: Vec::new(),
            mem_head: Vec::new(),
        }
    }
}

pub fn cma_debugfs_get(value: usize) -> u64 {
    value as u64
}

pub fn cma_used_get(cma: &CmaDebugArea) -> usize {
    cma.count.saturating_sub(cma.available_count)
}

pub fn cma_bitmap_maxno(cma: &CmaDebugArea, range: &CmaMemRange) -> usize {
    range.count >> cma.order_per_bit
}

pub fn cma_maxchunk_get(cma: &CmaDebugArea) -> usize {
    let mut maxchunk = 0usize;
    for range in &cma.ranges {
        let bitmap_maxno = cma_bitmap_maxno(cma, range).min(range.bitmap.len());
        let mut start = 0usize;
        while start < bitmap_maxno {
            while start < bitmap_maxno && range.bitmap[start] {
                start += 1;
            }
            let mut end = start;
            while end < bitmap_maxno && !range.bitmap[end] {
                end += 1;
            }
            maxchunk = maxchunk.max(end.saturating_sub(start));
            start = end.saturating_add(1);
        }
    }
    maxchunk << cma.order_per_bit
}

pub fn cma_add_to_cma_mem_list(cma: &mut CmaDebugArea, mem: CmaMem) {
    cma.mem_head.insert(0, mem);
}

pub fn cma_get_entry_from_list(cma: &mut CmaDebugArea) -> Option<CmaMem> {
    if cma.mem_head.is_empty() {
        None
    } else {
        Some(cma.mem_head.remove(0))
    }
}

pub fn cma_alloc_mem(cma: &mut CmaDebugArea, count: usize) -> Result<(), i32> {
    cma_alloc_mem_with_alloc(cma, count, true)
}

pub fn cma_alloc_mem_with_alloc(
    cma: &mut CmaDebugArea,
    count: usize,
    metadata_available: bool,
) -> Result<(), i32> {
    if !metadata_available {
        return Err(-ENOMEM);
    }
    if count == 0 || count > cma.available_count {
        return Err(-ENOMEM);
    }
    let page_index = cma.count - cma.available_count;
    cma.available_count -= count;
    cma_add_to_cma_mem_list(
        cma,
        CmaMem {
            page_index,
            n: count,
        },
    );
    Ok(())
}

pub fn cma_alloc_write(cma: &mut CmaDebugArea, val: u64) -> Result<(), i32> {
    cma_alloc_mem(cma, val as usize)
}

pub fn cma_free_mem(cma: &mut CmaDebugArea, mut count: usize) -> usize {
    let mut released = 0usize;
    while count != 0 {
        let Some(mut mem) = cma_get_entry_from_list(cma) else {
            break;
        };

        if mem.n <= count {
            count -= mem.n;
            released += mem.n;
        } else if cma.order_per_bit == 0 {
            mem.page_index += count;
            mem.n -= count;
            released += count;
            count = 0;
            cma_add_to_cma_mem_list(cma, mem);
        } else {
            cma_add_to_cma_mem_list(cma, mem);
            break;
        }
    }
    cma.available_count = cma.available_count.saturating_add(released).min(cma.count);
    released
}

pub fn cma_free_write(cma: &mut CmaDebugArea, val: u64) -> i32 {
    cma_free_mem(cma, val as usize);
    0
}

pub fn cma_debugfs_file_names() -> [&'static str; 8] {
    [
        "alloc",
        "free",
        "count",
        "order_per_bit",
        "used",
        "maxchunk",
        "ranges",
        "bitmap",
    ]
}

pub fn cma_range_bitmap_elements(cma: &CmaDebugArea, range: &CmaMemRange) -> usize {
    let bits_per_u32 = BITS_PER_BYTE * core::mem::size_of::<u32>();
    cma_bitmap_maxno(cma, range).div_ceil(bits_per_u32)
}

pub fn cma_debugfs_add_one_plan(cma: &CmaDebugArea) -> Vec<CmaDebugfsEntry> {
    let mut entries = Vec::new();
    let root = cma.name.clone();
    entries.push(CmaDebugfsEntry {
        path: root.clone(),
        mode: 0o555,
    });

    for (name, mode) in [
        ("alloc", 0o200),
        ("free", 0o200),
        ("count", 0o444),
        ("order_per_bit", 0o444),
        ("used", 0o444),
        ("maxchunk", 0o444),
    ] {
        entries.push(CmaDebugfsEntry {
            path: format!("{root}/{name}"),
            mode,
        });
    }

    entries.push(CmaDebugfsEntry {
        path: format!("{root}/ranges"),
        mode: 0o555,
    });
    for (index, _) in cma.ranges.iter().enumerate() {
        entries.push(CmaDebugfsEntry {
            path: format!("{root}/ranges/{index}"),
            mode: 0o555,
        });
        entries.push(CmaDebugfsEntry {
            path: format!("{root}/ranges/{index}/base_pfn"),
            mode: 0o444,
        });
        entries.push(CmaDebugfsEntry {
            path: format!("{root}/ranges/{index}/bitmap"),
            mode: 0o444,
        });
    }

    entries.push(CmaDebugfsEntry {
        path: format!("{root}/base_pfn -> ranges/0/base_pfn"),
        mode: 0o777,
    });
    entries.push(CmaDebugfsEntry {
        path: format!("{root}/bitmap -> ranges/0/bitmap"),
        mode: 0o777,
    });
    entries
}

pub fn cma_debugfs_init_plan(areas: &[CmaDebugArea]) -> Vec<CmaDebugfsEntry> {
    let mut entries = Vec::new();
    entries.push(CmaDebugfsEntry {
        path: CMA_DEBUGFS_ROOT.into(),
        mode: 0o555,
    });
    for area in areas {
        for mut entry in cma_debugfs_add_one_plan(area) {
            entry.path = format!("{CMA_DEBUGFS_ROOT}/{}", entry.path);
            entries.push(entry);
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cma_debugfs_rules_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/cma_debug.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/cma.h"
        ));
        assert!(source.contains("DEFINE_DEBUGFS_ATTRIBUTE(cma_debugfs_fops"));
        assert!(source.contains("*val = cma->count - cma->available_count;"));
        assert!(source.contains("for_each_clear_bitrange(start, end, cmr->bitmap, bitmap_maxno)"));
        assert!(source.contains("maxchunk = max(end - start, maxchunk);"));
        assert!(source.contains("*val = (u64)maxchunk << cma->order_per_bit;"));
        assert!(source.contains("hlist_add_head(&mem->node, &cma->mem_head);"));
        assert!(source.contains("cma_release(cma, mem->p, mem->n);"));
        assert!(source.contains("cma_release(cma, mem->p, count);"));
        assert!(source.contains("return cma_free_mem(cma, pages);"));
        assert!(source.contains("mem = kzalloc_obj(*mem);"));
        assert!(source.contains("p = cma_alloc(cma, count, 0, false);"));
        assert!(source.contains("return cma_alloc_mem(cma, pages);"));
        assert!(source.contains("tmp = debugfs_create_dir(cma->name, root_dentry);"));
        assert!(source.contains("debugfs_create_file(\"alloc\", 0200"));
        assert!(source.contains("debugfs_create_dir(\"ranges\", tmp);"));
        assert!(source.contains("snprintf(rdirname, sizeof(rdirname), \"%d\", r);"));
        assert!(source.contains("debugfs_create_file(\"base_pfn\", 0444"));
        assert!(source.contains("debugfs_create_u32_array(\"bitmap\", 0444"));
        assert!(
            source.contains("debugfs_create_symlink(\"base_pfn\", tmp, \"ranges/0/base_pfn\");")
        );
        assert!(source.contains("debugfs_create_symlink(\"bitmap\", tmp, \"ranges/0/bitmap\");"));
        assert!(source.contains("cma_debugfs_root = debugfs_create_dir(\"cma\", NULL);"));
        assert!(source.contains("for (i = 0; i < cma_area_count; i++)"));
        assert!(source.contains("late_initcall(cma_debugfs_init);"));
        assert!(header.contains("#define CMA_MAX_RANGES 8"));
        assert!(header.contains("static inline unsigned long cma_bitmap_maxno"));

        let mut cma = CmaDebugArea::new("unit", 16);
        cma.order_per_bit = 1;
        cma.ranges.push(CmaMemRange {
            base_pfn: 0,
            count: 16,
            bitmap: alloc::vec![false, false, true, false, false, false, true, false],
        });
        assert_eq!(cma_used_get(&cma), 0);
        assert_eq!(cma_maxchunk_get(&cma), 6);
        assert_eq!(cma_alloc_mem(&mut cma, 4), Ok(()));
        assert_eq!(cma_alloc_mem_with_alloc(&mut cma, 1, false), Err(-ENOMEM));
        assert_eq!(cma_used_get(&cma), 4);
        assert_eq!(cma_free_mem(&mut cma, 2), 0);
        cma.order_per_bit = 0;
        assert_eq!(cma_free_mem(&mut cma, 2), 2);
        assert_eq!(cma_alloc_write(&mut cma, 1), Ok(()));
        assert_eq!(cma_free_write(&mut cma, 1), 0);
        assert!(cma_debugfs_file_names().contains(&"maxchunk"));
        assert_eq!(cma_range_bitmap_elements(&cma, &cma.ranges[0]), 1);

        let plan = cma_debugfs_init_plan(&[cma.clone()]);
        assert!(plan.contains(&CmaDebugfsEntry {
            path: "cma/unit/alloc".into(),
            mode: 0o200,
        }));
        assert!(plan.contains(&CmaDebugfsEntry {
            path: "cma/unit/ranges/0/base_pfn".into(),
            mode: 0o444,
        }));
        assert!(plan.contains(&CmaDebugfsEntry {
            path: "cma/unit/bitmap -> ranges/0/bitmap".into(),
            mode: 0o777,
        }));
    }
}
