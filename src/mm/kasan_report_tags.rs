//! linux-parity: complete
//! linux-source: vendor/linux/mm/kasan/report_tags.c
//! test-origin: linux:vendor/linux/mm/kasan/report_tags.c
//! Tag-based KASAN report bug-type selection.

extern crate alloc;

use alloc::vec::Vec;

pub const BUG_TYPE_OUT_OF_BOUNDS: &str = "out-of-bounds";
pub const BUG_TYPE_INVALID_ACCESS: &str = "invalid-access";
pub const BUG_TYPE_SLAB_USE_AFTER_FREE: &str = "slab-use-after-free";
pub const BUG_TYPE_SLAB_OUT_OF_BOUNDS: &str = "slab-out-of-bounds";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KasanStackEntry {
    pub ptr: usize,
    pub size: usize,
    pub is_free: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KasanReportInfo {
    pub access_addr: usize,
    pub access_size: usize,
    pub object: Option<usize>,
    pub object_size: Option<usize>,
    pub bug_type: Option<&'static str>,
}

pub const fn kasan_reset_tag(addr: usize) -> usize {
    addr & 0x00ff_ffff_ffff_ffff
}

pub const fn get_tag(addr: usize) -> u8 {
    (addr >> 56) as u8
}

pub const fn get_common_bug_type(access_addr: usize, access_size: usize) -> &'static str {
    if access_addr.wrapping_add(access_size) < access_addr {
        BUG_TYPE_OUT_OF_BOUNDS
    } else {
        BUG_TYPE_INVALID_ACCESS
    }
}

pub fn complete_mode_report_bug_type(
    mut info: KasanReportInfo,
    stack_ring_newest_first: &[KasanStackEntry],
) -> &'static str {
    if (info.object.is_none() || info.object_size.is_none()) && info.bug_type.is_none() {
        return get_common_bug_type(info.access_addr, info.access_size);
    }
    if let Some(existing) = info.bug_type {
        return existing;
    }

    let mut alloc_found = false;
    let mut free_found = false;
    let object = info.object.unwrap_or(0);
    let object_size = info.object_size.unwrap_or(0);
    let access_tag = get_tag(info.access_addr);

    for entry in stack_ring_newest_first {
        if alloc_found && free_found {
            break;
        }
        if kasan_reset_tag(entry.ptr) != object
            || get_tag(entry.ptr) != access_tag
            || object_size != entry.size
        {
            continue;
        }
        if entry.is_free {
            if free_found {
                break;
            }
            free_found = true;
            if info.bug_type.is_none() {
                info.bug_type = Some(BUG_TYPE_SLAB_USE_AFTER_FREE);
            }
        } else {
            if alloc_found {
                break;
            }
            alloc_found = true;
            if info.bug_type.is_none() {
                info.bug_type = Some(BUG_TYPE_SLAB_OUT_OF_BOUNDS);
            }
        }
    }

    info.bug_type
        .unwrap_or_else(|| get_common_bug_type(info.access_addr, info.access_size))
}

pub fn newest_first(entries: &[KasanStackEntry]) -> Vec<KasanStackEntry> {
    entries.iter().rev().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_report_bug_type_selection_matches_linux_stack_ring_walk() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/kasan/report_tags.c"
        ));
        assert!(source.contains("static const char *get_common_bug_type"));
        assert!(source.contains("info->access_addr + info->access_size < info->access_addr"));
        assert!(source.contains("return \"out-of-bounds\";"));
        assert!(source.contains("return \"invalid-access\";"));
        assert!(source.contains("void kasan_complete_mode_report_info"));
        assert!(source.contains("bool alloc_found = false, free_found = false;"));
        assert!(source.contains("for (u64 i = pos - 1; i != pos - 1 - stack_ring.size; i--)"));
        assert!(source.contains("kasan_reset_tag(entry->ptr) != info->object"));
        assert!(source.contains("get_tag(entry->ptr) != get_tag(info->access_addr)"));
        assert!(source.contains("info->cache->object_size != entry->size"));
        assert!(source.contains("info->bug_type = \"slab-use-after-free\";"));
        assert!(source.contains("info->bug_type = \"slab-out-of-bounds\";"));

        assert_eq!(get_common_bug_type(usize::MAX, 1), BUG_TYPE_OUT_OF_BOUNDS);
        assert_eq!(get_common_bug_type(0x1000, 8), BUG_TYPE_INVALID_ACCESS);

        let info = KasanReportInfo {
            access_addr: 0xab00_0000_0000_1008,
            access_size: 8,
            object: Some(0x1000),
            object_size: Some(32),
            bug_type: None,
        };
        let entries = [
            KasanStackEntry {
                ptr: 0xab00_0000_0000_1000,
                size: 32,
                is_free: true,
            },
            KasanStackEntry {
                ptr: 0xab00_0000_0000_1000,
                size: 32,
                is_free: false,
            },
        ];
        assert_eq!(
            complete_mode_report_bug_type(info, &entries),
            BUG_TYPE_SLAB_USE_AFTER_FREE
        );
        let newest = newest_first(&entries);
        assert_eq!(newest[0].is_free, false);
        assert_eq!(
            complete_mode_report_bug_type(info, &newest),
            BUG_TYPE_SLAB_OUT_OF_BOUNDS
        );
    }
}
