//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/tests
//! test-origin: linux:vendor/linux/lib/math/tests
//! Linux math KUnit source coverage.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MathKunitModule {
    pub rust_module: &'static str,
    pub linux_source: &'static str,
    pub suite_name: &'static str,
    pub case_marker: &'static str,
    pub suite_marker: &'static str,
    pub makefile_config: &'static str,
    pub makefile_object: &'static str,
}

pub const MATH_KUNIT_MODULES: &[MathKunitModule] = &[
    MathKunitModule {
        rust_module: "gcd_kunit",
        linux_source: "vendor/linux/lib/math/tests/gcd_kunit.c",
        suite_name: "math-gcd",
        case_marker: "KUNIT_CASE_PARAM(gcd_test, gcd_gen_params)",
        suite_marker: "kunit_test_suite(gcd_test_suite);",
        makefile_config: "CONFIG_GCD_KUNIT_TEST",
        makefile_object: "gcd_kunit.o",
    },
    MathKunitModule {
        rust_module: "int_log_kunit",
        linux_source: "vendor/linux/lib/math/tests/int_log_kunit.c",
        suite_name: "math-int_log",
        case_marker: "KUNIT_CASE_PARAM(intlog2_test, intlog2_gen_params)",
        suite_marker: "kunit_test_suites(&int_log_test_suite);",
        makefile_config: "CONFIG_INT_LOG_KUNIT_TEST",
        makefile_object: "int_log_kunit.o",
    },
    MathKunitModule {
        rust_module: "int_pow_kunit",
        linux_source: "vendor/linux/lib/math/tests/int_pow_kunit.c",
        suite_name: "math-int_pow",
        case_marker: "KUNIT_CASE_PARAM(int_pow_test, int_pow_gen_params)",
        suite_marker: "kunit_test_suites(&int_pow_test_suite);",
        makefile_config: "CONFIG_INT_POW_KUNIT_TEST",
        makefile_object: "int_pow_kunit.o",
    },
    MathKunitModule {
        rust_module: "int_sqrt_kunit",
        linux_source: "vendor/linux/lib/math/tests/int_sqrt_kunit.c",
        suite_name: "math-int_sqrt",
        case_marker: "KUNIT_CASE_PARAM(int_sqrt_test, int_sqrt_gen_params)",
        suite_marker: "kunit_test_suites(&int_sqrt_test_suite);",
        makefile_config: "CONFIG_INT_SQRT_KUNIT_TEST",
        makefile_object: "int_sqrt_kunit.o",
    },
    MathKunitModule {
        rust_module: "prime_numbers_kunit",
        linux_source: "vendor/linux/lib/math/tests/prime_numbers_kunit.c",
        suite_name: "math-prime_numbers",
        case_marker: "KUNIT_CASE(prime_numbers_test)",
        suite_marker: "kunit_test_suite(prime_numbers_suite);",
        makefile_config: "CONFIG_PRIME_NUMBERS_KUNIT_TEST",
        makefile_object: "prime_numbers_kunit.o",
    },
    MathKunitModule {
        rust_module: "rational_kunit",
        linux_source: "vendor/linux/lib/math/tests/rational_kunit.c",
        suite_name: "rational",
        case_marker: "KUNIT_CASE_PARAM(rational_test, rational_gen_params)",
        suite_marker: "kunit_test_suites(&rational_test_suite);",
        makefile_config: "CONFIG_RATIONAL_KUNIT_TEST",
        makefile_object: "rational_kunit.o",
    },
];

pub mod gcd_kunit;
pub mod int_log_kunit;
pub mod int_pow_kunit;
pub mod int_sqrt_kunit;
pub mod prime_numbers_kunit;
pub mod rational_kunit;

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_math_kunit_inventory {
        ($(($module:literal, $source:literal)),+ $(,)?) => {
            #[test]
            fn math_kunit_inventory_matches_complete_children_and_vendor_sources() {
                let mut idx = 0usize;
                $(
                    let rust = include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/src/lib/math/tests/",
                        $module,
                        ".rs"
                    ));
                    let linux = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $source));
                    let declared = MATH_KUNIT_MODULES[idx];

                    assert_eq!(declared.rust_module, $module);
                    assert_eq!(declared.linux_source, $source);
                    assert!(rust.contains("//! linux-parity: complete"), "{}", $module);
                    assert!(
                        rust.contains(concat!("//! linux-source: ", $source)),
                        "{} missing source tag {}",
                        $module,
                        $source
                    );
                    assert!(
                        linux.contains("SPDX-License-Identifier: GPL-2.0"),
                        "{} missing SPDX",
                        $source
                    );
                    assert!(
                        linux.contains("#include <kunit/test.h>"),
                        "{} missing KUnit include",
                        $source
                    );
                    assert!(
                        linux.contains(declared.case_marker),
                        "{} missing {}",
                        $source,
                        declared.case_marker
                    );
                    assert!(
                        linux.contains(declared.suite_marker),
                        "{} missing {}",
                        $source,
                        declared.suite_marker
                    );
                    assert!(
                        linux.contains("MODULE_LICENSE(\"GPL"),
                        "{} missing GPL module license",
                        $source
                    );
                    assert!(
                        linux.contains(declared.suite_name),
                        "{} missing suite name {}",
                        $source,
                        declared.suite_name
                    );

                    idx += 1;
                )+
                assert_eq!(idx, MATH_KUNIT_MODULES.len());
            }
        };
    }

    assert_math_kunit_inventory!(
        ("gcd_kunit", "vendor/linux/lib/math/tests/gcd_kunit.c"),
        (
            "int_log_kunit",
            "vendor/linux/lib/math/tests/int_log_kunit.c"
        ),
        (
            "int_pow_kunit",
            "vendor/linux/lib/math/tests/int_pow_kunit.c"
        ),
        (
            "int_sqrt_kunit",
            "vendor/linux/lib/math/tests/int_sqrt_kunit.c"
        ),
        (
            "prime_numbers_kunit",
            "vendor/linux/lib/math/tests/prime_numbers_kunit.c"
        ),
        (
            "rational_kunit",
            "vendor/linux/lib/math/tests/rational_kunit.c"
        ),
    );

    #[test]
    fn math_kunit_makefile_references_every_child_object() {
        let makefile = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/tests/Makefile"
        ));

        assert!(makefile.contains("# SPDX-License-Identifier: GPL-2.0-only"));
        for module in MATH_KUNIT_MODULES {
            assert!(
                makefile.contains(module.makefile_config),
                "Makefile missing {}",
                module.makefile_config
            );
            assert!(
                makefile.contains(module.makefile_object),
                "Makefile missing {}",
                module.makefile_object
            );
        }
    }
}
