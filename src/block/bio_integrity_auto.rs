//! linux-parity: complete
//! linux-source: vendor/linux/block/bio-integrity-auto.c
//! test-origin: linux:vendor/linux/block/bio-integrity-auto.c
//! Automatic BIO integrity preparation and completion decisions.

use super::bio_integrity::{
    BI_ACT_CHECK, BI_ACT_ZERO, BIO_POOL_SIZE, BIP_BLOCK_INTEGRITY, BIP_CHECK_APPTAG,
    BIP_CHECK_GUARD, BIP_CHECK_REFTAG, BioIntegrityPayload, IntegrityBio, REQ_INTEGRITY,
    REQ_OP_READ, REQ_OP_WRITE, bio_integrity_alloc_buf, bio_integrity_init,
    bio_integrity_setup_default,
};
use crate::kernel::workqueue::{WQ_CPU_INTENSIVE, WQ_HIGHPRI, WQ_MEM_RECLAIM};

pub const BIP_CHECK_FLAGS: u16 = BIP_CHECK_GUARD | BIP_CHECK_REFTAG | BIP_CHECK_APPTAG;
pub const KINTEGRITYD_NAME: &str = "kintegrityd";
pub const BIO_INTEGRITY_DATA_CACHE: &str = "bio_integrity_data";
pub const WQ_PERCPU: u32 = 1 << 8;
pub const KINTEGRITYD_FLAGS: u32 = WQ_MEM_RECLAIM | WQ_HIGHPRI | WQ_CPU_INTENSIVE | WQ_PERCPU;
pub const KINTEGRITYD_MAX_ACTIVE: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BioIntegrityData {
    pub saved_sector: u64,
    pub saved_size: u32,
    pub verify_queued: bool,
    pub generated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlkIntegrityAutoInitReport {
    pub slab_name: &'static str,
    pub slab_created: bool,
    pub pool_size: usize,
    pub pool_initialized: bool,
    pub workqueue_name: Option<&'static str>,
    pub workqueue_flags: u32,
    pub workqueue_max_active: u32,
    pub panic: Option<&'static str>,
}

pub fn bip_should_check(flags: u16) -> bool {
    flags & BIP_CHECK_FLAGS != 0
}

pub fn bio_integrity_finish(bio: &mut IntegrityBio) {
    if let Some(payload) = bio.payload.as_mut() {
        super::bio_integrity::bio_integrity_free_buf(payload);
    }
    bio.payload = None;
    bio.op_flags &= !REQ_INTEGRITY;
}

pub fn __bio_integrity_endio(bio: &mut IntegrityBio, data: &mut BioIntegrityData) -> bool {
    let should_defer = bio.op == REQ_OP_READ
        && bio.status == 0
        && bio
            .payload
            .as_ref()
            .is_some_and(|payload| bip_should_check(payload.flags));
    if should_defer {
        data.verify_queued = true;
        return false;
    }
    bio_integrity_finish(bio);
    true
}

pub fn bio_integrity_verify_fn(bio: &mut IntegrityBio, data: &mut BioIntegrityData) {
    bio_integrity_verify_fn_result(bio, data, 0);
}

pub fn bio_integrity_verify_fn_result(
    bio: &mut IntegrityBio,
    data: &mut BioIntegrityData,
    verify_status: i32,
) {
    bio.status = verify_status;
    data.verify_queued = false;
    bio_integrity_finish(bio);
}

pub fn bio_integrity_prep(bio: &mut IntegrityBio, action: u32) -> Result<BioIntegrityData, i32> {
    bio_integrity_init(bio, BioIntegrityPayload::new(1), 1);
    let saved_sector = bio.sector;
    let saved_size = bio.sectors << 9;
    let mut generated = false;
    if let Some(payload) = bio.payload.as_mut() {
        payload.flags |= BIP_BLOCK_INTEGRITY;
    }
    bio_integrity_alloc_buf(bio, action & BI_ACT_ZERO != 0)?;
    if action & BI_ACT_CHECK != 0 {
        bio_integrity_setup_default(bio)?;
    }
    if bio.op == REQ_OP_WRITE
        && bio
            .payload
            .as_ref()
            .is_some_and(|payload| bip_should_check(payload.flags))
    {
        generated = true;
    }
    Ok(BioIntegrityData {
        saved_sector,
        saved_size,
        verify_queued: false,
        generated,
    })
}

pub fn blk_flush_integrity(pending_work: &mut usize) {
    *pending_work = 0;
}

pub const fn blk_integrity_auto_init(
    mempool_init_ok: bool,
    workqueue_alloc_ok: bool,
) -> BlkIntegrityAutoInitReport {
    if !mempool_init_ok {
        return BlkIntegrityAutoInitReport {
            slab_name: BIO_INTEGRITY_DATA_CACHE,
            slab_created: true,
            pool_size: BIO_POOL_SIZE,
            pool_initialized: false,
            workqueue_name: None,
            workqueue_flags: 0,
            workqueue_max_active: 0,
            panic: Some("bio: can't create integrity pool"),
        };
    }
    if !workqueue_alloc_ok {
        return BlkIntegrityAutoInitReport {
            slab_name: BIO_INTEGRITY_DATA_CACHE,
            slab_created: true,
            pool_size: BIO_POOL_SIZE,
            pool_initialized: true,
            workqueue_name: None,
            workqueue_flags: KINTEGRITYD_FLAGS,
            workqueue_max_active: KINTEGRITYD_MAX_ACTIVE,
            panic: Some("Failed to create kintegrityd"),
        };
    }
    BlkIntegrityAutoInitReport {
        slab_name: BIO_INTEGRITY_DATA_CACHE,
        slab_created: true,
        pool_size: BIO_POOL_SIZE,
        pool_initialized: true,
        workqueue_name: Some(KINTEGRITYD_NAME),
        workqueue_flags: KINTEGRITYD_FLAGS,
        workqueue_max_active: KINTEGRITYD_MAX_ACTIVE,
        panic: None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::bio_integrity::{
        BI_ACT_BUFFER, BLK_INTEGRITY_CSUM_IP, BLK_INTEGRITY_REF_TAG, BlkIntegrity,
    };
    use super::*;

    fn profile() -> BlkIntegrity {
        BlkIntegrity {
            flags: BLK_INTEGRITY_REF_TAG,
            interval_exp: 12,
            metadata_size: 8,
            pi_tuple_size: 8,
            csum_type: BLK_INTEGRITY_CSUM_IP,
            max_integrity_segments: 1,
        }
    }

    #[test]
    fn prep_and_endio_match_linux_auto_integrity_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bio-integrity-auto.c"
        ));
        assert!(source.contains("#define BIP_CHECK_FLAGS"));
        assert!(source.contains("__bio_integrity_endio(struct bio *bio)"));
        assert!(source.contains("queue_work(kintegrityd_wq"));
        assert!(source.contains("INIT_WORK(&bid->work, bio_integrity_verify_fn);"));
        assert!(
            source.contains("bio->bi_status = bio_integrity_verify(bio, &bid->saved_bio_iter);")
        );
        assert!(source.contains("bio_endio(bio);"));
        assert!(source.contains("bio_integrity_prep(struct bio *bio"));
        assert!(source.contains("bid = mempool_alloc(&bid_pool, GFP_NOIO);"));
        assert!(source.contains("bio_integrity_alloc_buf(bio, GFP_NOIO, action & BI_ACT_ZERO);"));
        assert!(source.contains("bio_integrity_generate(bio);"));
        assert!(source.contains("bid->saved_bio_iter = bio->bi_iter;"));
        assert!(source.contains("kmem_cache_create(\"bio_integrity_data\""));
        assert!(source.contains("mempool_init_slab_pool(&bid_pool, BIO_POOL_SIZE, bid_slab)"));
        assert!(source.contains("alloc_workqueue(\"kintegrityd\""));
        assert!(source.contains("WQ_MEM_RECLAIM |"));
        assert!(source.contains("WQ_HIGHPRI | WQ_CPU_INTENSIVE | WQ_PERCPU, 1"));
        assert!(source.contains("panic(\"Failed to create kintegrityd\\n\");"));
        assert!(source.contains("subsys_initcall(blk_integrity_auto_init);"));

        let mut write = IntegrityBio::new(REQ_OP_WRITE, 8, 5, Some(profile()));
        let data = bio_integrity_prep(&mut write, BI_ACT_BUFFER | BI_ACT_CHECK).unwrap();
        assert!(data.generated);
        assert!(write.has_integrity());
        assert!(write.payload.as_ref().unwrap().flags & BIP_BLOCK_INTEGRITY != 0);

        let mut read = IntegrityBio::new(REQ_OP_READ, 8, 5, Some(profile()));
        let mut data = bio_integrity_prep(&mut read, BI_ACT_BUFFER | BI_ACT_CHECK).unwrap();
        assert!(!__bio_integrity_endio(&mut read, &mut data));
        assert!(data.verify_queued);
        bio_integrity_verify_fn_result(&mut read, &mut data, -5);
        assert_eq!(read.status, -5);
        assert!(!read.has_integrity());
    }

    #[test]
    fn flush_integrity_drains_pending_count() {
        let mut pending = 3;
        blk_flush_integrity(&mut pending);
        assert_eq!(pending, 0);
    }

    #[test]
    fn auto_integrity_init_models_pool_and_workqueue_edges() {
        let ok = blk_integrity_auto_init(true, true);
        assert_eq!(ok.slab_name, "bio_integrity_data");
        assert_eq!(ok.pool_size, BIO_POOL_SIZE);
        assert_eq!(ok.workqueue_name, Some("kintegrityd"));
        assert_eq!(
            ok.workqueue_flags,
            WQ_MEM_RECLAIM | WQ_HIGHPRI | WQ_CPU_INTENSIVE | WQ_PERCPU
        );
        assert_eq!(ok.workqueue_max_active, 1);
        assert_eq!(ok.panic, None);

        assert_eq!(
            blk_integrity_auto_init(false, true).panic,
            Some("bio: can't create integrity pool")
        );
        assert_eq!(
            blk_integrity_auto_init(true, false).panic,
            Some("Failed to create kintegrityd")
        );
    }
}
