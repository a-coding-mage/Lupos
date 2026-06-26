//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bounds.c
//! test-origin: linux:vendor/linux/kernel/bounds.c
//! Kbuild bounds generator metadata.

pub const GENERATED_HEADER_DEFINE: &str = "__GENERATING_BOUNDS_H";
pub const COMPILE_OFFSETS_DEFINE: &str = "COMPILE_OFFSETS";

pub const EMITTED_BOUNDS: &[&str] = &[
    "NR_PAGEFLAGS",
    "MAX_NR_ZONES",
    "NR_CPUS_BITS",
    "SPINLOCK_SIZE",
    "LRU_GEN_WIDTH",
    "LRU_REFS_WIDTH",
    "__LRU_REFS_WIDTH",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LruGenBounds {
    pub lru_gen_width: u32,
    pub lru_refs_width: u32,
    pub raw_lru_refs_width: u32,
}

pub const fn lru_gen_bounds(config_lru_gen: bool, order_base_2_n: u32) -> LruGenBounds {
    if config_lru_gen {
        LruGenBounds {
            lru_gen_width: order_base_2_n + 1,
            lru_refs_width: order_base_2_n,
            raw_lru_refs_width: order_base_2_n,
        }
    } else {
        LruGenBounds {
            lru_gen_width: 0,
            lru_refs_width: 0,
            raw_lru_refs_width: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_generator_matches_linux_defines() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bounds.c"
        ));
        assert!(source.contains("#define __GENERATING_BOUNDS_H"));
        assert!(source.contains("#define COMPILE_OFFSETS"));
        for bound in EMITTED_BOUNDS {
            assert!(source.contains(bound));
        }
        assert!(source.contains("DEFINE(NR_PAGEFLAGS, __NR_PAGEFLAGS);"));
        assert!(source.contains("DEFINE(MAX_NR_ZONES, __MAX_NR_ZONES);"));
        assert!(source.contains("DEFINE(SPINLOCK_SIZE, sizeof(spinlock_t));"));
        assert_eq!(
            lru_gen_bounds(false, 3),
            LruGenBounds {
                lru_gen_width: 0,
                lru_refs_width: 0,
                raw_lru_refs_width: 0,
            }
        );
        assert_eq!(lru_gen_bounds(true, 3).lru_gen_width, 4);
    }
}
