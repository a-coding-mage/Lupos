//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/xfs_bio_io.c
//! test-origin: linux:vendor/linux/fs/xfs/xfs_bio_io.c
//! XFS block-device metadata I/O path selection.

pub const PAGE_SIZE: usize = 4096;
pub const REQ_OP_BITS: u32 = 8;
pub const REQ_OP_READ: u32 = 0;
pub const REQ_OP_WRITE: u32 = 1;
pub const REQ_SYNC: u32 = 1 << (REQ_OP_BITS + 3);
pub const REQ_META: u32 = 1 << (REQ_OP_BITS + 4);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XfsRwBdevPath {
    DirectBdevRwVirt,
    VmallocBio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsRwBdevPlan {
    pub path: XfsRwBdevPath,
    pub op_flags: u32,
    pub bio_max_vecs: usize,
    pub invalidate_vmap_after_read: bool,
}

pub const fn howmany_pages(bytes: usize) -> usize {
    if bytes == 0 {
        0
    } else {
        (bytes + PAGE_SIZE - 1) / PAGE_SIZE
    }
}

pub fn xfs_rw_bdev_plan(is_vmalloc: bool, count: usize, op: u32) -> XfsRwBdevPlan {
    let op_flags = op | REQ_META | REQ_SYNC;
    if !is_vmalloc {
        return XfsRwBdevPlan {
            path: XfsRwBdevPath::DirectBdevRwVirt,
            op_flags,
            bio_max_vecs: 0,
            invalidate_vmap_after_read: false,
        };
    }

    XfsRwBdevPlan {
        path: XfsRwBdevPath::VmallocBio,
        op_flags,
        bio_max_vecs: howmany_pages(count),
        invalidate_vmap_after_read: op_flags == REQ_OP_READ,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_bio_io_path_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/xfs_bio_io.c"
        ));
        assert!(source.contains("#include \"xfs_platform.h\""));
        assert!(source.contains("static inline unsigned int bio_max_vecs"));
        assert!(source.contains("bio_max_segs(howmany(count, PAGE_SIZE))"));
        assert!(source.contains("xfs_rw_bdev"));
        assert!(source.contains("op |= REQ_META | REQ_SYNC;"));
        assert!(source.contains("if (!is_vmalloc_addr(data))"));
        assert!(source.contains("return bdev_rw_virt(bdev, sector, data, count, op);"));
        assert!(source.contains("bio_alloc(bdev, bio_max_vecs(count), op, GFP_KERNEL);"));
        assert!(source.contains("bio_add_vmalloc_chunk(bio, data + done, count - done);"));
        assert!(source.contains("bio_chain(prev, bio);"));
        assert!(source.contains("submit_bio(prev);"));
        assert!(source.contains("error = submit_bio_wait(bio);"));
        assert!(source.contains("bio_put(bio);"));
        assert!(source.contains("invalidate_kernel_vmap_range(data, count);"));

        assert_eq!(howmany_pages(PAGE_SIZE + 1), 2);
        let direct = xfs_rw_bdev_plan(false, 8192, REQ_OP_WRITE);
        assert_eq!(direct.path, XfsRwBdevPath::DirectBdevRwVirt);
        assert_eq!(direct.bio_max_vecs, 0);
        let bio = xfs_rw_bdev_plan(true, PAGE_SIZE + 1, REQ_OP_READ);
        assert_eq!(bio.path, XfsRwBdevPath::VmallocBio);
        assert_eq!(bio.bio_max_vecs, 2);
        assert_eq!(bio.op_flags, REQ_OP_READ | REQ_META | REQ_SYNC);
        assert!(!bio.invalidate_vmap_after_read);
    }
}
