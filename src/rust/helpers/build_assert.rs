//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/build_assert.c
//! test-origin: linux:vendor/linux/rust/helpers/build_assert.c
//! Rust helper build-time ABI assertion.

const _: () = assert!(
    core::mem::size_of::<usize>() == core::mem::size_of::<*const ()>()
        && core::mem::align_of::<usize>() == core::mem::align_of::<*const ()>(),
    "Rust code expects C `size_t` to match Rust `usize`"
);

pub const INCLUDE_LINE: &str = "#include <linux/build_bug.h>";
pub const STATIC_ASSERT_MACRO: &str = "static_assert";
pub const C_SIZE_T: &str = "size_t";
pub const C_UINTPTR_T: &str = "uintptr_t";
pub const SIZEOF_OPERATOR: &str = "sizeof";
pub const ALIGNOF_OPERATOR: &str = "__alignof__";
pub const SIZE_ASSERTION: &str = "sizeof(size_t) == sizeof(uintptr_t)";
pub const ALIGN_ASSERTION: &str = "__alignof__(size_t) == __alignof__(uintptr_t)";
pub const ASSERT_MESSAGE: &str = "Rust code expects C `size_t` to match Rust `usize`";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustUsizeAbiAssert {
    pub include_line: &'static str,
    pub macro_name: &'static str,
    pub size_assertion: &'static str,
    pub align_assertion: &'static str,
    pub message: &'static str,
}

pub const RUST_USIZE_ABI_ASSERT: RustUsizeAbiAssert = RustUsizeAbiAssert {
    include_line: INCLUDE_LINE,
    macro_name: STATIC_ASSERT_MACRO,
    size_assertion: SIZE_ASSERTION,
    align_assertion: ALIGN_ASSERTION,
    message: ASSERT_MESSAGE,
};

pub fn rust_usize_abi_assertion_passes(size_matches: bool, align_matches: bool) -> bool {
    size_matches && align_matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_build_assert_source_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/build_assert.c"
        ));
        assert!(source.contains(INCLUDE_LINE));
        assert!(source.contains("static_assert("));
        assert!(source.contains(SIZEOF_OPERATOR));
        assert!(source.contains(ALIGNOF_OPERATOR));
        assert!(source.contains(C_SIZE_T));
        assert!(source.contains(C_UINTPTR_T));
        assert!(source.contains("sizeof(size_t) == sizeof(uintptr_t) &&"));
        assert!(source.contains(SIZE_ASSERTION));
        assert!(source.contains("__alignof__(size_t) == __alignof__(uintptr_t),"));
        assert!(source.contains(ALIGN_ASSERTION));
        assert!(source.contains(ASSERT_MESSAGE));
        assert_eq!(
            RUST_USIZE_ABI_ASSERT,
            RustUsizeAbiAssert {
                include_line: "#include <linux/build_bug.h>",
                macro_name: "static_assert",
                size_assertion: "sizeof(size_t) == sizeof(uintptr_t)",
                align_assertion: "__alignof__(size_t) == __alignof__(uintptr_t)",
                message: "Rust code expects C `size_t` to match Rust `usize`",
            }
        );
        assert!(rust_usize_abi_assertion_passes(true, true));
        assert!(!rust_usize_abi_assertion_passes(false, true));
        assert!(!rust_usize_abi_assertion_passes(true, false));
    }
}
