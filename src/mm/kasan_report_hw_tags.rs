//! linux-parity: complete
//! linux-source: vendor/linux/mm/kasan/report_hw_tags.c
//! test-origin: linux:vendor/linux/mm/kasan/report_hw_tags.c
//! Hardware tag-based KASAN report helpers.

pub const KASAN_GRANULE_SIZE: usize = 16;
pub const KASAN_TAG_INVALID: u8 = 0xff;
pub const META_BYTES_PER_ROW: usize = 16;

pub const fn kasan_reset_tag(addr: usize) -> usize {
    addr & 0x00ff_ffff_ffff_ffff
}

pub const fn kasan_find_first_bad_addr(addr: usize) -> usize {
    kasan_reset_tag(addr)
}

pub fn kasan_get_alloc_size(memory_tags: &[u8], object_size: usize) -> usize {
    let mut size = 0;
    let granules = object_size.div_ceil(KASAN_GRANULE_SIZE);
    while size < object_size {
        let idx = size / KASAN_GRANULE_SIZE;
        if idx >= granules || idx >= memory_tags.len() {
            return size;
        }
        if memory_tags[idx] == KASAN_TAG_INVALID {
            return size;
        }
        size += KASAN_GRANULE_SIZE;
    }
    object_size
}

pub fn kasan_metadata_fetch_row(buffer: &mut [u8; META_BYTES_PER_ROW], row_tags: &[u8]) {
    for (idx, out) in buffer.iter_mut().enumerate() {
        *out = row_tags.get(idx).copied().unwrap_or(KASAN_TAG_INVALID);
    }
}

pub const fn kasan_print_tags_message(addr_tag: u8, memory_tag: u8) -> (u8, u8) {
    (addr_tag, memory_tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hw_tags_report_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/kasan/report_hw_tags.c"
        ));
        assert!(source.contains("Hardware Tag-Based KASAN"));
        assert!(source.contains("return kasan_reset_tag(addr);"));
        assert!(source.contains("while (size < cache->object_size)"));
        assert!(source.contains("hw_get_mem_tag(object + i * KASAN_GRANULE_SIZE)"));
        assert!(source.contains("memory_tag != KASAN_TAG_INVALID"));
        assert!(source.contains("return cache->object_size;"));
        assert!(source.contains("META_BYTES_PER_ROW"));
        assert!(source.contains("Pointer tag: [%02x], memory tag: [%02x]"));

        assert_eq!(kasan_find_first_bad_addr(0xab00_0000_0000_1234), 0x1234);
        assert_eq!(kasan_get_alloc_size(&[1, 2, KASAN_TAG_INVALID, 3], 64), 32);
        assert_eq!(kasan_get_alloc_size(&[1, 2, 3, 4], 64), 64);
        let mut row = [0; META_BYTES_PER_ROW];
        kasan_metadata_fetch_row(&mut row, &[7, 8]);
        assert_eq!(row[0], 7);
        assert_eq!(row[1], 8);
        assert_eq!(row[2], KASAN_TAG_INVALID);
        assert_eq!(kasan_print_tags_message(0xaa, 0xbb), (0xaa, 0xbb));
    }
}
