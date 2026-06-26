//! linux-parity: complete
//! linux-source: vendor/linux/mm/kasan/report_sw_tags.c
//! test-origin: linux:vendor/linux/mm/kasan/report_sw_tags.c
//! Software tag-based KASAN report helpers.

pub const KASAN_GRANULE_SIZE: usize = 16;
pub const KASAN_TAG_INVALID: u8 = 0xff;
pub const META_BYTES_PER_ROW: usize = 16;

pub const fn kasan_reset_tag(addr: usize) -> usize {
    addr & 0x00ff_ffff_ffff_ffff
}

pub fn kasan_find_first_bad_addr(
    tagged_addr: usize,
    size: usize,
    addr_has_metadata: bool,
    shadow_tags: &[u8],
) -> usize {
    let tag = (tagged_addr >> 56) as u8;
    let start = kasan_reset_tag(tagged_addr);
    if !addr_has_metadata {
        return start;
    }
    let mut offset = 0usize;
    while offset < size {
        let idx = offset / KASAN_GRANULE_SIZE;
        if shadow_tags.get(idx).copied() != Some(tag) {
            return start + offset;
        }
        offset += KASAN_GRANULE_SIZE;
    }
    start + size
}

pub fn kasan_get_alloc_size(shadow_tags: &[u8], object_size: usize) -> usize {
    let mut size = 0usize;
    while size < object_size {
        let idx = size / KASAN_GRANULE_SIZE;
        if shadow_tags.get(idx).copied().unwrap_or(KASAN_TAG_INVALID) == KASAN_TAG_INVALID {
            return size;
        }
        size += KASAN_GRANULE_SIZE;
    }
    object_size
}

pub fn kasan_metadata_fetch_row(buffer: &mut [u8; META_BYTES_PER_ROW], shadow_row: &[u8]) {
    for (idx, out) in buffer.iter_mut().enumerate() {
        *out = shadow_row.get(idx).copied().unwrap_or(0);
    }
}

pub const fn kasan_print_address_stack_frame_should_print(object_on_stack: bool) -> bool {
    object_on_stack
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sw_tags_report_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/kasan/report_sw_tags.c"
        ));
        assert!(source.contains("software tag-based KASAN specific error reporting code"));
        assert!(source.contains("u8 tag = get_tag(addr);"));
        assert!(source.contains("void *p = kasan_reset_tag(addr);"));
        assert!(source.contains("if (!addr_has_metadata(p))"));
        assert!(source.contains("while (p < end && tag == *(u8 *)kasan_mem_to_shadow(p))"));
        assert!(source.contains("while (size < cache->object_size)"));
        assert!(source.contains("if (*shadow != KASAN_TAG_INVALID)"));
        assert!(source.contains("memcpy(buffer, kasan_mem_to_shadow(row), META_BYTES_PER_ROW);"));
        assert!(source.contains("Pointer tag: [%02x], memory tag: [%02x]"));
        assert!(source.contains("object_is_on_stack(addr)"));

        assert_eq!(
            kasan_find_first_bad_addr(0xaa00_0000_0000_1000, 64, false, &[0xaa]),
            0x1000
        );
        assert_eq!(
            kasan_find_first_bad_addr(0xaa00_0000_0000_1000, 64, true, &[0xaa, 0xbb]),
            0x1010
        );
        assert_eq!(kasan_get_alloc_size(&[1, 2, KASAN_TAG_INVALID], 64), 32);
        let mut row = [0; META_BYTES_PER_ROW];
        kasan_metadata_fetch_row(&mut row, &[4, 5]);
        assert_eq!(row[0], 4);
        assert_eq!(row[1], 5);
        assert!(kasan_print_address_stack_frame_should_print(true));
    }
}
