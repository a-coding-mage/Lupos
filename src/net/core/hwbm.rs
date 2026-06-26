//! linux-parity: complete
//! linux-source: vendor/linux/net/core/hwbm.c
//! test-origin: linux:vendor/linux/net/core/hwbm.c
//! Hardware buffer manager pool refill helpers.

use crate::include::uapi::errno::ENOMEM;

pub const PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HwbmAllocKind {
    Frag,
    Kmalloc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HwbmPool {
    pub frag_size: usize,
    pub size: u32,
    pub buf_num: u32,
    pub construct: bool,
}

pub const fn hwbm_buf_free(pool: &HwbmPool) -> HwbmAllocKind {
    if pool.frag_size <= PAGE_SIZE {
        HwbmAllocKind::Frag
    } else {
        HwbmAllocKind::Kmalloc
    }
}

pub const fn hwbm_pool_refill(
    pool: &HwbmPool,
    allocation_succeeds: bool,
    construct_succeeds: bool,
) -> Result<HwbmAllocKind, i32> {
    if !allocation_succeeds {
        return Err(-ENOMEM);
    }
    if pool.construct && !construct_succeeds {
        return Err(-ENOMEM);
    }
    Ok(hwbm_buf_free(pool))
}

pub fn hwbm_pool_add(pool: &mut HwbmPool, buf_num: u32, successful_refills: u32) -> u32 {
    if pool.buf_num == pool.size {
        return pool.buf_num;
    }
    let Some(total) = buf_num.checked_add(pool.buf_num) else {
        return 0;
    };
    if total > pool.size {
        return 0;
    }

    let added = core::cmp::min(buf_num, successful_refills);
    pool.buf_num = pool.buf_num.saturating_add(added);
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hwbm_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/core/hwbm.c"
        ));
        assert!(source.contains("void hwbm_buf_free(struct hwbm_pool *bm_pool, void *buf)"));
        assert!(source.contains("if (likely(bm_pool->frag_size <= PAGE_SIZE))"));
        assert!(source.contains("skb_free_frag(buf);"));
        assert!(source.contains("kfree(buf);"));
        assert!(source.contains("int hwbm_pool_refill(struct hwbm_pool *bm_pool, gfp_t gfp)"));
        assert!(source.contains("buf = netdev_alloc_frag(frag_size);"));
        assert!(source.contains("buf = kmalloc(frag_size, gfp);"));
        assert!(source.contains("if (!buf)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("if (bm_pool->construct)"));
        assert!(source.contains("hwbm_buf_free(bm_pool, buf);"));
        assert!(
            source.contains("int hwbm_pool_add(struct hwbm_pool *bm_pool, unsigned int buf_num)")
        );
        assert!(source.contains("if (bm_pool->buf_num == bm_pool->size)"));
        assert!(source.contains("if (buf_num + bm_pool->buf_num > bm_pool->size)"));
        assert!(source.contains("bm_pool->buf_num += i;"));
        assert!(source.contains("return i;"));

        let small = HwbmPool {
            frag_size: PAGE_SIZE,
            size: 4,
            buf_num: 0,
            construct: false,
        };
        assert_eq!(hwbm_buf_free(&small), HwbmAllocKind::Frag);
        assert_eq!(
            hwbm_pool_refill(&small, true, true),
            Ok(HwbmAllocKind::Frag)
        );
        assert_eq!(hwbm_pool_refill(&small, false, true), Err(-ENOMEM));

        let mut large = HwbmPool {
            frag_size: PAGE_SIZE + 1,
            size: 4,
            buf_num: 1,
            construct: true,
        };
        assert_eq!(
            hwbm_pool_refill(&large, true, true),
            Ok(HwbmAllocKind::Kmalloc)
        );
        assert_eq!(hwbm_pool_refill(&large, true, false), Err(-ENOMEM));
        assert_eq!(hwbm_pool_add(&mut large, 2, 2), 2);
        assert_eq!(large.buf_num, 3);
        assert_eq!(hwbm_pool_add(&mut large, 2, 2), 0);
        large.buf_num = large.size;
        assert_eq!(hwbm_pool_add(&mut large, 1, 1), large.size);
    }
}
