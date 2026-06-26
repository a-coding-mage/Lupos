//! linux-parity: complete
//! linux-source: vendor/linux/net/xfrm/xfrm_hash.c
//! test-origin: linux:vendor/linux/net/xfrm/xfrm_hash.c
//! XFRM hash table allocation strategy.

pub const PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XfrmHashAllocation {
    Kzalloc,
    Vzalloc,
    GetFreePages { order: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XfrmHashFree {
    Kfree,
    Vfree,
    FreePages { order: u32 },
}

pub fn get_order(size: usize) -> u32 {
    let pages = size.div_ceil(PAGE_SIZE).max(1);
    usize::BITS - (pages - 1).leading_zeros()
}

pub fn xfrm_hash_alloc_strategy(size: usize, hashdist: bool) -> XfrmHashAllocation {
    if size <= PAGE_SIZE {
        XfrmHashAllocation::Kzalloc
    } else if hashdist {
        XfrmHashAllocation::Vzalloc
    } else {
        XfrmHashAllocation::GetFreePages {
            order: get_order(size),
        }
    }
}

pub fn xfrm_hash_free_strategy(size: usize, hashdist: bool) -> XfrmHashFree {
    if size <= PAGE_SIZE {
        XfrmHashFree::Kfree
    } else if hashdist {
        XfrmHashFree::Vfree
    } else {
        XfrmHashFree::FreePages {
            order: get_order(size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfrm_hash_allocator_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/xfrm/xfrm_hash.c"
        ));
        assert!(source.contains("struct hlist_head *xfrm_hash_alloc(unsigned int sz)"));
        assert!(source.contains("if (sz <= PAGE_SIZE)"));
        assert!(source.contains("n = kzalloc(sz, GFP_KERNEL);"));
        assert!(source.contains("else if (hashdist)"));
        assert!(source.contains("n = vzalloc(sz);"));
        assert!(source.contains("__get_free_pages"));
        assert!(source.contains("get_order(sz)"));
        assert!(source.contains("void xfrm_hash_free"));
        assert!(source.contains("kfree(n);"));
        assert!(source.contains("vfree(n);"));
        assert!(source.contains("free_pages((unsigned long)n, get_order(sz));"));

        assert_eq!(
            xfrm_hash_alloc_strategy(PAGE_SIZE, false),
            XfrmHashAllocation::Kzalloc
        );
        assert_eq!(
            xfrm_hash_alloc_strategy(PAGE_SIZE + 1, true),
            XfrmHashAllocation::Vzalloc
        );
        assert_eq!(
            xfrm_hash_alloc_strategy(PAGE_SIZE * 4, false),
            XfrmHashAllocation::GetFreePages { order: 2 }
        );
        assert_eq!(
            xfrm_hash_free_strategy(PAGE_SIZE * 4, false),
            XfrmHashFree::FreePages { order: 2 }
        );
    }
}
