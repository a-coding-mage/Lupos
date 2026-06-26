//! linux-parity: complete
//! linux-source: vendor/linux/lib/iommu-helper.c
//! test-origin: linux:vendor/linux/lib/iommu-helper.c
//! IOMMU bitmap allocation helper.

pub const fn iommu_is_span_boundary(
    index: usize,
    nr: usize,
    shift: usize,
    boundary_size: usize,
) -> bool {
    ((shift + index) & (boundary_size - 1)) + nr > boundary_size
}

const fn align(value: usize, boundary: usize) -> usize {
    (value + boundary - 1) & !(boundary - 1)
}

fn find_next_zero_area(
    map: &[bool],
    size: usize,
    mut start: usize,
    nr: usize,
    align_mask: usize,
) -> usize {
    while start + nr <= size {
        let aligned = (start + align_mask) & !align_mask;
        if aligned != start {
            start = aligned;
            continue;
        }
        let mut offset = 0usize;
        while offset < nr && !map[start + offset] {
            offset += 1;
        }
        if offset == nr {
            return start;
        }
        start += offset + 1;
    }
    size
}

pub fn iommu_area_alloc(
    map: &mut [bool],
    size: usize,
    mut start: usize,
    nr: usize,
    shift: usize,
    boundary_size: usize,
    align_mask: usize,
) -> Option<usize> {
    if size == 0 || nr == 0 {
        return None;
    }
    let limit = size - 1;
    loop {
        let index = find_next_zero_area(map, limit, start, nr, align_mask);
        if index < limit {
            if iommu_is_span_boundary(index, nr, shift, boundary_size) {
                start = align(shift + index, boundary_size) - shift;
                continue;
            }
            let end = index + nr;
            if end > map.len() {
                return None;
            }
            map[index..end].fill(true);
            return Some(index);
        }
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iommu_area_alloc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/iommu-helper.c"
        ));
        assert!(source.contains("size -= 1;"));
        assert!(source.contains("bitmap_find_next_zero_area"));
        assert!(source.contains("iommu_is_span_boundary(index, nr, shift, boundary_size)"));
        assert!(source.contains("bitmap_set(map, index, nr);"));
        assert!(iommu_is_span_boundary(6, 4, 0, 8));
        assert!(!iommu_is_span_boundary(4, 4, 0, 8));
        let mut map = [false; 16];
        assert_eq!(iommu_area_alloc(&mut map, 16, 0, 4, 0, 8, 0), Some(0));
        assert_eq!(iommu_area_alloc(&mut map, 16, 4, 4, 0, 8, 0), Some(4));
        assert_eq!(iommu_area_alloc(&mut map, 16, 6, 4, 0, 8, 0), Some(8));
    }
}
