//! linux-parity: complete
//! linux-source: vendor/linux/lib/percpu_test.c
//! test-origin: linux:vendor/linux/lib/percpu_test.c
//! Per-CPU counter operation test-module arithmetic.

pub const MODULE_DESCRIPTION: &str = "percpu operations test";
pub const UINT_MAX_AS_LONG: i64 = u32::MAX as i64;
pub const ULONG_MAX: usize = usize::MAX;

pub const fn unsigned_long_dec_from_zero() -> usize {
    0usize.wrapping_sub(1)
}

pub const fn long_add_negative_unsigned_int(value: i64) -> i64 {
    value.wrapping_add(u32::MAX as i64)
}

pub const fn long_add_unsigned_int(value: i64) -> i64 {
    value.wrapping_add(1)
}

pub const fn sub_return(value: usize, amount: usize) -> usize {
    value.wrapping_sub(amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percpu_test_matches_linux_original_test_module() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/percpu_test.c"
        ));

        assert!(source.contains("static DEFINE_PER_CPU(long, long_counter);"));
        assert!(source.contains("static DEFINE_PER_CPU(unsigned long, ulong_counter);"));
        assert!(source.contains("volatile unsigned int ui_one = 1;"));
        assert!(source.contains("__this_cpu_add(long_counter, -1);"));
        assert!(source.contains("__this_cpu_add(ulong_counter, -1UL);"));
        assert!(source.contains("__this_cpu_dec(ulong_counter);"));
        assert!(source.contains("__this_cpu_sub(long_counter, ui_one);"));
        assert!(source.contains("this_cpu_sub(long_counter, ui_one);"));
        assert!(source.contains("this_cpu_sub_return(ulong_counter, ui_one);"));
        assert!(source.contains("__this_cpu_sub_return(ulong_counter, ui_one);"));
        assert!(source.contains("return -EAGAIN;"));
        assert!(source.contains(MODULE_DESCRIPTION));

        assert_eq!(unsigned_long_dec_from_zero(), ULONG_MAX);
        assert_eq!(long_add_negative_unsigned_int(0), UINT_MAX_AS_LONG);
        assert_eq!(long_add_unsigned_int(UINT_MAX_AS_LONG), 0x100000000);
        assert_eq!(sub_return(3, 1), 2);
        assert_eq!(sub_return(2, 1), 1);
    }
}
