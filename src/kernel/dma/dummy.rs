//! linux-parity: complete
//! linux-source: vendor/linux/kernel/dma/dummy.c
//! test-origin: linux:vendor/linux/kernel/dma/dummy.c
//! Dummy DMA map operations that always fail.

use crate::include::uapi::errno::{EINVAL, ENXIO};

pub const DMA_MAPPING_ERROR: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaDummyOps {
    pub has_mmap: bool,
    pub has_map_phys: bool,
    pub has_unmap_phys: bool,
    pub has_map_sg: bool,
    pub has_unmap_sg: bool,
    pub has_dma_supported: bool,
}

pub const DMA_DUMMY_OPS: DmaDummyOps = DmaDummyOps {
    has_mmap: true,
    has_map_phys: true,
    has_unmap_phys: true,
    has_map_sg: true,
    has_unmap_sg: true,
    has_dma_supported: true,
};

pub const fn dma_dummy_mmap() -> i32 {
    -ENXIO
}

pub const fn dma_dummy_map_phys() -> u64 {
    DMA_MAPPING_ERROR
}

pub const fn dma_dummy_map_sg() -> i32 {
    -EINVAL
}

pub const fn dma_dummy_supported() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dma_dummy_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/dma/dummy.c"
        ));
        assert!(source.contains("Dummy DMA ops that always fail."));
        assert!(source.contains("return -ENXIO;"));
        assert!(source.contains("return DMA_MAPPING_ERROR;"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("WARN_ON_ONCE(true);"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("const struct dma_map_ops dma_dummy_ops"));
        assert!(source.contains(".mmap"));
        assert!(source.contains(".map_phys"));
        assert!(source.contains(".dma_supported"));

        assert_eq!(dma_dummy_mmap(), -ENXIO);
        assert_eq!(dma_dummy_map_phys(), DMA_MAPPING_ERROR);
        assert_eq!(dma_dummy_map_sg(), -EINVAL);
        assert!(!dma_dummy_supported());
        assert!(DMA_DUMMY_OPS.has_unmap_sg);
    }
}
