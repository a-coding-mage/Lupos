//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_fortify
//! test-origin: linux:vendor/linux/lib/test_fortify
//! Linux fortify compile-probe source contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FortifyProbe {
    pub linux_source: &'static str,
    pub test_expression: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FortifyProbeModule {
    pub rust_module: &'static str,
    pub linux_source: &'static str,
}

pub const FORTIFY_PROBE_MODULES: &[FortifyProbeModule] = &[
    FortifyProbeModule {
        rust_module: "read_overflow2_field_memcpy",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow2_field-memcpy.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow2_field_memmove",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow2_field-memmove.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow2_memcmp",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow2-memcmp.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow2_memcpy",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow2-memcpy.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow2_memmove",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow2-memmove.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow_memchr",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow-memchr.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow_memchr_inv",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow-memchr_inv.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow_memcmp",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow-memcmp.c",
    },
    FortifyProbeModule {
        rust_module: "read_overflow_memscan",
        linux_source: "vendor/linux/lib/test_fortify/read_overflow-memscan.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_field_memcpy",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow_field-memcpy.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_field_memmove",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow_field-memmove.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_field_memset",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow_field-memset.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_memcpy",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-memcpy.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_memmove",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-memmove.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_memset",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-memset.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_strcpy",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-strcpy.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_strcpy_lit",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-strcpy-lit.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_strncpy",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-strncpy.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_strncpy_src",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-strncpy-src.c",
    },
    FortifyProbeModule {
        rust_module: "write_overflow_strscpy",
        linux_source: "vendor/linux/lib/test_fortify/write_overflow-strscpy.c",
    },
];

pub const TEST_FORTIFY_HEADER: &str = "vendor/linux/lib/test_fortify/test_fortify.h";
pub const TEST_FORTIFY_SCRIPT: &str = "vendor/linux/lib/test_fortify/test_fortify.sh";
pub const TEST_FORTIFY_MAKEFILE: &str = "vendor/linux/lib/test_fortify/Makefile";

pub mod read_overflow2_field_memcpy;
pub mod read_overflow2_field_memmove;
pub mod read_overflow2_memcmp;
pub mod read_overflow2_memcpy;
pub mod read_overflow2_memmove;
pub mod read_overflow_memchr;
pub mod read_overflow_memchr_inv;
pub mod read_overflow_memcmp;
pub mod read_overflow_memscan;
pub mod write_overflow_field_memcpy;
pub mod write_overflow_field_memmove;
pub mod write_overflow_field_memset;
pub mod write_overflow_memcpy;
pub mod write_overflow_memmove;
pub mod write_overflow_memset;
pub mod write_overflow_strcpy;
pub mod write_overflow_strcpy_lit;
pub mod write_overflow_strncpy;
pub mod write_overflow_strncpy_src;
pub mod write_overflow_strscpy;

