//! linux-parity: complete
//! linux-source: vendor/linux/mm/tests
//! test-origin: linux:vendor/linux/mm/tests
//! MM KUnit parity modules.

pub mod lazy_mmu_mode_kunit;

pub const MM_TEST_MODULES: [&str; 1] = ["lazy_mmu_mode_kunit"];
pub const LAZY_MMU_MODE_KUNIT_SUITE: &str = "lazy_mmu_mode";

#[cfg(test)]
mod tests {
    use super::*;

    const LAZY_MMU_MODE_KUNIT_C: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/mm/tests/lazy_mmu_mode_kunit.c"
    ));

    #[test]
    fn mm_tests_wrapper_matches_linux_source_set() {
        assert_eq!(MM_TEST_MODULES, ["lazy_mmu_mode_kunit"]);
        assert_eq!(LAZY_MMU_MODE_KUNIT_SUITE, "lazy_mmu_mode");
        assert!(LAZY_MMU_MODE_KUNIT_C.contains("#include <kunit/test.h>"));
        assert!(LAZY_MMU_MODE_KUNIT_C.contains("KUNIT_CASE(lazy_mmu_mode_active)"));
        assert!(LAZY_MMU_MODE_KUNIT_C.contains(".name = \"lazy_mmu_mode\""));
        assert!(LAZY_MMU_MODE_KUNIT_C.contains("kunit_test_suite(lazy_mmu_mode_test_suite);"));
        assert!(LAZY_MMU_MODE_KUNIT_C.contains("MODULE_LICENSE(\"GPL\");"));
    }

    #[test]
    fn mm_tests_wrapper_reexports_lazy_mmu_contract() {
        assert!(LAZY_MMU_MODE_KUNIT_C.contains("lazy_mmu_mode_pause();"));
        assert!(LAZY_MMU_MODE_KUNIT_C.contains("lazy_mmu_mode_resume();"));

        let mut mode = lazy_mmu_mode_kunit::LazyMmuMode::new();
        assert!(!mode.is_active());
        mode.enable();
        assert!(mode.is_active());
        mode.pause();
        assert!(!mode.is_active());
        mode.enable();
        mode.disable();
        assert!(!mode.is_active());
        mode.resume();
        assert!(mode.is_active());
        mode.disable();
        assert!(!mode.is_active());
    }
}
