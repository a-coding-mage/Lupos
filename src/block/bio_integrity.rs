//! linux-parity: partial
//! linux-source: vendor/linux/block/bio-integrity.c
//! test-origin: linux:vendor/linux/block/bio-integrity.c
//! BIO integrity payload allocation, action selection, and vector helpers.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

pub const BLK_INTEGRITY_NOVERIFY: u32 = 1 << 0;
pub const BLK_INTEGRITY_NOGENERATE: u32 = 1 << 1;
pub const BLK_INTEGRITY_DEVICE_CAPABLE: u32 = 1 << 2;
pub const BLK_INTEGRITY_REF_TAG: u32 = 1 << 3;
pub const BLK_INTEGRITY_STACKED: u32 = 1 << 4;
pub const BLK_SPLIT_INTERVAL_CAPABLE: u32 = 1 << 5;

pub const BIP_BLOCK_INTEGRITY: u16 = 1 << 0;
pub const BIP_MAPPED_INTEGRITY: u16 = 1 << 1;
pub const BIP_DISK_NOCHECK: u16 = 1 << 2;
pub const BIP_IP_CHECKSUM: u16 = 1 << 3;
pub const BIP_COPY_USER: u16 = 1 << 4;
pub const BIP_CHECK_GUARD: u16 = 1 << 5;
pub const BIP_CHECK_REFTAG: u16 = 1 << 6;
pub const BIP_CHECK_APPTAG: u16 = 1 << 7;
pub const BIP_MEMPOOL: u16 = 1 << 15;
pub const BIP_CLONE_FLAGS: u16 =
    BIP_MAPPED_INTEGRITY | BIP_IP_CHECKSUM | BIP_CHECK_GUARD | BIP_CHECK_REFTAG | BIP_CHECK_APPTAG;

pub const BI_ACT_BUFFER: u32 = 1 << 0;
pub const BI_ACT_CHECK: u32 = 1 << 1;
pub const BI_ACT_ZERO: u32 = 1 << 2;

pub const REQ_OP_READ: u8 = 0;
pub const REQ_OP_WRITE: u8 = 1;
pub const REQ_OP_DISCARD: u8 = 3;
pub const REQ_INTEGRITY: u64 = 1 << 16;
pub const BIO_POOL_SIZE: usize = 2;

pub const BLK_INTEGRITY_CSUM_IP: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlkIntegrity {
    pub flags: u32,
    pub interval_exp: u8,
    pub metadata_size: u32,
    pub pi_tuple_size: u32,
    pub csum_type: u8,
    pub max_integrity_segments: u16,
}

