//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/drm.c
//! test-origin: linux:vendor/linux/rust/helpers/drm.c
//! Rust helper shims for DRM GEM and GEM shmem operations.

use super::RustHelperSource;

pub const SOURCES: &[RustHelperSource] = &[
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem.h>",
        helper_symbol: "rust_helper_drm_gem_object_get",
        forwards_to: "drm_gem_object_get(obj)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem.h>",
        helper_symbol: "rust_helper_drm_gem_object_put",
        forwards_to: "drm_gem_object_put(obj)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_vma_manager.h>",
        helper_symbol: "rust_helper_drm_vma_node_offset_addr",
        forwards_to: "drm_vma_node_offset_addr(node)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_free",
        forwards_to: "drm_gem_shmem_object_free(obj)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_print_info",
        forwards_to: "drm_gem_shmem_object_print_info(p, indent, obj)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_pin",
        forwards_to: "drm_gem_shmem_object_pin(obj)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_unpin",
        forwards_to: "drm_gem_shmem_object_unpin(obj)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_get_sg_table",
        forwards_to: "drm_gem_shmem_object_get_sg_table(obj)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_vmap",
        forwards_to: "drm_gem_shmem_object_vmap(obj, map)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_vunmap",
        forwards_to: "drm_gem_shmem_object_vunmap(obj, map)",
    },
    RustHelperSource {
        linux_source: "vendor/linux/rust/helpers/drm.c",
        include_line: "#include <drm/drm_gem_shmem_helper.h>",
        helper_symbol: "rust_helper_drm_gem_shmem_object_mmap",
        forwards_to: "drm_gem_shmem_object_mmap(obj, vma)",
    },
];

pub fn sources() -> &'static [RustHelperSource] {
    SOURCES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/drm.c"
        ));
        for contract in SOURCES {
            super::super::assert_helper_source(source, *contract);
        }
    }
}
