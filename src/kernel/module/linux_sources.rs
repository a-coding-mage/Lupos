//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module
//! test-origin: linux:vendor/linux/kernel/module
//! Linux module loader (kernel/module) auxiliary source coverage.
//!
//! Linux source inventory for this subsystem. These references are source
//! truth for module-loader ABI glue, Kbuild staging, and parity audits;
//! driver implementations remain vendor-built Linux artifacts.
//!
//! Refs:
//! - `vendor/linux/kernel/module/{debug_kmemleak,decompress,dups,kallsyms,kdb,kmod,livepatch,procfs,signing,stats,strict_rwx,sysfs,tracking,tree_lookup,version}.c`

/// Number of Linux `.c` files catalogued for this subsystem.
pub const MODULE_LOADER_SOURCES_COUNT: usize = 15;

/// Catalogued upstream Linux source paths used as source truth.
pub const MODULE_LOADER_SOURCES: &[&str] = &[
    "vendor/linux/kernel/module/debug_kmemleak.c",
    "vendor/linux/kernel/module/decompress.c",
    "vendor/linux/kernel/module/dups.c",
    "vendor/linux/kernel/module/kallsyms.c",
    "vendor/linux/kernel/module/kdb.c",
    "vendor/linux/kernel/module/kmod.c",
    "vendor/linux/kernel/module/livepatch.c",
    "vendor/linux/kernel/module/procfs.c",
    "vendor/linux/kernel/module/signing.c",
    "vendor/linux/kernel/module/stats.c",
    "vendor/linux/kernel/module/strict_rwx.c",
    "vendor/linux/kernel/module/sysfs.c",
    "vendor/linux/kernel/module/tracking.c",
    "vendor/linux/kernel/module/tree_lookup.c",
    "vendor/linux/kernel/module/version.c",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_table() {
        assert_eq!(MODULE_LOADER_SOURCES.len(), MODULE_LOADER_SOURCES_COUNT);
    }

    #[test]
    fn all_paths_have_canonical_prefix() {
        for path in MODULE_LOADER_SOURCES {
            assert!(path.starts_with("vendor/linux/kernel/module/"), "{path}");
            assert!(path.ends_with(".c"));
        }
    }
}
