//! linux-parity: partial
//! linux-source: vendor/linux/mm/execmem.c
//! test-origin: linux:vendor/linux/mm/execmem.c
//! Executable-memory range validation, fallback, and ROX-cache bookkeeping.

use crate::mm::vmalloc::{VMALLOC_END, VMALLOC_START};

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_KERNEL: u64 = 1;
pub const PAGE_KERNEL_EXEC: u64 = 2;

pub const EXECMEM_DEFAULT: usize = 0;
pub const EXECMEM_MODULE_TEXT: usize = EXECMEM_DEFAULT;
pub const EXECMEM_KPROBES: usize = 1;
pub const EXECMEM_FTRACE: usize = 2;
pub const EXECMEM_BPF: usize = 3;
pub const EXECMEM_MODULE_DATA: usize = 4;
pub const EXECMEM_TYPE_MAX: usize = 5;

pub const EXECMEM_KASAN_SHADOW: u32 = 1 << 0;
pub const EXECMEM_ROX_CACHE: u32 = 1 << 1;
pub const VM_FLUSH_RESET_PERMS: u64 = 1 << 0;
pub const VM_DEFER_KMEMLEAK: u64 = 1 << 1;
pub const PENDING_FREE_MASK: usize = 1 << (12 - 1);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExecmemRange {
    pub start: usize,
    pub end: usize,
    pub fallback_start: usize,
    pub fallback_end: usize,
    pub pgprot: u64,
    pub alignment: usize,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecmemInfo {
    pub ranges: [ExecmemRange; EXECMEM_TYPE_MAX],
}

impl Default for ExecmemInfo {
    fn default() -> Self {
        Self {
            ranges: [ExecmemRange::default(); EXECMEM_TYPE_MAX],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecmemAllocation {
    pub typ: usize,
    pub start: usize,
    pub size: usize,
    pub pgprot: u64,
    pub vm_flags: u64,
    pub used_fallback: bool,
    pub use_cache: bool,
}

pub fn default_execmem_info() -> ExecmemInfo {
    let mut info = ExecmemInfo::default();
    info.ranges[EXECMEM_DEFAULT] = ExecmemRange {
        start: VMALLOC_START as usize,
        end: VMALLOC_END as usize - 1,
        fallback_start: 0,
        fallback_end: 0,
        pgprot: PAGE_KERNEL_EXEC,
        alignment: 1,
        flags: 0,
    };
    info
}

pub fn execmem_prepare(mut info: ExecmemInfo, arch_has_rox: bool) -> Option<ExecmemInfo> {
    let default = info.ranges[EXECMEM_DEFAULT];
    if default.alignment == 0 || default.start == 0 || default.end == 0 || default.pgprot == 0 {
        return None;
    }

    if !arch_has_rox {
        for range in &mut info.ranges {
            range.flags &= !EXECMEM_ROX_CACHE;
        }
    }

    execmem_init_missing(&mut info);
    Some(info)
}

pub fn execmem_init_missing(info: &mut ExecmemInfo) {
    let default = info.ranges[EXECMEM_DEFAULT];
    for i in EXECMEM_DEFAULT + 1..EXECMEM_TYPE_MAX {
        if info.ranges[i].start == 0 {
            info.ranges[i] = ExecmemRange {
                pgprot: if i == EXECMEM_MODULE_DATA {
                    PAGE_KERNEL
                } else {
                    default.pgprot
                },
                ..default
            };
        }
    }
}

pub fn execmem_alloc_plan(
    info: &ExecmemInfo,
    typ: usize,
    size: usize,
) -> Option<ExecmemAllocation> {
    let range = *info.ranges.get(typ)?;
    let size = page_align(size);
    let use_cache = range.flags & EXECMEM_ROX_CACHE != 0;
    let vm_flags = if range.flags & EXECMEM_KASAN_SHADOW != 0 {
        VM_FLUSH_RESET_PERMS | VM_DEFER_KMEMLEAK
    } else {
        VM_FLUSH_RESET_PERMS
    };

    let Some(start) = aligned_range_start(range.start, range.end, range.alignment, size) else {
        let fallback = aligned_range_start(
            range.fallback_start,
            range.fallback_end,
            range.alignment,
            size,
        )?;
        return Some(ExecmemAllocation {
            typ,
            start: fallback,
            size,
            pgprot: range.pgprot,
            vm_flags,
            used_fallback: true,
            use_cache,
        });
    };

    Some(ExecmemAllocation {
        typ,
        start,
        size,
        pgprot: range.pgprot,
        vm_flags,
        used_fallback: false,
        use_cache,
    })
}

pub const fn within_range(range: ExecmemRange, addr: usize, size: usize) -> bool {
    let Some(end) = addr.checked_add(size) else {
        return false;
    };
    if addr >= range.start && end < range.end {
        return true;
    }
    range.fallback_start != 0 && addr >= range.fallback_start && end < range.fallback_end
}

pub const fn is_pending_free(ptr: usize) -> bool {
    ptr & PENDING_FREE_MASK != 0
}

pub const fn pending_free_set(ptr: usize) -> usize {
    ptr | PENDING_FREE_MASK
}

pub const fn pending_free_clear(ptr: usize) -> usize {
    ptr & !PENDING_FREE_MASK
}

pub const fn page_align(size: usize) -> usize {
    (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

fn aligned_range_start(start: usize, end: usize, align: usize, size: usize) -> Option<usize> {
    if start == 0 || end == 0 || align == 0 {
        return None;
    }
    let aligned = (start + align - 1) & !(align - 1);
    if aligned.checked_add(size)? < end {
        Some(aligned)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execmem_validation_and_missing_ranges_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/execmem.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/execmem.h"
        ));

        assert!(header.contains("EXECMEM_MODULE_TEXT = EXECMEM_DEFAULT"));
        assert!(header.contains("EXECMEM_MODULE_DATA"));
        assert!(header.contains("EXECMEM_KASAN_SHADOW\t= (1 << 0)"));
        assert!(header.contains("EXECMEM_ROX_CACHE\t= (1 << 1)"));
        assert!(
            source.contains("if (!r->alignment || !r->start || !r->end || !pgprot_val(r->pgprot))")
        );
        assert!(source.contains("r->flags &= ~EXECMEM_ROX_CACHE;"));
        assert!(source.contains("if (!r->start)"));
        assert!(source.contains("if (i == EXECMEM_MODULE_DATA)"));
        assert!(source.contains("size = PAGE_ALIGN(size);"));
        assert!(source.contains("if (use_cache)"));
        assert!(source.contains("if (!p && range->fallback_start)"));
        assert!(source.contains("#define PENDING_FREE_MASK\t(1 << (PAGE_SHIFT - 1))"));

        let mut info = default_execmem_info();
        info.ranges[EXECMEM_DEFAULT].flags = EXECMEM_ROX_CACHE;
        let prepared = execmem_prepare(info, false).unwrap();
        assert_eq!(
            prepared.ranges[EXECMEM_DEFAULT].flags & EXECMEM_ROX_CACHE,
            0
        );
        assert_eq!(prepared.ranges[EXECMEM_MODULE_DATA].pgprot, PAGE_KERNEL);
        assert!(execmem_prepare(ExecmemInfo::default(), true).is_none());
    }

    #[test]
    fn execmem_alloc_uses_primary_then_fallback_ranges() {
        let mut info = ExecmemInfo::default();
        info.ranges[EXECMEM_DEFAULT] = ExecmemRange {
            start: 0x1000,
            end: 0x3000,
            fallback_start: 0x8000,
            fallback_end: 0xc000,
            pgprot: PAGE_KERNEL_EXEC,
            alignment: 0x1000,
            flags: EXECMEM_KASAN_SHADOW | EXECMEM_ROX_CACHE,
        };
        execmem_init_missing(&mut info);

        let alloc = execmem_alloc_plan(&info, EXECMEM_KPROBES, 0x800).unwrap();
        assert_eq!(alloc.start, 0x1000);
        assert_eq!(alloc.size, PAGE_SIZE);
        assert!(!alloc.used_fallback);
        assert!(alloc.use_cache);
        assert_eq!(alloc.vm_flags, VM_FLUSH_RESET_PERMS | VM_DEFER_KMEMLEAK);

        let fallback = execmem_alloc_plan(&info, EXECMEM_KPROBES, 0x1800).unwrap();
        assert_eq!(fallback.start, 0x8000);
        assert!(fallback.used_fallback);

        let pending = pending_free_set(0x4000);
        assert!(is_pending_free(pending));
        assert_eq!(pending_free_clear(pending), 0x4000);
    }
}