#[cfg(test)]
pub(crate) fn assert_fortify_probe(source: &str, probe: FortifyProbe) {
    let mut lines = source.lines();
    assert_eq!(
        lines.next(),
        Some("// SPDX-License-Identifier: GPL-2.0-only"),
        "{}",
        probe.linux_source
    );
    assert!(
        source.contains("#define TEST"),
        "{} missing TEST macro",
        probe.linux_source
    );
    assert!(
        source.contains(probe.test_expression),
        "{} missing {}",
        probe.linux_source,
        probe.test_expression
    );
    assert!(
        source.contains("#include \"test_fortify.h\""),
        "{} missing test_fortify.h include",
        probe.linux_source
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_probe_inventory {
        ($(($module:literal, $source:literal)),+ $(,)?) => {
            #[test]
            fn fortify_probe_inventory_matches_complete_children_and_vendor_sources() {
                let mut idx = 0usize;
                $(
                    let rust = include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/src/lib/test_fortify/",
                        $module,
                        ".rs"
                    ));
                    let linux = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $source));
                    let declared = FORTIFY_PROBE_MODULES[idx];

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
                        linux.starts_with("// SPDX-License-Identifier: GPL-2.0-only"),
                        "{}",
                        $source
                    );
                    assert!(linux.contains("#define TEST"), "{} missing TEST", $source);
                    assert!(
                        linux.contains("#include \"test_fortify.h\""),
                        "{} missing harness include",
                        $source
                    );

                    idx += 1;
                )+
                assert_eq!(idx, FORTIFY_PROBE_MODULES.len());
            }
        };
    }

    assert_probe_inventory!(
        (
            "read_overflow2_field_memcpy",
            "vendor/linux/lib/test_fortify/read_overflow2_field-memcpy.c"
        ),
        (
            "read_overflow2_field_memmove",
            "vendor/linux/lib/test_fortify/read_overflow2_field-memmove.c"
        ),
        (
            "read_overflow2_memcmp",
            "vendor/linux/lib/test_fortify/read_overflow2-memcmp.c"
        ),
        (
            "read_overflow2_memcpy",
            "vendor/linux/lib/test_fortify/read_overflow2-memcpy.c"
        ),
        (
            "read_overflow2_memmove",
            "vendor/linux/lib/test_fortify/read_overflow2-memmove.c"
        ),
        (
            "read_overflow_memchr",
            "vendor/linux/lib/test_fortify/read_overflow-memchr.c"
        ),
        (
            "read_overflow_memchr_inv",
            "vendor/linux/lib/test_fortify/read_overflow-memchr_inv.c"
        ),
        (
            "read_overflow_memcmp",
            "vendor/linux/lib/test_fortify/read_overflow-memcmp.c"
        ),
        (
            "read_overflow_memscan",
            "vendor/linux/lib/test_fortify/read_overflow-memscan.c"
        ),
        (
            "write_overflow_field_memcpy",
            "vendor/linux/lib/test_fortify/write_overflow_field-memcpy.c"
        ),
        (
            "write_overflow_field_memmove",
            "vendor/linux/lib/test_fortify/write_overflow_field-memmove.c"
        ),
        (
            "write_overflow_field_memset",
            "vendor/linux/lib/test_fortify/write_overflow_field-memset.c"
        ),
        (
            "write_overflow_memcpy",
            "vendor/linux/lib/test_fortify/write_overflow-memcpy.c"
        ),
        (
            "write_overflow_memmove",
            "vendor/linux/lib/test_fortify/write_overflow-memmove.c"
        ),
        (
            "write_overflow_memset",
            "vendor/linux/lib/test_fortify/write_overflow-memset.c"
        ),
        (
            "write_overflow_strcpy",
            "vendor/linux/lib/test_fortify/write_overflow-strcpy.c"
        ),
        (
            "write_overflow_strcpy_lit",
            "vendor/linux/lib/test_fortify/write_overflow-strcpy-lit.c"
        ),
        (
            "write_overflow_strncpy",
            "vendor/linux/lib/test_fortify/write_overflow-strncpy.c"
        ),
        (
            "write_overflow_strncpy_src",
            "vendor/linux/lib/test_fortify/write_overflow-strncpy-src.c"
        ),
        (
            "write_overflow_strscpy",
            "vendor/linux/lib/test_fortify/write_overflow-strscpy.c"
        ),
    );

    #[test]
    fn fortify_harness_header_matches_linux_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_fortify/test_fortify.h"
        ));
        assert_eq!(
            TEST_FORTIFY_HEADER,
            "vendor/linux/lib/test_fortify/test_fortify.h"
        );
        assert!(source.contains("SPDX-License-Identifier: GPL-2.0-only"));
        assert!(source.contains("#define __BUF_SMALL\t16"));
        assert!(source.contains("#define __BUF_LARGE\t32"));
        assert!(source.contains("struct fortify_object"));
        assert!(source.contains("#define LITERAL_SMALL"));
        assert!(source.contains("#define LITERAL_LARGE"));
        assert!(source.contains("void do_fortify_tests(void)"));
        assert!(source.contains("TEST;"));
    }

    #[test]
    fn fortify_harness_script_matches_linux_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_fortify/test_fortify.sh"
        ));
        assert_eq!(
            TEST_FORTIFY_SCRIPT,
            "vendor/linux/lib/test_fortify/test_fortify.sh"
        );
        assert!(source.starts_with("#!/bin/sh"));
        assert!(source.contains("SPDX-License-Identifier: GPL-2.0-only"));
        assert!(source.contains("set -e"));
        assert!(source.contains("WANT=\"__${FILE%%-*}\""));
        assert!(source.contains("grep -Eq -m1"));
        assert!(source.contains("error: call to .?\\b${WANT}\\b.?"));
        assert!(source.contains("ok: unsafe ${FUNC}() usage correctly detected"));
    }

    #[test]
    fn fortify_makefile_matches_linux_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_fortify/Makefile"
        ));
        assert_eq!(
            TEST_FORTIFY_MAKEFILE,
            "vendor/linux/lib/test_fortify/Makefile"
        );
        assert!(source.contains("SPDX-License-Identifier: GPL-2.0"));
        assert!(source.contains("ccflags-y := $(call cc-disable-warning,fortify-source)"));
        assert!(source.contains("quiet_cmd_test_fortify = TEST"));
        assert!(source.contains("cmd_test_fortify = $(CONFIG_SHELL) $(src)/test_fortify.sh"));
        assert!(source.contains("logs = $(patsubst $(src)/%.c, %.log, $(wildcard $(src)/*-*.c))"));
        assert!(source.contains("always-y += test_fortify.log"));
        assert!(source.contains("KASAN_SANITIZE := y"));
    }
}
