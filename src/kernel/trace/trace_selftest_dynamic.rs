//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_selftest_dynamic.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_selftest_dynamic.c
//! Out-of-tree functions referenced by the boot-time self-tests.  Kept
//! in their own object so symbols are findable by `selftest_test_*` paths.
//!
//! Ref: vendor/linux/kernel/trace/trace_selftest_dynamic.c

/// `trace_selftest_test_dyn_recursion_func` — recurses to depth `n`,
/// returning the final depth value.
pub fn recurse_to(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    recurse_to(n - 1) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurse_returns_depth() {
        assert_eq!(recurse_to(5), 5);
    }
}
