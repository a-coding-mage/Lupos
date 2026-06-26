//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_free_pages.c
//! test-origin: linux:vendor/linux/lib/test_free_pages.c
//! Free-pages leak test loop metadata.

pub const TEST_FREE_PAGES_ITERATIONS: usize = 1000 * 1000;
pub const TEST_FREE_PAGES_ORDER: u32 = 3;
pub const GFP_KERNEL_CASE: u32 = 0;
pub const GFP_KERNEL_COMP_CASE: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreePagesTestCase {
    pub gfp_case: u32,
    pub order: u32,
    pub iterations: usize,
}

pub const FREE_PAGES_TEST_CASES: [FreePagesTestCase; 2] = [
    FreePagesTestCase {
        gfp_case: GFP_KERNEL_CASE,
        order: TEST_FREE_PAGES_ORDER,
        iterations: TEST_FREE_PAGES_ITERATIONS,
    },
    FreePagesTestCase {
        gfp_case: GFP_KERNEL_COMP_CASE,
        order: TEST_FREE_PAGES_ORDER,
        iterations: TEST_FREE_PAGES_ITERATIONS,
    },
];

pub fn free_pages_test_cases() -> &'static [FreePagesTestCase] {
    &FREE_PAGES_TEST_CASES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_pages_test_metadata_matches_linux_module() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_free_pages.c"
        ));
        assert!(source.contains("static void test_free_pages(gfp_t gfp)"));
        assert!(source.contains("for (i = 0; i < 1000 * 1000; i++)"));
        assert!(source.contains("__get_free_pages(gfp, 3);"));
        assert!(source.contains("get_page(page);"));
        assert!(source.contains("free_pages(addr, 3);"));
        assert!(source.contains("put_page(page);"));
        assert!(source.contains("test_free_pages(GFP_KERNEL);"));
        assert!(source.contains("test_free_pages(GFP_KERNEL | __GFP_COMP);"));

        assert_eq!(TEST_FREE_PAGES_ITERATIONS, 1_000_000);
        assert_eq!(TEST_FREE_PAGES_ORDER, 3);
        assert_eq!(free_pages_test_cases().len(), 2);
        assert_eq!(free_pages_test_cases()[0].gfp_case, GFP_KERNEL_CASE);
        assert_eq!(free_pages_test_cases()[1].gfp_case, GFP_KERNEL_COMP_CASE);
    }
}
