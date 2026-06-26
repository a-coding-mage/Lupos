//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/atomic64_32.c
//! test-origin: linux:vendor/linux/arch/x86/lib/atomic64_32.c
//! 32-bit x86 atomic64 export wrapper.

pub use super::arch_lib::{atomic64_add_return_32, atomic64_cmpxchg_32};

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicI64, Ordering};

    #[test]
    fn wrapper_matches_linux_atomic64_export_file() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/atomic64_32.c"
        ));
        assert!(source.contains("#define ATOMIC64_EXPORT EXPORT_SYMBOL"));
        assert!(source.contains("#include <linux/atomic.h>"));

        let value = AtomicI64::new(1);
        assert_eq!(atomic64_add_return_32(&value, 2), 3);
        assert_eq!(atomic64_cmpxchg_32(&value, 3, 9), 3);
        assert_eq!(value.load(Ordering::Acquire), 9);
    }
}
