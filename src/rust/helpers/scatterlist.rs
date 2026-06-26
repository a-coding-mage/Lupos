//! linux-parity: complete
//! linux-source: vendor/linux/rust/helpers/scatterlist.c
//! test-origin: linux:vendor/linux/rust/helpers/scatterlist.c
//! Rust helper shims for scatterlist DMA accessors.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScatterlistHelper {
    pub symbol: &'static str,
    pub forwards_to: &'static str,
}

pub const INCLUDE_LINE: &str = "#include <linux/dma-direction.h>";
pub const HELPERS: &[ScatterlistHelper] = &[
    ScatterlistHelper {
        symbol: "rust_helper_sg_dma_address",
        forwards_to: "sg_dma_address(sg)",
    },
    ScatterlistHelper {
        symbol: "rust_helper_sg_dma_len",
        forwards_to: "sg_dma_len(sg)",
    },
    ScatterlistHelper {
        symbol: "rust_helper_sg_next",
        forwards_to: "sg_next(sg)",
    },
    ScatterlistHelper {
        symbol: "rust_helper_dma_unmap_sgtable",
        forwards_to: "dma_unmap_sgtable(dev, sgt, dir, attrs)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_scatterlist_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/rust/helpers/scatterlist.c"
        ));
        assert!(source.contains(INCLUDE_LINE));
        for helper in HELPERS {
            assert!(source.contains("__rust_helper"));
            assert!(source.contains(helper.symbol));
            assert!(source.contains(helper.forwards_to));
        }
    }
}
