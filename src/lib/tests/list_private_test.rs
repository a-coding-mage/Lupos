//! linux-parity: complete
//! linux-source: vendor/linux/lib/tests/list-private-test.c
//! test-origin: linux:vendor/linux/lib/tests/list-private-test.c
//! KUnit compile-smoke inventory for private list primitives.

pub const LIST_PRIVATE_KUNIT_SUITE: &str = "list-private-kunit-test";

pub const PRIVATE_LIST_PRIMITIVES: &[&str] = &[
    "list_private_entry",
    "list_private_first_entry",
    "list_private_last_entry",
    "list_private_next_entry",
    "list_private_prev_entry",
    "list_private_next_entry_circular",
    "list_private_prev_entry_circular",
    "list_private_entry_is_head",
    "list_private_for_each_entry",
    "list_private_for_each_entry_reverse",
    "list_private_for_each_entry_continue",
    "list_private_for_each_entry_continue_reverse",
    "list_private_for_each_entry_from",
    "list_private_for_each_entry_from_reverse",
    "list_private_for_each_entry_safe",
    "list_private_safe_reset_next",
    "list_private_for_each_entry_safe_continue",
    "list_private_for_each_entry_safe_from",
    "list_private_for_each_entry_safe_reverse",
];

pub fn covers_private_list_primitive(name: &str) -> bool {
    PRIVATE_LIST_PRIMITIVES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_private_test_covers_linux_compile_smoke_macros() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/list-private-test.c"
        ));
        assert!(source.contains("#undef __private"));
        assert!(source.contains("#define __private volatile"));
        assert!(source.contains("#undef ACCESS_PRIVATE"));
        assert!(source.contains("list_private_compile_test"));
        assert!(source.contains(".name = \"list-private-kunit-test\""));
        for primitive in PRIVATE_LIST_PRIMITIVES {
            assert!(source.contains(primitive));
            assert!(covers_private_list_primitive(primitive));
        }
    }
}
