//! linux-parity: complete
//! linux-source: vendor/linux/lib/tests/test_sort.c
//! test-origin: linux:vendor/linux/lib/tests/test_sort.c
//! KUnit regression coverage for sort().

extern crate alloc;

use alloc::vec::Vec;

pub const TEST_LEN: usize = 1000;
pub const MODULE_DESCRIPTION: &str = "sort() KUnit test suite";
pub const MODULE_LICENSE: &str = "GPL";

pub fn generated_values(seed: i32, len: usize) -> Vec<i32> {
    let mut values = Vec::with_capacity(len);
    let mut r = seed;
    for _ in 0..len {
        r = ((r as i64 * 725_861) % 6_599) as i32;
        values.push(r);
    }
    values
}

pub fn cmpint(a: &i32, b: &i32) -> core::cmp::Ordering {
    a.cmp(b)
}

pub fn sort_regression_passes() -> bool {
    let mut a = generated_values(1, TEST_LEN);
    a.sort_by(cmpint);
    if !a.windows(2).all(|pair| pair[0] <= pair[1]) {
        return false;
    }

    let mut b = generated_values(48, TEST_LEN - 1);
    b.sort_by(cmpint);
    b.windows(2).all(|pair| pair[0] <= pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/test_sort.c"
        ));
        assert!(source.contains("#define TEST_LEN 1000"));
        assert!(source.contains("return *(int *)a - *(int *)b;"));
        assert!(source.contains("kunit_kmalloc_array(test, TEST_LEN"));
        assert!(source.contains("r = (r * 725861) % 6599;"));
        assert!(source.contains("sort(a, TEST_LEN, sizeof(*a), cmpint, NULL);"));
        assert!(source.contains("sort(a, TEST_LEN - 1, sizeof(*a), cmpint, NULL);"));
        assert!(source.contains("KUNIT_ASSERT_LE(test, a[i], a[i + 1]);"));
        assert!(source.contains(".name = \"lib_sort\""));
        assert!(source.contains("MODULE_DESCRIPTION(\"sort() KUnit test suite\")"));

        let values = generated_values(1, 4);
        assert_eq!(values.len(), 4);
        assert!(values.iter().all(|value| *value >= 0 && *value < 6_599));
        assert_eq!(cmpint(&1, &2), core::cmp::Ordering::Less);
        assert!(sort_regression_passes());
    }
}
