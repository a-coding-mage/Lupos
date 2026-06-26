//! linux-parity: complete
//! linux-source: vendor/linux/block/bio-integrity-fs.c
//! test-origin: linux:vendor/linux/block/bio-integrity-fs.c
//! Filesystem BIO integrity buffer helpers.

use crate::include::uapi::errno::EINVAL;

pub const BI_ACT_BUFFER: u32 = 1 << 0;
pub const BI_ACT_CHECK: u32 = 1 << 1;
pub const BI_ACT_ZERO: u32 = 1 << 2;
pub const SECTOR_SHIFT: u32 = 9;
pub const BIO_POOL_SIZE: usize = 2;
pub const FS_BIO_INTEGRITY_CACHE_NAME: &str = "fs_bio_integrity";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlkIntegrity {
    pub interval_exp: u8,
    pub metadata_size: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsBioIntegrityPayload {
    pub vector_count: u8,
    pub iter_sector: u64,
    pub iter_size: u32,
    pub zeroed: bool,
    pub default_profile_setup: bool,
    pub generated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsBio {
    pub integrity_action: u32,
    pub integrity: Option<FsBioIntegrityPayload>,
    pub req_integrity: bool,
}

impl FsBio {
    pub const fn new(integrity_action: u32) -> Self {
        Self {
            integrity_action,
            integrity: None,
            req_integrity: false,
        }
    }
}

pub fn bio_integrity_bytes(bi: BlkIntegrity, sectors: u32) -> u32 {
    if bi.interval_exp < SECTOR_SHIFT as u8 {
        return 0;
    }
    (sectors >> (bi.interval_exp as u32 - SECTOR_SHIFT)) * bi.metadata_size
}

pub fn fs_bio_integrity_alloc(bio: &mut FsBio) -> u32 {
    let action = bio.integrity_action;
    if action == 0 {
        return 0;
    }

    bio.integrity = Some(FsBioIntegrityPayload {
        vector_count: 1,
        iter_sector: 0,
        iter_size: 0,
        zeroed: action & BI_ACT_ZERO != 0,
        default_profile_setup: action & BI_ACT_CHECK != 0,
        generated: false,
    });
    bio.req_integrity = true;
    action
}

pub fn fs_bio_integrity_free(bio: &mut FsBio) {
    bio.integrity = None;
    bio.req_integrity = false;
}

pub fn fs_bio_integrity_generate(bio: &mut FsBio) -> bool {
    if fs_bio_integrity_alloc(bio) == 0 {
        return false;
    }
    if let Some(payload) = bio.integrity.as_mut() {
        payload.generated = true;
    }
    true
}

pub fn fs_bio_integrity_verify(
    bio: &mut FsBio,
    bi: BlkIntegrity,
    sector: u64,
    size: u32,
) -> Result<u32, i32> {
    let Some(payload) = bio.integrity.as_mut() else {
        return Err(-EINVAL);
    };
    payload.iter_sector = sector;
    payload.iter_size = bio_integrity_bytes(bi, size >> SECTOR_SHIFT);
    Ok(payload.iter_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_bio_integrity_lifecycle_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bio-integrity-fs.c"
        ));
        assert!(source.contains("struct fs_bio_integrity_buf"));
        assert!(source.contains("action = bio_integrity_action(bio);"));
        assert!(source.contains("bio_integrity_init(bio, &iib->bip, &iib->bvec, 1);"));
        assert!(source.contains("bio_integrity_alloc_buf(bio, action & BI_ACT_ZERO);"));
        assert!(source.contains("if (action & BI_ACT_CHECK)"));
        assert!(source.contains("bio_integrity_setup_default(bio);"));
        assert!(source.contains("bio->bi_integrity = NULL;"));
        assert!(source.contains("bio->bi_opf &= ~REQ_INTEGRITY;"));
        assert!(source.contains("bio_integrity_generate(bio);"));
        assert!(source.contains("bip->bip_iter.bi_sector = sector;"));
        assert!(source.contains("bio_integrity_bytes(bi, size >> SECTOR_SHIFT);"));
        assert!(source.contains("kmem_cache_create(\"fs_bio_integrity\""));
        assert!(source.contains("mempool_init_slab_pool(&fs_bio_integrity_pool, BIO_POOL_SIZE"));

        let mut bio = FsBio::new(BI_ACT_BUFFER | BI_ACT_CHECK | BI_ACT_ZERO);
        assert_eq!(fs_bio_integrity_alloc(&mut bio), 7);
        let payload = bio.integrity.expect("payload allocated");
        assert_eq!(payload.vector_count, 1);
        assert!(payload.zeroed);
        assert!(payload.default_profile_setup);
        assert!(bio.req_integrity);

        let bi = BlkIntegrity {
            interval_exp: 12,
            metadata_size: 8,
        };
        assert_eq!(fs_bio_integrity_verify(&mut bio, bi, 8, 4096), Ok(8));
        assert_eq!(bio.integrity.unwrap().iter_sector, 8);

        fs_bio_integrity_free(&mut bio);
        assert_eq!(bio.integrity, None);
        assert!(!bio.req_integrity);

        let mut generated = FsBio::new(BI_ACT_BUFFER | BI_ACT_CHECK);
        assert!(fs_bio_integrity_generate(&mut generated));
        assert!(generated.integrity.unwrap().generated);
    }
}