impl BlkIntegrity {
    pub const fn offload_capable(self) -> bool {
        self.metadata_size == self.pi_tuple_size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BioVec {
    pub page_id: usize,
    pub len: u32,
    pub offset: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BioIntegrityPayload {
    pub iter_sector: u64,
    pub iter_size: u32,
    pub vcnt: u16,
    pub max_vcnt: u16,
    pub flags: u16,
    pub app_tag: u16,
    pub vecs: Vec<BioVec>,
}

impl BioIntegrityPayload {
    pub fn new(max_vcnt: u16) -> Self {
        Self {
            iter_sector: 0,
            iter_size: 0,
            vcnt: 0,
            max_vcnt,
            flags: 0,
            app_tag: 0,
            vecs: Vec::new(),
        }
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.iter_sector = seed;
    }

    pub fn seed(&self) -> u64 {
        self.iter_sector
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrityBio {
    pub op: u8,
    pub sectors: u32,
    pub sector: u64,
    pub op_flags: u64,
    pub status: i32,
    pub has_crypt_ctx: bool,
    pub integrity: Option<BlkIntegrity>,
    pub payload: Option<BioIntegrityPayload>,
}

impl IntegrityBio {
    pub const fn new(op: u8, sectors: u32, sector: u64, integrity: Option<BlkIntegrity>) -> Self {
        Self {
            op,
            sectors,
            sector,
            op_flags: 0,
            status: 0,
            has_crypt_ctx: false,
            integrity,
            payload: None,
        }
    }

    pub fn has_integrity(&self) -> bool {
        self.op_flags & REQ_INTEGRITY != 0 && self.payload.is_some()
    }
}

pub fn bio_integrity_intervals(bi: BlkIntegrity, sectors: u32) -> u32 {
    if bi.interval_exp < 9 {
        return 0;
    }
    sectors >> (bi.interval_exp - 9)
}

pub fn bio_integrity_bytes(bi: BlkIntegrity, sectors: u32) -> u32 {
    bio_integrity_intervals(bi, sectors).saturating_mul(bi.metadata_size)
}

pub fn bio_integrity_action(bio: &IntegrityBio) -> u32 {
    if bio.integrity.is_none() || bio.payload.is_some() {
        return 0;
    }
    __bio_integrity_action(bio)
}

pub fn __bio_integrity_action(bio: &IntegrityBio) -> u32 {
    if bio.has_crypt_ctx {
        return 0;
    }
    let Some(bi) = bio.integrity else {
        return 0;
    };
    match bio.op {
        REQ_OP_READ => {
            if bi.flags & BLK_INTEGRITY_NOVERIFY != 0 {
                if bi.offload_capable() {
                    0
                } else {
                    BI_ACT_BUFFER
                }
            } else {
                BI_ACT_BUFFER | BI_ACT_CHECK
            }
        }
        REQ_OP_WRITE => {
            if bio.sectors == 0 {
                return 0;
            }
            if bi.flags & BLK_INTEGRITY_NOGENERATE != 0 {
                if bi.offload_capable() {
                    0
                } else {
                    BI_ACT_BUFFER | BI_ACT_ZERO
                }
            } else if bi.metadata_size > bi.pi_tuple_size {
                BI_ACT_BUFFER | BI_ACT_CHECK | BI_ACT_ZERO
            } else {
                BI_ACT_BUFFER | BI_ACT_CHECK
            }
        }
        _ => 0,
    }
}

pub fn bio_integrity_init(bio: &mut IntegrityBio, mut bip: BioIntegrityPayload, nr_vecs: u16) {
    bip.max_vcnt = nr_vecs;
    bio.payload = Some(bip);
    bio.op_flags |= REQ_INTEGRITY;
}

pub fn bio_integrity_alloc(
    bio: &mut IntegrityBio,
    nr_vecs: u16,
) -> Result<&mut BioIntegrityPayload, i32> {
    if bio.has_crypt_ctx {
        return Err(-EOPNOTSUPP);
    }
    bio_integrity_init(bio, BioIntegrityPayload::new(nr_vecs), nr_vecs);
    Ok(bio.payload.as_mut().expect("payload was just attached"))
}

pub fn bio_integrity_free(bio: &mut IntegrityBio) {
    bio.payload = None;
    bio.op_flags &= !REQ_INTEGRITY;
}

pub fn bio_integrity_alloc_buf(bio: &mut IntegrityBio, zero_buffer: bool) -> Result<u32, i32> {
    let bi = bio.integrity.ok_or(-EINVAL)?;
    let len = bio_integrity_bytes(bi, bio.sectors);
    let bip = bio.payload.as_mut().ok_or(-EINVAL)?;
    bip.vecs.clear();
    bip.vecs.push(BioVec {
        page_id: 0,
        len,
        offset: 0,
    });
    bip.vcnt = 1;
    bip.iter_size = len;
    if zero_buffer {
        bip.flags |= BIP_MEMPOOL;
    }
    Ok(len)
}

pub fn bio_integrity_free_buf(bip: &mut BioIntegrityPayload) {
    bip.vecs.clear();
    bip.vcnt = 0;
    bip.iter_size = 0;
    bip.flags &= !BIP_MEMPOOL;
}

pub fn bio_integrity_setup_default(bio: &mut IntegrityBio) -> Result<(), i32> {
    let bi = bio.integrity.ok_or(-EINVAL)?;
    let bip = bio.payload.as_mut().ok_or(-EINVAL)?;
    bip.set_seed(bio.sector);
    if bi.csum_type != 0 {
        bip.flags |= BIP_CHECK_GUARD;
        if bi.csum_type == BLK_INTEGRITY_CSUM_IP {
            bip.flags |= BIP_IP_CHECKSUM;
        }
    }
    if bi.flags & BLK_INTEGRITY_REF_TAG != 0 {
        bip.flags |= BIP_CHECK_REFTAG;
    }
    Ok(())
}

pub fn bio_integrity_add_page(
    bio: &mut IntegrityBio,
    page_id: usize,
    len: u32,
    offset: u32,
) -> Result<u32, i32> {
    let max_segments = bio.integrity.map_or(0, |bi| bi.max_integrity_segments);
    let bip = bio.payload.as_mut().ok_or(-EINVAL)?;
    if bip.vcnt > 0 {
        if let Some(last) = bip.vecs.last_mut() {
            let last_end = last.offset.saturating_add(last.len);
            if last.page_id == page_id && last_end == offset {
                last.len = last.len.saturating_add(len);
                bip.iter_size = bip.iter_size.saturating_add(len);
                return Ok(len);
            }
        }
        if bip.vcnt >= bip.max_vcnt.min(max_segments) {
            return Ok(0);
        }
    }

    bip.vecs.push(BioVec {
        page_id,
        len,
        offset,
    });
    bip.vcnt += 1;
    bip.iter_size = bip.iter_size.saturating_add(len);
    Ok(len)
}

pub fn bio_integrity_advance(bio: &mut IntegrityBio, bytes_done: u32) -> Result<u32, i32> {
    let bi = bio.integrity.ok_or(-EINVAL)?;
    let bytes = bio_integrity_bytes(bi, bytes_done >> 9);
    let intervals = bio_integrity_intervals(bi, bytes_done >> 9);
    let bip = bio.payload.as_mut().ok_or(-EINVAL)?;
    bip.iter_sector = bip.iter_sector.saturating_add(intervals as u64);
    bip.iter_size = bip.iter_size.saturating_sub(bytes);
    Ok(bytes)
}

pub fn bio_integrity_trim(bio: &mut IntegrityBio) -> Result<u32, i32> {
    let bi = bio.integrity.ok_or(-EINVAL)?;
    let size = bio_integrity_bytes(bi, bio.sectors);
    let bip = bio.payload.as_mut().ok_or(-EINVAL)?;
    bip.iter_size = size;
    Ok(size)
}

pub fn bio_integrity_clone(dst: &mut IntegrityBio, src: &IntegrityBio) -> Result<(), i32> {
    let src_bip = src.payload.as_ref().ok_or(-EINVAL)?;
    let mut cloned = BioIntegrityPayload::new(0);
    cloned.vecs = src_bip.vecs.clone();
    cloned.iter_sector = src_bip.iter_sector;
    cloned.iter_size = src_bip.iter_size;
    cloned.vcnt = src_bip.vcnt;
    cloned.flags = src_bip.flags & BIP_CLONE_FLAGS;
    cloned.app_tag = src_bip.app_tag;
    dst.payload = Some(cloned);
    dst.op_flags |= REQ_INTEGRITY;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(flags: u32, metadata_size: u32, tuple_size: u32) -> BlkIntegrity {
        BlkIntegrity {
            flags,
            interval_exp: 12,
            metadata_size,
            pi_tuple_size: tuple_size,
            csum_type: BLK_INTEGRITY_CSUM_IP,
            max_integrity_segments: 2,
        }
    }

    #[test]
    fn action_selection_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bio-integrity.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/blk-integrity.h"
        ));
        assert!(source.contains("__bio_integrity_action(struct bio *bio)"));
        assert!(source.contains("BLK_INTEGRITY_NOVERIFY"));
        assert!(source.contains("BLK_INTEGRITY_NOGENERATE"));
        assert!(source.contains("bio_integrity_setup_default"));
        assert!(header.contains("BI_ACT_BUFFER"));
        assert!(header.contains("bio_integrity_bytes"));

        let read = IntegrityBio::new(REQ_OP_READ, 8, 12, Some(profile(0, 8, 8)));
        assert_eq!(bio_integrity_action(&read), BI_ACT_BUFFER | BI_ACT_CHECK);

        let noverify_offload = IntegrityBio::new(
            REQ_OP_READ,
            8,
            12,
            Some(profile(BLK_INTEGRITY_NOVERIFY, 8, 8)),
        );
        assert_eq!(bio_integrity_action(&noverify_offload), 0);

        let write_extended = IntegrityBio::new(REQ_OP_WRITE, 8, 12, Some(profile(0, 16, 8)));
        assert_eq!(
            bio_integrity_action(&write_extended),
            BI_ACT_BUFFER | BI_ACT_CHECK | BI_ACT_ZERO
        );
    }

    #[test]
    fn payload_lifecycle_and_vectors_are_source_backed() {
        let mut bio = IntegrityBio::new(
            REQ_OP_WRITE,
            8,
            99,
            Some(profile(BLK_INTEGRITY_REF_TAG, 8, 8)),
        );
        bio_integrity_alloc(&mut bio, 2).unwrap();
        assert!(bio.has_integrity());
        assert_eq!(bio_integrity_alloc_buf(&mut bio, false), Ok(8));
        bio_integrity_setup_default(&mut bio).unwrap();
        let bip = bio.payload.as_ref().unwrap();
        assert_eq!(bip.seed(), 99);
        assert!(bip.flags & BIP_CHECK_GUARD != 0);
        assert!(bip.flags & BIP_CHECK_REFTAG != 0);

        assert_eq!(bio_integrity_add_page(&mut bio, 1, 4, 16), Ok(4));
        assert_eq!(bio_integrity_add_page(&mut bio, 1, 4, 20), Ok(4));
        assert_eq!(bio.payload.as_ref().unwrap().vcnt, 2);
        assert_eq!(bio_integrity_advance(&mut bio, 4096), Ok(8));
        assert_eq!(bio.payload.as_ref().unwrap().seed(), 100);

        let mut clone = IntegrityBio::new(REQ_OP_WRITE, 8, 99, bio.integrity);
        bio_integrity_clone(&mut clone, &bio).unwrap();
        assert_eq!(
            clone.payload.as_ref().unwrap().flags,
            bio.payload.as_ref().unwrap().flags & BIP_CLONE_FLAGS
        );
        bio_integrity_free(&mut bio);
        assert!(!bio.has_integrity());
    }
}
