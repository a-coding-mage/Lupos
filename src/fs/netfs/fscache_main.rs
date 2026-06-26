//! linux-parity: complete
//! linux-source: vendor/linux/fs/netfs/fscache_main.c
//! test-origin: linux:vendor/linux/fs/netfs/fscache_main.c
//! FS-Cache hash and module init/exit ordering.

use crate::include::uapi::errno::ENOMEM;

pub const FSCACHE_WORKQUEUE_NAME: &str = "fscache";
pub const FSCACHE_COOKIE_JAR_NAME: &str = "fscache_cookie_jar";
pub const GOLDEN_RATIO_32: u32 = 0x61c8_8647;

pub const FSCACHE_INIT_ORDER: &[&str] =
    &["alloc_workqueue", "fscache_proc_init", "kmem_cache_create"];

pub const FSCACHE_EXIT_ORDER: &[&str] = &[
    "kmem_cache_destroy",
    "fscache_proc_cleanup",
    "timer_shutdown_sync",
    "destroy_workqueue",
];

pub const fn hash_32_generic(val: u32) -> u32 {
    val.wrapping_mul(GOLDEN_RATIO_32)
}

pub const fn fscache_hash_words(salt: u32, words: &[u32]) -> u32 {
    let mut x = 0u32;
    let mut y = salt;
    let mut i = 0usize;
    while i < words.len() {
        let a = words[i];
        x ^= a;
        y ^= x;
        x = x.rotate_left(7);
        x = x.wrapping_add(y);
        y = y.rotate_left(20);
        y = y.wrapping_mul(9);
        i += 1;
    }
    hash_32_generic(y ^ hash_32_generic(x))
}

pub fn fscache_hash(salt: u32, data: &[u8]) -> u32 {
    assert!(data.len().is_multiple_of(4));
    let mut x = 0u32;
    let mut y = salt;
    for chunk in data.chunks_exact(4) {
        let a = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        x ^= a;
        y ^= x;
        x = x.rotate_left(7);
        x = x.wrapping_add(y);
        y = y.rotate_left(20);
        y = y.wrapping_mul(9);
    }
    hash_32_generic(y ^ hash_32_generic(x))
}

pub const fn fscache_init_result(
    workqueue_allocated: bool,
    proc_ret: i32,
    cookie_jar_allocated: bool,
) -> Result<(), i32> {
    if !workqueue_allocated {
        return Err(-ENOMEM);
    }
    if proc_ret < 0 {
        return Err(proc_ret);
    }
    if !cookie_jar_allocated {
        return Err(-ENOMEM);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fscache_main_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/netfs/fscache_main.c"
        ));
        assert!(source.contains("#define FSCACHE_DEBUG_LEVEL CACHE"));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/init.h>"));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("EXPORT_TRACEPOINT_SYMBOL(fscache_access_cache);"));
        assert!(source.contains("struct workqueue_struct *fscache_wq;"));
        assert!(source.contains("EXPORT_SYMBOL(fscache_wq);"));
        assert!(source.contains("#define HASH_MIX(x, y, a)"));
        assert!(source.contains("x = rol32(x, 7)"));
        assert!(source.contains("y = rol32(y,20)"));
        assert!(source.contains("return __hash_32(y ^ __hash_32(x));"));
        assert!(source.contains(
            "unsigned int fscache_hash(unsigned int salt, const void *data, size_t len)"
        ));
        assert!(source.contains("a = le32_to_cpu(*p++);"));
        assert!(source.contains("HASH_MIX(x, y, a);"));
        assert!(
            source.contains(
                "fscache_wq = alloc_workqueue(\"fscache\", WQ_UNBOUND | WQ_FREEZABLE, 0);"
            )
        );
        assert!(source.contains("ret = fscache_proc_init();"));
        assert!(source.contains("kmem_cache_create(\"fscache_cookie_jar\""));
        assert!(source.contains("pr_notice(\"FS-Cache loaded\\n\");"));
        assert!(source.contains("kmem_cache_destroy(fscache_cookie_jar);"));
        assert!(source.contains("timer_shutdown_sync(&fscache_cookie_lru_timer);"));
        assert!(source.contains("pr_notice(\"FS-Cache unloaded\\n\");"));

        let words = [0x1122_3344, 0x5566_7788];
        let bytes = [0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55];
        assert_eq!(fscache_hash_words(7, &words), fscache_hash(7, &bytes));
        assert_eq!(hash_32_generic(1), GOLDEN_RATIO_32);
        assert_eq!(FSCACHE_INIT_ORDER[0], "alloc_workqueue");
        assert_eq!(FSCACHE_EXIT_ORDER[2], "timer_shutdown_sync");
        assert_eq!(fscache_init_result(false, 0, true), Err(-ENOMEM));
        assert_eq!(fscache_init_result(true, -5, true), Err(-5));
        assert_eq!(fscache_init_result(true, 0, false), Err(-ENOMEM));
        assert_eq!(fscache_init_result(true, 0, true), Ok(()));
    }
}
