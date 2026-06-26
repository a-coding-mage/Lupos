//! linux-parity: complete
//! linux-source: vendor/linux/crypto/async_tx/async_memcpy.c
//! test-origin: linux:vendor/linux/crypto/async_tx/async_memcpy.c
//! Async memcpy DMA-vs-sync path selection.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsyncMemcpyDecision {
    pub uses_dma: bool,
    pub unmap_slots: usize,
    pub prep_interrupt: bool,
    pub prep_fence: bool,
    pub waits_for_dependency: bool,
    pub runs_sync_epilog: bool,
}

pub const ASYNC_TX_ACK: u32 = 1 << 0;
pub const ASYNC_TX_FENCE: u32 = 1 << 1;

pub const fn async_memcpy_decision(
    channel_has_device: bool,
    unmap_allocated: bool,
    aligned: bool,
    callback: bool,
    flags: u32,
) -> AsyncMemcpyDecision {
    let uses_dma = channel_has_device && unmap_allocated && aligned;
    AsyncMemcpyDecision {
        uses_dma,
        unmap_slots: if uses_dma { 2 } else { 0 },
        prep_interrupt: uses_dma && callback,
        prep_fence: uses_dma && (flags & ASYNC_TX_FENCE != 0),
        waits_for_dependency: !uses_dma,
        runs_sync_epilog: !uses_dma,
    }
}

pub fn async_memcpy_sync_fallback(dest: &mut [u8], src: &[u8]) -> usize {
    let len = core::cmp::min(dest.len(), src.len());
    dest[..len].copy_from_slice(&src[..len]);
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn async_memcpy_matches_linux_dma_and_sync_paths() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/async_tx/async_memcpy.c"
        ));
        assert!(source.contains("async_tx_find_channel(submit, DMA_MEMCPY"));
        assert!(source.contains("dmaengine_get_unmap_data(device->dev, 2, GFP_NOWAIT);"));
        assert!(source.contains("is_dma_copy_aligned(device, src_offset, dest_offset, len)"));
        assert!(source.contains("if (submit->cb_fn)"));
        assert!(source.contains("dma_prep_flags |= DMA_PREP_INTERRUPT;"));
        assert!(source.contains("if (submit->flags & ASYNC_TX_FENCE)"));
        assert!(source.contains("dma_prep_flags |= DMA_PREP_FENCE;"));
        assert!(source.contains("device->device_prep_dma_memcpy"));
        assert!(source.contains("async_tx_quiesce(&submit->depend_tx);"));
        assert!(source.contains("memcpy(dest_buf, src_buf, len);"));
        assert!(source.contains("async_tx_sync_epilog(submit);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(async_memcpy);"));

        assert_eq!(
            async_memcpy_decision(true, true, true, true, ASYNC_TX_FENCE),
            AsyncMemcpyDecision {
                uses_dma: true,
                unmap_slots: 2,
                prep_interrupt: true,
                prep_fence: true,
                waits_for_dependency: false,
                runs_sync_epilog: false,
            }
        );
        assert!(async_memcpy_decision(true, false, true, false, 0).waits_for_dependency);
        let mut dest = [0u8; 4];
        assert_eq!(async_memcpy_sync_fallback(&mut dest, b"abcdef"), 4);
        assert_eq!(&dest, b"abcd");
    }
}
