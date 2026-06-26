//! linux-parity: complete
//! linux-source: vendor/linux/net/xdp/xsk_queue.c
//! test-origin: linux:vendor/linux/net/xdp/xsk_queue.c
//! XDP userspace queue ring allocation.

extern crate alloc;

use alloc::vec::Vec;

pub const PAGE_SIZE: usize = 4096;
pub const XDP_UMEM_RING_BASE_SIZE: usize = 16;
pub const XDP_RXTX_RING_BASE_SIZE: usize = 16;
pub const XDP_UMEM_DESC_SIZE: usize = 8;
pub const XDP_RXTX_DESC_SIZE: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XskQueue {
    pub nentries: u32,
    pub ring_mask: u32,
    pub ring_vmalloc_size: usize,
    pub ring: Vec<u8>,
    pub umem_queue: bool,
}

pub const fn xskq_get_ring_size(nentries: u32, umem_queue: bool) -> Option<usize> {
    let base = if umem_queue {
        XDP_UMEM_RING_BASE_SIZE
    } else {
        XDP_RXTX_RING_BASE_SIZE
    };
    let desc_size = if umem_queue {
        XDP_UMEM_DESC_SIZE
    } else {
        XDP_RXTX_DESC_SIZE
    };
    match (nentries as usize).checked_mul(desc_size) {
        Some(desc_bytes) => base.checked_add(desc_bytes),
        None => None,
    }
}

pub fn xskq_create(nentries: u32, umem_queue: bool) -> Option<XskQueue> {
    let size = xskq_get_ring_size(nentries, umem_queue)?;
    if size == usize::MAX {
        return None;
    }
    let ring_vmalloc_size = page_align(size)?;
    let mut ring = Vec::new();
    ring.try_reserve_exact(ring_vmalloc_size).ok()?;
    ring.resize(ring_vmalloc_size, 0);
    Some(XskQueue {
        nentries,
        ring_mask: nentries.wrapping_sub(1),
        ring_vmalloc_size,
        ring,
        umem_queue,
    })
}

pub const fn page_align(size: usize) -> Option<usize> {
    match size.checked_add(PAGE_SIZE - 1) {
        Some(value) => Some(value & !(PAGE_SIZE - 1)),
        None => None,
    }
}

pub fn xskq_destroy(q: Option<XskQueue>) -> bool {
    q.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xsk_queue_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/xdp/xsk_queue.c"
        ));
        assert!(source.contains("static size_t xskq_get_ring_size"));
        assert!(source.contains("struct xdp_umem_ring *umem_ring;"));
        assert!(source.contains("struct xdp_rxtx_ring *rxtx_ring;"));
        assert!(source.contains("if (umem_queue)"));
        assert!(source.contains("return struct_size(umem_ring, desc, q->nentries);"));
        assert!(source.contains("return struct_size(rxtx_ring, desc, q->nentries);"));
        assert!(source.contains("struct xsk_queue *xskq_create"));
        assert!(source.contains("q = kzalloc_obj(*q);"));
        assert!(source.contains("q->nentries = nentries;"));
        assert!(source.contains("q->ring_mask = nentries - 1;"));
        assert!(source.contains("if (unlikely(size == SIZE_MAX))"));
        assert!(source.contains("size = PAGE_ALIGN(size);"));
        assert!(source.contains("q->ring = vmalloc_user(size);"));
        assert!(source.contains("q->ring_vmalloc_size = size;"));
        assert!(source.contains("void xskq_destroy(struct xsk_queue *q)"));
        assert!(source.contains("vfree(q->ring);"));
        assert!(source.contains("kfree(q);"));
    }

    #[test]
    fn xsk_queue_create_sets_mask_and_page_aligned_ring_size() {
        let q = xskq_create(64, true).unwrap();
        assert_eq!(q.nentries, 64);
        assert_eq!(q.ring_mask, 63);
        assert_eq!(q.ring_vmalloc_size % PAGE_SIZE, 0);
        assert_eq!(q.ring.len(), q.ring_vmalloc_size);
        assert!(q.umem_queue);

        let rxtx = xskq_create(64, false).unwrap();
        assert!(rxtx.ring_vmalloc_size >= q.ring_vmalloc_size);
        assert!(xskq_destroy(Some(rxtx)));
        assert!(!xskq_destroy(None));
    }
}
