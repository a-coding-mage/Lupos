//! linux-parity: partial
//! linux-source: vendor/linux/block
//! test-origin: linux:vendor/linux/block
//! Linux block-core ABI glue for vendor-built block drivers.
//!
//! This file mirrors core block-layer entry points from `vendor/linux/block/`
//! and Linux block headers. It must not contain a function driver such as
//! virtio-blk; those payloads are built from `vendor/linux/drivers/`.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use spin::Mutex;

use crate::block::bio::BioRef;
use crate::block::block_device::{
    BlockDevice, BlockDeviceOps, BlockDeviceRef, register_block_device, unregister_block_device,
};
use crate::block::gendisk::{register_gendisk, unregister_gendisk};
use crate::include::uapi::errno::{EBUSY, EINVAL, EIO, ENODEV, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};

pub const DISK_NAME_LEN: usize = 32;
pub const HCTX_MAX_TYPES: usize = 3;
pub const BLK_FEAT_ROTATIONAL: u32 = 1 << 2;
pub const GD_READ_ONLY: u32 = 1;
pub const GD_ADDED: u32 = 4;
pub const GD_OWNS_QUEUE: u32 = 6;
pub const LINUX_QUEUE_LIMITS_LOGICAL_BLOCK_SIZE_OFFSET: usize = 0x38;
pub const LINUX_REQUEST_QUEUE_DISK_OFFSET: usize = 0x60;
pub const LINUX_REQUEST_QUEUE_MQ_KOBJ_OFFSET: usize = 0x68;
pub const LINUX_REQUEST_QUEUE_LIMITS_OFFSET: usize = 0x70;
pub const LINUX_REQUEST_QUEUE_PM_ONLY_OFFSET: usize = 0x13c;
pub const LINUX_REQUEST_QUEUE_STATS_OFFSET: usize = 0x140;
pub const LINUX_REQUEST_QUEUE_RQ_QOS_OFFSET: usize = 0x148;
pub const LINUX_REQUEST_QUEUE_RQ_QOS_MUTEX_OFFSET: usize = 0x150;
pub const LINUX_REQUEST_QUEUE_ID_OFFSET: usize = 0x168;
pub const LINUX_REQUEST_QUEUE_NR_REQUESTS_OFFSET: usize = 0x16c;
pub const LINUX_REQUEST_QUEUE_ASYNC_DEPTH_OFFSET: usize = 0x170;
pub const LINUX_REQUEST_QUEUE_TIMEOUT_OFFSET: usize = 0x178;
pub const LINUX_REQUEST_QUEUE_TIMEOUT_WORK_OFFSET: usize = 0x1a0;
pub const LINUX_REQUEST_QUEUE_NR_ACTIVE_REQUESTS_SHARED_TAGS_OFFSET: usize = 0x1c0;
pub const LINUX_REQUEST_QUEUE_SCHED_SHARED_TAGS_OFFSET: usize = 0x1c8;
pub const LINUX_REQUEST_QUEUE_ICQ_LIST_OFFSET: usize = 0x1d0;
pub const LINUX_REQUEST_QUEUE_NODE_OFFSET: usize = 0x218;
pub const LINUX_REQUEST_QUEUE_REQUEUE_LOCK_OFFSET: usize = 0x21c;
pub const LINUX_REQUEST_QUEUE_REQUEUE_LIST_OFFSET: usize = 0x220;
pub const LINUX_REQUEST_QUEUE_REQUEUE_WORK_OFFSET: usize = 0x230;
pub const LINUX_REQUEST_QUEUE_FQ_OFFSET: usize = 0x288;
pub const LINUX_REQUEST_QUEUE_FLUSH_LIST_OFFSET: usize = 0x290;
pub const LINUX_REQUEST_QUEUE_ELEVATOR_LOCK_OFFSET: usize = 0x2a0;
pub const LINUX_REQUEST_QUEUE_SYSFS_LOCK_OFFSET: usize = 0x2b8;
pub const LINUX_REQUEST_QUEUE_LIMITS_LOCK_OFFSET: usize = 0x2d0;
pub const LINUX_REQUEST_QUEUE_UNUSED_HCTX_LIST_OFFSET: usize = 0x2e8;
pub const LINUX_REQUEST_QUEUE_UNUSED_HCTX_LOCK_OFFSET: usize = 0x2f8;
pub const LINUX_REQUEST_QUEUE_MQ_FREEZE_DEPTH_OFFSET: usize = 0x2fc;
pub const LINUX_REQUEST_QUEUE_RCU_HEAD_OFFSET: usize = 0x300;
pub const LINUX_REQUEST_QUEUE_MQ_FREEZE_WQ_OFFSET: usize = 0x310;
pub const LINUX_REQUEST_QUEUE_MQ_FREEZE_LOCK_OFFSET: usize = 0x328;
pub const LINUX_REQUEST_QUEUE_TAG_SET_OFFSET: usize = 0x340;
pub const LINUX_REQUEST_QUEUE_TAG_SET_LIST_OFFSET: usize = 0x348;
pub const LINUX_REQUEST_QUEUE_DEBUGFS_DIR_OFFSET: usize = 0x358;
pub const LINUX_REQUEST_QUEUE_SCHED_DEBUGFS_DIR_OFFSET: usize = 0x360;
pub const LINUX_REQUEST_QUEUE_RQOS_DEBUGFS_DIR_OFFSET: usize = 0x368;
pub const LINUX_REQUEST_QUEUE_DEBUGFS_MUTEX_OFFSET: usize = 0x370;
pub const LINUX_REQUEST_QUEUE_SIZE: usize = 0x388;
pub const LINUX_MUTEX_SIZE: usize = 0x18;
pub const LINUX_BLK_MQ_TAG_SET_DRIVER_DATA_OFFSET: usize = 0x58;
pub const LINUX_BLK_MQ_TAG_SET_TAGS_OFFSET: usize = 0x60;
pub const LINUX_BLK_MQ_TAG_SET_SHARED_TAGS_OFFSET: usize = 0x68;
pub const LINUX_BLK_MQ_TAG_SET_TAG_LIST_LOCK_OFFSET: usize = 0x70;
pub const LINUX_BLK_MQ_TAG_SET_TAG_LIST_OFFSET: usize = 0x88;
pub const LINUX_BLK_MQ_TAG_SET_SRCU_OFFSET: usize = 0x98;
pub const LINUX_BLK_MQ_TAG_SET_TAGS_SRCU_OFFSET: usize = 0xa0;
pub const LINUX_BLK_MQ_TAG_SET_UPDATE_NR_HWQ_LOCK_OFFSET: usize = 0xc0;
pub const LINUX_BLK_MQ_TAG_SET_SIZE: usize = 0xe0;
pub const LINUX_BLK_MQ_OPS_POLL_OFFSET: usize = 0x40;
pub const LINUX_BLK_MQ_OPS_COMPLETE_OFFSET: usize = 0x48;
pub const LINUX_BLK_MQ_OPS_MAP_QUEUES_OFFSET: usize = 0x80;
pub const LINUX_BLK_MQ_HW_CTX_QUEUE_OFFSET: usize = 0xb8;
pub const LINUX_BLK_MQ_HW_CTX_FQ_OFFSET: usize = 0xc0;
pub const LINUX_BLK_MQ_HW_CTX_DRIVER_DATA_OFFSET: usize = 0xc8;
pub const LINUX_BLK_MQ_HW_CTX_CTX_MAP_OFFSET: usize = 0xd0;
pub const LINUX_BLK_MQ_HW_CTX_DISPATCH_FROM_OFFSET: usize = 0xf0;
pub const LINUX_BLK_MQ_HW_CTX_DISPATCH_BUSY_OFFSET: usize = 0xf8;
pub const LINUX_BLK_MQ_HW_CTX_TYPE_OFFSET: usize = 0xfc;
pub const LINUX_BLK_MQ_HW_CTX_NR_CTX_OFFSET: usize = 0xfe;
pub const LINUX_BLK_MQ_HW_CTX_CTXS_OFFSET: usize = 0x100;
pub const LINUX_BLK_MQ_HW_CTX_TAGS_OFFSET: usize = 0x140;
pub const LINUX_BLK_MQ_HW_CTX_SCHED_TAGS_OFFSET: usize = 0x148;
pub const LINUX_BLK_MQ_HW_CTX_NUMA_NODE_OFFSET: usize = 0x150;
pub const LINUX_BLK_MQ_HW_CTX_QUEUE_NUM_OFFSET: usize = 0x154;
pub const LINUX_BLK_MQ_HW_CTX_NR_ACTIVE_OFFSET: usize = 0x158;
pub const LINUX_BLK_MQ_HW_CTX_SIZE: usize = 0x200;
pub const LINUX_REQUEST_TAG_OFFSET: usize = 0x20;
pub const LINUX_REQUEST_INTERNAL_TAG_OFFSET: usize = 0x24;
pub const LINUX_REQUEST_TIMEOUT_OFFSET: usize = 0x28;
pub const LINUX_REQUEST_DATA_LEN_OFFSET: usize = 0x2c;
pub const LINUX_REQUEST_SECTOR_OFFSET: usize = 0x30;
pub const LINUX_REQUEST_BIO_OFFSET: usize = 0x38;
pub const LINUX_REQUEST_BIOTAIL_OFFSET: usize = 0x40;
pub const LINUX_REQUEST_RQ_NEXT_OFFSET: usize = 0x48;
pub const LINUX_REQUEST_PART_OFFSET: usize = 0x58;
pub const LINUX_REQUEST_ALLOC_TIME_NS_OFFSET: usize = 0x60;
pub const LINUX_REQUEST_START_TIME_NS_OFFSET: usize = 0x68;
pub const LINUX_REQUEST_IO_START_TIME_NS_OFFSET: usize = 0x70;
pub const LINUX_REQUEST_STATS_SECTORS_OFFSET: usize = 0x78;
pub const LINUX_REQUEST_NR_PHYS_SEGMENTS_OFFSET: usize = 0x7a;
pub const LINUX_REQUEST_NR_INTEGRITY_SEGMENTS_OFFSET: usize = 0x7c;
pub const LINUX_REQUEST_PHYS_GAP_BIT_OFFSET: usize = 0x7e;
pub const LINUX_REQUEST_STATE_OFFSET: usize = 0x80;
pub const LINUX_REQUEST_REF_OFFSET: usize = 0x84;
pub const LINUX_REQUEST_DEADLINE_OFFSET: usize = 0x88;
pub const LINUX_REQUEST_HASH_OFFSET: usize = 0x90;
pub const LINUX_REQUEST_SPECIAL_VEC_OFFSET: usize = 0xa0;
pub const LINUX_REQUEST_ELV_OFFSET: usize = 0xb8;
pub const LINUX_REQUEST_FLUSH_OFFSET: usize = 0xd0;
pub const LINUX_REQUEST_FIFO_TIME_OFFSET: usize = 0xe0;
pub const LINUX_REQUEST_END_IO_OFFSET: usize = 0xe8;
pub const LINUX_REQUEST_SIZE: usize = 0xf8;
pub(crate) const LINUX_REQUEST_PDU_OFFSET: usize = LINUX_REQUEST_SIZE;
pub const LINUX_BIO_VEC_BV_PAGE_OFFSET: usize = 0x0;
pub const LINUX_BIO_VEC_BV_LEN_OFFSET: usize = 0x8;
pub const LINUX_BIO_VEC_BV_OFFSET_OFFSET: usize = 0xc;
pub const LINUX_BIO_VEC_SIZE: usize = 0x10;
pub const LINUX_BVEC_ITER_BI_SECTOR_OFFSET: usize = 0x0;
pub const LINUX_BVEC_ITER_BI_SIZE_OFFSET: usize = 0x8;
pub const LINUX_BVEC_ITER_BI_IDX_OFFSET: usize = 0xc;
pub const LINUX_BVEC_ITER_BI_BVEC_DONE_OFFSET: usize = 0x10;
pub const LINUX_BVEC_ITER_SIZE: usize = 0x14;
pub const LINUX_BIO_BI_NEXT_OFFSET: usize = 0x0;
pub const LINUX_BIO_BI_BDEV_OFFSET: usize = 0x8;
pub const LINUX_BIO_BI_OPF_OFFSET: usize = 0x10;
pub const LINUX_BIO_BI_FLAGS_OFFSET: usize = 0x14;
pub const LINUX_BIO_BI_IOPRIO_OFFSET: usize = 0x16;
pub const LINUX_BIO_BI_WRITE_HINT_OFFSET: usize = 0x18;
pub const LINUX_BIO_BI_WRITE_STREAM_OFFSET: usize = 0x19;
pub const LINUX_BIO_BI_STATUS_OFFSET: usize = 0x1a;
pub const LINUX_BIO_BI_BVEC_GAP_BIT_OFFSET: usize = 0x1b;
pub const LINUX_BIO_BI_REMAINING_OFFSET: usize = 0x1c;
pub const LINUX_BIO_BI_IO_VEC_OFFSET: usize = 0x20;
pub const LINUX_BIO_BI_ITER_OFFSET: usize = 0x28;
pub const LINUX_BIO_BI_END_IO_OFFSET: usize = 0x40;
pub const LINUX_BIO_BI_PRIVATE_OFFSET: usize = 0x48;
pub const LINUX_BIO_BI_BLKG_OFFSET: usize = 0x50;
pub const LINUX_BIO_ISSUE_TIME_NS_OFFSET: usize = 0x58;
pub const LINUX_BIO_BI_IOCOST_COST_OFFSET: usize = 0x60;
pub const LINUX_BIO_BI_VCNT_OFFSET: usize = 0x68;
pub const LINUX_BIO_BI_MAX_VECS_OFFSET: usize = 0x6a;
pub const LINUX_BIO_BI_CNT_OFFSET: usize = 0x6c;
pub const LINUX_BIO_BI_POOL_OFFSET: usize = 0x70;
pub const LINUX_BIO_SIZE: usize = 0x78;
pub const LINUX_BLOCK_DEVICE_SIZE: usize = 0x3b8;
pub const LINUX_BLOCK_DEVICE_BD_STATS_OFFSET: usize = 0x20;
pub const LINUX_BLOCK_DEVICE_BD_STAMP_OFFSET: usize = 0x28;
pub const LINUX_BLOCK_DEVICE_BD_FLAGS_OFFSET: usize = 0x30;
pub const LINUX_BLOCK_DEVICE_BD_DEV_OFFSET: usize = 0x34;
pub const LINUX_BLOCK_DEVICE_BD_MAPPING_OFFSET: usize = 0x38;
pub const LINUX_BLOCK_DEVICE_BD_OPENERS_OFFSET: usize = 0x40;
pub const LINUX_BLOCK_DEVICE_BD_SIZE_LOCK_OFFSET: usize = 0x44;
pub const LINUX_BLOCK_DEVICE_BD_CLAIMING_OFFSET: usize = 0x48;
pub const LINUX_BLOCK_DEVICE_BD_HOLDER_OFFSET: usize = 0x50;
pub const LINUX_BLOCK_DEVICE_BD_HOLDER_OPS_OFFSET: usize = 0x58;
pub const LINUX_BLOCK_DEVICE_BD_HOLDER_LOCK_OFFSET: usize = 0x60;
pub const LINUX_BLOCK_DEVICE_BD_HOLDERS_OFFSET: usize = 0x78;
pub const LINUX_BLOCK_DEVICE_BD_HOLDER_DIR_OFFSET: usize = 0x80;
pub const LINUX_BLOCK_DEVICE_BD_FSFREEZE_COUNT_OFFSET: usize = 0x88;
pub const LINUX_BLOCK_DEVICE_BD_FSFREEZE_MUTEX_OFFSET: usize = 0x90;
pub const LINUX_BLOCK_DEVICE_BD_META_INFO_OFFSET: usize = 0xa8;
pub const LINUX_BLOCK_DEVICE_BD_WRITERS_OFFSET: usize = 0xb0;
pub const LINUX_BLOCK_DEVICE_BD_DEVICE_OFFSET: usize = 0xc0;
pub const LINUX_ATOMIC_T_SIZE: usize = 0x4;
pub const LINUX_DEV_T_SIZE: usize = 0x4;
pub const LINUX_SPINLOCK_T_SIZE: usize = 0x4;
pub const LINUX_STRUCT_DEVICE_SIZE: usize = 0x2f8;
pub const LINUX_GENDISK_BIO_SPLIT_OFFSET: usize = 0x60;
pub const LINUX_GENDISK_FLAGS_OFFSET: usize = 0x158;
pub const LINUX_GENDISK_STATE_OFFSET: usize = 0x160;
pub const LINUX_GENDISK_SIZE: usize = 0x240;
pub const LINUX_BIO_SET_SIZE: usize = LINUX_GENDISK_FLAGS_OFFSET - LINUX_GENDISK_BIO_SPLIT_OFFSET;
pub const RQF_SPECIAL_PAYLOAD_BYTE_OFFSET: usize = 0x1d;
pub const RQF_SPECIAL_PAYLOAD_BYTE_MASK: u8 = 0x10;
pub const MQ_RQ_IDLE: u32 = 0;
pub const MQ_RQ_IN_FLIGHT: u32 = 1;
pub const MQ_RQ_COMPLETE: u32 = 2;
pub const BLK_STS_OK: u8 = 0;
pub const BLK_STS_RESOURCE: u8 = 9;
pub const BLK_STS_IOERR: u8 = 10;
pub const BLK_STS_TIMEOUT: u8 = 11;
pub const BLK_STS_AGAIN: u8 = 12;
pub const BLK_STS_DEV_RESOURCE: u8 = 13;
#[cfg(test)]
const RQF_DONTPREP: u32 = 1 << 3;
const RQF_SPECIAL_PAYLOAD: u32 = 1 << 12;

pub type LinuxBlkStatus = u8;

/// Prefix of `struct blk_mq_queue_data`.
///
/// Source: `vendor/linux/include/linux/blk-mq.h:565`.
#[repr(C)]
pub struct LinuxBlkMqQueueData {
    pub rq: *mut LinuxRequest,
    pub last: bool,
}

/// Callback prefix of `struct blk_mq_ops`.
///
/// Source: `vendor/linux/include/linux/blk-mq.h:577`.
#[repr(C)]
pub struct LinuxBlkMqOps {
    pub queue_rq: Option<
        unsafe extern "C" fn(*mut LinuxBlkMqHwCtx, *const LinuxBlkMqQueueData) -> LinuxBlkStatus,
    >,
    pub commit_rqs: Option<unsafe extern "C" fn(*mut LinuxBlkMqHwCtx)>,
    pub queue_rqs: Option<unsafe extern "C" fn(*mut c_void)>,
    pub get_budget: Option<unsafe extern "C" fn(*mut LinuxRequestQueue) -> i32>,
    pub put_budget: Option<unsafe extern "C" fn(*mut LinuxRequestQueue, i32)>,
    pub set_rq_budget_token: Option<unsafe extern "C" fn(*mut LinuxRequest, i32)>,
    pub get_rq_budget_token: Option<unsafe extern "C" fn(*mut LinuxRequest) -> i32>,
    pub timeout: Option<unsafe extern "C" fn(*mut LinuxRequest) -> u32>,
    pub poll: Option<unsafe extern "C" fn(*mut LinuxBlkMqHwCtx, *mut c_void) -> i32>,
    pub complete: Option<unsafe extern "C" fn(*mut LinuxRequest)>,
    pub init_hctx: Option<unsafe extern "C" fn(*mut LinuxBlkMqHwCtx, *mut c_void, u32) -> i32>,
    pub exit_hctx: Option<unsafe extern "C" fn(*mut LinuxBlkMqHwCtx, u32)>,
    pub init_request:
        Option<unsafe extern "C" fn(*mut LinuxBlkMqTagSet, *mut LinuxRequest, u32, i32) -> i32>,
    pub exit_request: Option<unsafe extern "C" fn(*mut LinuxBlkMqTagSet, *mut LinuxRequest, u32)>,
    pub cleanup_rq: Option<unsafe extern "C" fn(*mut LinuxRequest)>,
    pub busy: Option<unsafe extern "C" fn(*mut LinuxRequestQueue) -> bool>,
    pub map_queues: Option<unsafe extern "C" fn(*mut LinuxBlkMqTagSet)>,
    pub show_rq: Option<unsafe extern "C" fn(*mut c_void, *mut LinuxRequest)>,
}

/// Prefix of `struct blk_mq_hw_ctx`.
///
/// Source: `vendor/linux/include/linux/blk-mq.h` and the Linux-built
/// `virtio_blk.ko` request path.
#[repr(C, align(64))]
pub struct LinuxBlkMqHwCtx {
    pub _pad_to_queue: [u8; LINUX_BLK_MQ_HW_CTX_QUEUE_OFFSET],
    pub queue: *mut LinuxRequestQueue,
    pub _pad_to_driver_data: [u8; LINUX_BLK_MQ_HW_CTX_DRIVER_DATA_OFFSET
        - (LINUX_BLK_MQ_HW_CTX_QUEUE_OFFSET + core::mem::size_of::<*mut LinuxRequestQueue>())],
    pub driver_data: *mut c_void,
    pub _pad_to_queue_num: [u8; LINUX_BLK_MQ_HW_CTX_QUEUE_NUM_OFFSET
        - (LINUX_BLK_MQ_HW_CTX_DRIVER_DATA_OFFSET + core::mem::size_of::<*mut c_void>())],
    pub queue_num: u32,
    pub _pad_after_queue_num:
        [u8; LINUX_BLK_MQ_HW_CTX_SIZE - (LINUX_BLK_MQ_HW_CTX_QUEUE_NUM_OFFSET + 4)],
}

/// Prefix of `struct request` through `end_io_data`, with PDU at `+0xf8`.
///
/// Source: `vendor/linux/include/linux/blk-mq.h:105`; offsets are pinned to
/// the Linux-built `virtio_blk.ko` path that embeds `struct virtblk_req` in the
/// request PDU.
#[repr(C)]
pub struct LinuxRequest {
    pub q: *mut LinuxRequestQueue,
    pub mq_ctx: *mut c_void,
    pub mq_hctx: *mut LinuxBlkMqHwCtx,
    pub cmd_flags: u32,
    pub rq_flags: u32,
    pub tag: i32,
    pub internal_tag: i32,
    pub timeout: u32,
    pub data_len: u32,
    pub sector: u64,
    pub bio: *mut c_void,
    pub biotail: *mut c_void,
    pub rq_next: *mut LinuxRequest,
    pub _pad_queuelist_prev: *mut LinuxRequest,
    pub part: *mut c_void,
    pub _pad_to_nr_phys_segments: [u8; LINUX_REQUEST_NR_PHYS_SEGMENTS_OFFSET - 0x60],
    pub nr_phys_segments: u16,
    pub nr_integrity_segments: u16,
    pub _pad_to_state:
        [u8; LINUX_REQUEST_STATE_OFFSET - (LINUX_REQUEST_NR_INTEGRITY_SEGMENTS_OFFSET + 2)],
    pub state: u32,
    pub _pad_to_special_vec:
        [u8; LINUX_REQUEST_SPECIAL_VEC_OFFSET - (LINUX_REQUEST_STATE_OFFSET + 4)],
    pub special_vec_bv_page: *mut c_void,
    pub special_vec_bv_len: u32,
    pub special_vec_bv_offset: u32,
    pub _pad_after_special_vec: [u8; LINUX_REQUEST_ELV_OFFSET
        - (LINUX_REQUEST_SPECIAL_VEC_OFFSET + core::mem::size_of::<LinuxBioVec>())],
    pub elv: [u8; LINUX_REQUEST_FLUSH_OFFSET - LINUX_REQUEST_ELV_OFFSET],
    pub flush: [u8; LINUX_REQUEST_FIFO_TIME_OFFSET - LINUX_REQUEST_FLUSH_OFFSET],
    pub fifo_time: u64,
    pub end_io: *mut c_void,
    pub end_io_data: *mut c_void,
}

/// `struct bio_vec` prefix used by vendor block helpers that inspect `rq->bio`.
///
/// Source: `vendor/linux/include/linux/bvec.h:28`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxBioVec {
    pub bv_page: *mut c_void,
    pub bv_len: u32,
    pub bv_offset: u32,
}

/// `struct bvec_iter`.
///
/// Source: `vendor/linux/include/linux/bvec.h:92`, which is packed and
/// aligned to four bytes.
#[repr(C, packed(4))]
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxBvecIter {
    pub bi_sector: u64,
    pub bi_size: u32,
    pub bi_idx: u32,
    pub bi_bvec_done: u32,
}

/// `struct bio` prefix through `bi_pool`.
///
/// Source: `vendor/linux/include/linux/blk_types.h:209`. Lupos keeps the
/// native `BioRef` in `LINUX_REQUESTS` for actual segment mapping; this mirror
/// gives Linux-built SCSI/libata helpers a dereferenceable `rq->bio` for
/// metadata such as `bi_ioprio`, `bi_write_hint`, and `bi_iter`.
#[repr(C)]
pub struct LinuxBio {
    pub bi_next: *mut LinuxBio,
    pub bi_bdev: *mut c_void,
    pub bi_opf: u32,
    pub bi_flags: u16,
    pub bi_ioprio: u16,
    pub bi_write_hint: u8,
    pub bi_write_stream: u8,
    pub bi_status: u8,
    pub bi_bvec_gap_bit: u8,
    pub bi_remaining: i32,
    pub bi_io_vec: *mut LinuxBioVec,
    pub bi_iter: LinuxBvecIter,
    pub bi_cookie: u32,
    pub bi_end_io: *mut c_void,
    pub bi_private: *mut c_void,
    pub bi_blkg: *mut c_void,
    pub issue_time_ns: u64,
    pub bi_iocost_cost: u64,
    pub bi_vcnt: u16,
    pub bi_max_vecs: u16,
    pub bi_cnt: i32,
    pub bi_pool: *mut c_void,
}

// These are inert ABI records owned by `LINUX_REQUESTS`; all mutation is
// serialized by that registry, and Linux-built code receives only stable raw
// pointers into the owned records.
unsafe impl Send for LinuxBioVec {}
unsafe impl Send for LinuxBio {}

/// `struct blk_mq_queue_map` — `vendor/linux/include/linux/blk-mq.h:475`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxBlkMqQueueMap {
    pub mq_map: *mut u32,
    pub nr_queues: u32,
    pub queue_offset: u32,
}

/// `struct blk_mq_tag_set` with vendor-size tail padding.
///
/// Source: `vendor/linux/include/linux/blk-mq.h:534`.
#[repr(C)]
pub struct LinuxBlkMqTagSet {
    pub ops: *const c_void,
    pub map: [LinuxBlkMqQueueMap; HCTX_MAX_TYPES],
    pub nr_maps: u32,
    pub nr_hw_queues: u32,
    pub queue_depth: u32,
    pub reserved_tags: u32,
    pub cmd_size: u32,
    pub numa_node: i32,
    pub timeout: u32,
    pub flags: u32,
    pub driver_data: *mut c_void,
    pub tags: *mut *mut c_void,
    pub shared_tags: *mut c_void,
    pub _pad_after_shared_tags: [u8; LINUX_BLK_MQ_TAG_SET_SIZE
        - (LINUX_BLK_MQ_TAG_SET_SHARED_TAGS_OFFSET + core::mem::size_of::<*mut c_void>())],
}

/// Prefix of `struct queue_limits` through the fields virtio-blk updates.
///
/// Source: `vendor/linux/include/linux/blkdev.h:376`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxQueueLimits {
    pub features: u32,
    pub flags: u32,
    pub seg_boundary_mask: usize,
    pub virt_boundary_mask: usize,
    pub max_hw_sectors: u32,
    pub max_dev_sectors: u32,
    pub chunk_sectors: u32,
    pub max_sectors: u32,
    pub max_user_sectors: u32,
    pub max_segment_size: u32,
    pub max_fast_segment_size: u32,
    pub physical_block_size: u32,
    pub logical_block_size: u32,
    pub alignment_offset: u32,
    pub io_min: u32,
    pub io_opt: u32,
    pub max_discard_sectors: u32,
    pub max_hw_discard_sectors: u32,
    pub max_user_discard_sectors: u32,
    pub max_secure_erase_sectors: u32,
    pub max_write_zeroes_sectors: u32,
    pub max_wzeroes_unmap_sectors: u32,
    pub max_hw_wzeroes_unmap_sectors: u32,
    pub max_user_wzeroes_unmap_sectors: u32,
    pub max_hw_zone_append_sectors: u32,
    pub max_zone_append_sectors: u32,
    pub discard_granularity: u32,
    pub discard_alignment: u32,
    pub zone_write_granularity: u32,
    pub atomic_write_hw_max: u32,
    pub atomic_write_max_sectors: u32,
    pub atomic_write_hw_boundary: u32,
    pub atomic_write_boundary_sectors: u32,
    pub atomic_write_hw_unit_min: u32,
    pub atomic_write_unit_min: u32,
    pub atomic_write_hw_unit_max: u32,
    pub atomic_write_unit_max: u32,
    pub max_segments: u16,
    pub max_integrity_segments: u16,
    pub max_discard_segments: u16,
    pub max_write_streams: u16,
    pub write_stream_granularity: u32,
    pub max_open_zones: u32,
    pub max_active_zones: u32,
    pub dma_alignment: u32,
    pub dma_pad_mask: u32,
    pub integrity: [u8; 8],
}

/// Opaque Linux `struct mutex` storage for fields accessed through exported
/// mutex helpers.  The current non-RT vendor config compiles this to 16 bytes;
/// Lupos' helper implementation uses the first owner word.
#[repr(C, align(8))]
pub struct LinuxAbiMutex {
    bytes: [u8; LINUX_MUTEX_SIZE],
}

impl LinuxAbiMutex {
    const fn new() -> Self {
        Self {
            bytes: [0; LINUX_MUTEX_SIZE],
        }
    }
}

unsafe fn linux_request_queue_limits_lock(
    q: *mut LinuxRequestQueue,
) -> *mut crate::kernel::locking::mutex::LinuxRawMutex {
    unsafe {
        core::ptr::addr_of_mut!((*q).limits_lock)
            .cast::<crate::kernel::locking::mutex::LinuxRawMutex>()
    }
}

/// Prefix and ABI-relevant tail of `struct request_queue`.
///
/// Source: `vendor/linux/include/linux/blkdev.h:484`.
#[repr(C)]
pub struct LinuxRequestQueue {
    pub queuedata: *mut c_void,
    pub elevator: *mut c_void,
    pub mq_ops: *const c_void,
    pub queue_ctx: *mut c_void,
    pub queue_flags: usize,
    pub rq_timeout: u32,
    pub queue_depth: u32,
    pub refs: i32,
    pub nr_hw_queues: u32,
    pub queue_hw_ctx: *mut c_void,
    pub _pad_after_queue_hw_ctx: [u8; LINUX_REQUEST_QUEUE_DISK_OFFSET - 0x40],
    pub disk: *mut LinuxGendisk,
    pub mq_kobj: *mut c_void,
    pub limits: LinuxQueueLimits,
    pub _pad_after_limits: [u8; LINUX_REQUEST_QUEUE_PM_ONLY_OFFSET
        - (0x70 + core::mem::size_of::<LinuxQueueLimits>())],
    pub pm_only: i32,
    pub _pad_after_pm_only: [u8; LINUX_REQUEST_QUEUE_STATS_OFFSET
        - (LINUX_REQUEST_QUEUE_PM_ONLY_OFFSET + core::mem::size_of::<i32>())],
    pub stats: *mut c_void,
    pub rq_qos: *mut c_void,
    pub rq_qos_mutex: LinuxAbiMutex,
    pub id: i32,
    pub nr_requests: u32,
    pub async_depth: u32,
    pub _pad_after_async_depth: [u8; LINUX_REQUEST_QUEUE_LIMITS_LOCK_OFFSET
        - (LINUX_REQUEST_QUEUE_ASYNC_DEPTH_OFFSET + core::mem::size_of::<u32>())],
    pub limits_lock: LinuxAbiMutex,
    pub _pad_after_limits_lock: [u8; LINUX_REQUEST_QUEUE_TAG_SET_OFFSET
        - (LINUX_REQUEST_QUEUE_LIMITS_LOCK_OFFSET + LINUX_MUTEX_SIZE)],
    pub tag_set: *mut LinuxBlkMqTagSet,
    pub _pad_after_tag_set: [u8; LINUX_REQUEST_QUEUE_SIZE
        - (LINUX_REQUEST_QUEUE_TAG_SET_OFFSET + core::mem::size_of::<*mut LinuxBlkMqTagSet>())],
    pub hctx_table_storage: *mut LinuxBlkMqHwCtx,
}

/// `struct block_device` prefix through `bd_queue`, padded to vendor size.
///
/// Source: `vendor/linux/include/linux/blk_types.h:41`.  The SCSI disk driver
/// uses Linux's inline `get_capacity()`, which reads `disk->part0->bd_nr_sectors`;
/// other inline helpers read the opaque tail fields pinned by the constants
/// above, such as `__bd_flags`, `bd_openers`, and `bd_device`.
#[repr(C)]
pub struct LinuxBlockDevicePrefix {
    pub bd_start_sect: u64,
    pub bd_nr_sectors: u64,
    pub bd_disk: *mut LinuxGendisk,
    pub bd_queue: *mut LinuxRequestQueue,
    pub _pad_after_bd_queue: [u8; LINUX_BLOCK_DEVICE_SIZE - 0x20],
}

/// `struct gendisk` prefix through `state`, padded to vendor size.
///
/// Source: `vendor/linux/include/linux/blkdev.h:146`.
#[repr(C)]
pub struct LinuxGendisk {
    pub major: i32,
    pub first_minor: i32,
    pub minors: i32,
    pub disk_name: [u8; DISK_NAME_LEN],
    pub events: u16,
    pub event_flags: u16,
    pub part_tbl: [usize; 2],
    pub part0: *mut LinuxBlockDevicePrefix,
    pub fops: *const c_void,
    pub queue: *mut LinuxRequestQueue,
    pub private_data: *mut c_void,
    pub bio_split: [u8; LINUX_BIO_SET_SIZE],
    pub flags: i32,
    pub _pad_after_flags: [u8; LINUX_GENDISK_STATE_OFFSET
        - (LINUX_GENDISK_FLAGS_OFFSET + core::mem::size_of::<i32>())],
    pub state: usize,
    pub _pad_after_state:
        [u8; LINUX_GENDISK_SIZE - (LINUX_GENDISK_STATE_OFFSET + core::mem::size_of::<usize>())],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxBlockMajor {
    pub major: u32,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxDiskRegistration {
    pub disk: usize,
    pub queue: usize,
    pub parent: usize,
    pub name: [u8; DISK_NAME_LEN],
}

struct LinuxRequestAllocation {
    ptr: usize,
    len: usize,
    bio: Option<BioRef>,
    linux_bio: Option<Box<LinuxBio>>,
    linux_bvecs: Vec<LinuxBioVec>,
    _words: Vec<usize>,
}

struct LinuxGendiskBlockDevice {
    disk: usize,
}

static LINUX_BLOCK_MAJORS: Mutex<Vec<LinuxBlockMajor>> = Mutex::new(Vec::new());
static LINUX_DISKS: Mutex<Vec<LinuxDiskRegistration>> = Mutex::new(Vec::new());
static LINUX_REQUESTS: Mutex<Vec<LinuxRequestAllocation>> = Mutex::new(Vec::new());

static NEXT_BLOCK_MAJOR: AtomicU32 = AtomicU32::new(240);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__register_blkdev", __register_blkdev as usize, true);
    export_symbol_once("unregister_blkdev", unregister_blkdev as usize, true);
    export_symbol_once("blk_mq_alloc_tag_set", blk_mq_alloc_tag_set as usize, true);
    export_symbol_once("blk_mq_free_tag_set", blk_mq_free_tag_set as usize, true);
    export_symbol_once("blk_mq_alloc_request", blk_mq_alloc_request as usize, true);
    export_symbol_once("blk_mq_free_request", blk_mq_free_request as usize, true);
    export_symbol_once("blk_mq_start_request", blk_mq_start_request as usize, true);
    export_symbol_once("blk_mq_end_request", blk_mq_end_request as usize, true);
    export_symbol_once(
        "blk_mq_complete_request",
        blk_mq_complete_request as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_complete_request_remote",
        blk_mq_complete_request_remote as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_end_request_batch",
        blk_mq_end_request_batch as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_requeue_request",
        blk_mq_requeue_request as usize,
        true,
    );
    export_symbol_once("blk_mq_stop_hw_queue", blk_mq_stop_hw_queue as usize, true);
    export_symbol_once(
        "blk_mq_start_stopped_hw_queues",
        blk_mq_start_stopped_hw_queues as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_quiesce_queue_nowait",
        blk_mq_quiesce_queue_nowait as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_unquiesce_queue",
        blk_mq_unquiesce_queue as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_freeze_queue_nomemsave",
        blk_mq_freeze_queue_nomemsave as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_unfreeze_queue_nomemrestore",
        blk_mq_unfreeze_queue_nomemrestore as usize,
        true,
    );
    export_symbol_once(
        "blk_mq_num_possible_queues",
        blk_mq_num_possible_queues as usize,
        true,
    );
    export_symbol_once("blk_mq_map_queues", blk_mq_map_queues as usize, true);
    export_symbol_once("blk_mq_map_hw_queues", blk_mq_map_hw_queues as usize, true);
    export_symbol_once("__blk_rq_map_sg", __blk_rq_map_sg as usize, true);
    export_symbol_once("blk_rq_map_kern", blk_rq_map_kern as usize, true);
    export_symbol_once("blk_execute_rq", blk_execute_rq as usize, true);
    export_symbol_once("blk_status_to_errno", blk_status_to_errno as usize, true);
    export_symbol_once(
        "queue_limits_commit_update_frozen",
        queue_limits_commit_update_frozen as usize,
        true,
    );
    export_symbol_once("__blk_mq_alloc_disk", __blk_mq_alloc_disk as usize, true);
    export_symbol_once("device_add_disk", device_add_disk as usize, true);
    export_symbol_once("put_disk", put_disk as usize, true);
    export_symbol_once("del_gendisk", del_gendisk as usize, true);
    export_symbol_once("set_disk_ro", set_disk_ro as usize, true);
    export_symbol_once("set_capacity", set_capacity as usize, true);
    export_symbol_once(
        "set_capacity_and_notify",
        set_capacity_and_notify as usize,
        true,
    );
}

unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String, i32> {
    if ptr.is_null() {
        return Err(EINVAL);
    }
    let mut len = 0usize;
    while len < 256 {
        if unsafe { *ptr.add(len) } == 0 {
            let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
            return core::str::from_utf8(bytes)
                .map(|s| s.to_string())
                .map_err(|_| EINVAL);
        }
        len += 1;
    }
    Err(EINVAL)
}

fn disk_name_bytes(disk: &LinuxGendisk) -> [u8; DISK_NAME_LEN] {
    disk.disk_name
}

fn disk_name_bytes_to_string(bytes: &[u8; DISK_NAME_LEN]) -> Result<String, i32> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    if end == 0 {
        return Err(EINVAL);
    }
    core::str::from_utf8(&bytes[..end])
        .map(|name| name.to_string())
        .map_err(|_| EINVAL)
}

pub fn registered_linux_block_majors() -> Vec<LinuxBlockMajor> {
    LINUX_BLOCK_MAJORS.lock().clone()
}

pub fn registered_linux_disk_count() -> usize {
    LINUX_DISKS.lock().len()
}

pub fn registered_linux_disk_names() -> Vec<String> {
    LINUX_DISKS
        .lock()
        .iter()
        .filter_map(|registered| disk_name_bytes_to_string(&registered.name).ok())
        .collect()
}

pub fn linux_disk_registered(disk: *const LinuxGendisk) -> bool {
    LINUX_DISKS
        .lock()
        .iter()
        .any(|registered| registered.disk == disk as usize)
}

/// `__register_blkdev` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __register_blkdev(
    major: u32,
    name: *const c_char,
    _probe: *mut c_void,
) -> i32 {
    let name = match unsafe { cstr_to_string(name) } {
        Ok(name) => name,
        Err(err) => return -err,
    };

    let assigned = if major == 0 {
        NEXT_BLOCK_MAJOR.fetch_add(1, Ordering::AcqRel)
    } else {
        major
    };
    let mut majors = LINUX_BLOCK_MAJORS.lock();
    if majors.iter().any(|registered| registered.major == assigned) {
        return -EBUSY;
    }
    majors.push(LinuxBlockMajor {
        major: assigned,
        name,
    });
    if major == 0 { assigned as i32 } else { 0 }
}

/// `unregister_blkdev` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unregister_blkdev(major: u32, name: *const c_char) {
    let name = unsafe { cstr_to_string(name).ok() };
    LINUX_BLOCK_MAJORS.lock().retain(|registered| {
        registered.major != major || name.as_ref().is_some_and(|name| registered.name != *name)
    });
}

/// `blk_mq_alloc_tag_set` — `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_alloc_tag_set(set: *mut LinuxBlkMqTagSet) -> i32 {
    if set.is_null() {
        crate::log_warn!("block", "blk_mq_alloc_tag_set: null tag set");
        return -EINVAL;
    }
    unsafe {
        if (*set).ops.is_null() || (*set).nr_hw_queues == 0 || (*set).queue_depth == 0 {
            crate::log_warn!(
                "block",
                "blk_mq_alloc_tag_set: invalid ops={:p} nr_hw_queues={} queue_depth={} nr_maps={}",
                (*set).ops,
                (*set).nr_hw_queues,
                (*set).queue_depth,
                (*set).nr_maps
            );
            return -EINVAL;
        }
        if (*set).nr_maps == 0 {
            (*set).nr_maps = 1;
        }
        let maps = core::cmp::min((*set).nr_maps as usize, HCTX_MAX_TYPES);
        let mut offset = 0u32;
        let mut idx = 0usize;
        while idx < maps {
            if (*set).map[idx].nr_queues == 0 {
                (*set).map[idx].nr_queues = (*set).nr_hw_queues;
                (*set).map[idx].queue_offset = offset;
            }
            offset = offset.saturating_add((*set).map[idx].nr_queues);
            idx += 1;
        }
    }
    0
}

/// `blk_mq_free_tag_set` — `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_free_tag_set(_set: *mut LinuxBlkMqTagSet) {}

fn linux_err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as usize as *mut c_void
}

fn linux_ptr_errno(ptr: *mut c_void) -> Option<i32> {
    let value = ptr as isize;
    if (-4095..0).contains(&value) {
        Some((-value) as i32)
    } else {
        None
    }
}

fn linux_request_hctx(q: *mut LinuxRequestQueue) -> *mut LinuxBlkMqHwCtx {
    if q.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        let table = (*q).queue_hw_ctx as *mut *mut LinuxBlkMqHwCtx;
        if table.is_null() {
            core::ptr::null_mut()
        } else {
            *table
        }
    }
}

fn linux_request_bio(rq: *mut LinuxRequest) -> Option<BioRef> {
    LINUX_REQUESTS
        .lock()
        .iter()
        .find(|allocation| allocation.ptr == rq as usize)
        .and_then(|allocation| allocation.bio.clone())
}

fn linux_request_bdev(rq: *mut LinuxRequest) -> *mut c_void {
    if rq.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        if !(*rq).part.is_null() {
            return (*rq).part;
        }
        let q = (*rq).q;
        if q.is_null() {
            return core::ptr::null_mut();
        }
        let disk = (*q).disk;
        if disk.is_null() {
            core::ptr::null_mut()
        } else {
            (*disk).part0.cast::<c_void>()
        }
    }
}

fn linux_bio_vecs_from_bio(vecs: &[crate::block::bio::BioVec]) -> Result<Vec<LinuxBioVec>, i32> {
    let mut linux_bvecs = Vec::new();
    linux_bvecs.try_reserve(vecs.len()).map_err(|_| ENOMEM)?;
    for vec in vecs {
        linux_bvecs.push(LinuxBioVec {
            bv_page: core::ptr::null_mut(),
            bv_len: u32::try_from(vec.len).map_err(|_| EINVAL)?,
            bv_offset: u32::try_from(vec.off).map_err(|_| EINVAL)?,
        });
    }
    Ok(linux_bvecs)
}

fn linux_install_request_bio_mirror(
    allocation: &mut LinuxRequestAllocation,
    rq: *mut LinuxRequest,
    bdev: *mut c_void,
    opf: u32,
    sector: u64,
    total_len: u32,
    nr_phys_segments: u16,
    linux_bvecs: Vec<LinuxBioVec>,
) {
    allocation.linux_bvecs = linux_bvecs;
    let bi_io_vec = if allocation.linux_bvecs.is_empty() {
        core::ptr::null_mut()
    } else {
        allocation.linux_bvecs.as_mut_ptr()
    };
    allocation.linux_bio = Some(Box::new(LinuxBio {
        bi_next: core::ptr::null_mut(),
        bi_bdev: bdev,
        bi_opf: opf,
        bi_flags: 0,
        bi_ioprio: 0,
        bi_write_hint: 0,
        bi_write_stream: 0,
        bi_status: BLK_STS_OK,
        bi_bvec_gap_bit: 0,
        bi_remaining: 1,
        bi_io_vec,
        bi_iter: LinuxBvecIter {
            bi_sector: sector,
            bi_size: total_len,
            bi_idx: 0,
            bi_bvec_done: 0,
        },
        bi_cookie: 0,
        bi_end_io: core::ptr::null_mut(),
        bi_private: core::ptr::null_mut(),
        bi_blkg: core::ptr::null_mut(),
        issue_time_ns: 0,
        bi_iocost_cost: 0,
        bi_vcnt: nr_phys_segments,
        bi_max_vecs: nr_phys_segments,
        bi_cnt: 1,
        bi_pool: core::ptr::null_mut(),
    }));
    let linux_bio = allocation
        .linux_bio
        .as_deref_mut()
        .map(|bio| (bio as *mut LinuxBio).cast::<c_void>())
        .unwrap_or(core::ptr::null_mut());
    unsafe {
        (*rq).bio = linux_bio;
        (*rq).biotail = linux_bio;
    }
}

fn linux_request_mark_completed(rq: *mut LinuxRequest, status: LinuxBlkStatus) {
    if rq.is_null() {
        return;
    }
    unsafe {
        core::ptr::write_volatile(&mut (*rq).internal_tag, status as i32);
        core::ptr::write_volatile(&mut (*rq).state, MQ_RQ_COMPLETE);
    }
    // The synchronous submit path polls MQ_RQ_COMPLETE directly; keep the
    // completion transition lock-free because this can run from the AHCI IRQ
    // path or from the software reaper.
}

fn linux_request_reset_completion(rq: *mut LinuxRequest) {
    if rq.is_null() {
        return;
    }
    unsafe {
        core::ptr::write_volatile(&mut (*rq).internal_tag, BLK_STS_OK as i32);
        core::ptr::write_volatile(&mut (*rq).state, MQ_RQ_IDLE);
    }
}

fn linux_request_completed_status(rq: *mut LinuxRequest) -> Option<LinuxBlkStatus> {
    if rq.is_null() {
        return None;
    }
    let state = unsafe { core::ptr::read_volatile(&(*rq).state) };
    (state == MQ_RQ_COMPLETE)
        .then(|| unsafe { core::ptr::read_volatile(&(*rq).internal_tag) as LinuxBlkStatus })
}

fn linux_blk_status_result(status: LinuxBlkStatus) -> Result<(), i32> {
    let errno = unsafe { blk_status_to_errno(status) };
    if errno == 0 {
        Ok(())
    } else {
        Err(errno.unsigned_abs() as i32)
    }
}

fn linux_blk_status_retryable(status: LinuxBlkStatus) -> bool {
    matches!(
        status,
        BLK_STS_RESOURCE | BLK_STS_AGAIN | BLK_STS_DEV_RESOURCE
    )
}

fn linux_poll_request_queue(rq: *mut LinuxRequest) -> i32 {
    if rq.is_null() {
        return 0;
    }
    let q = unsafe { (*rq).q };
    if q.is_null() {
        return 0;
    }
    let ops = unsafe { (*q).mq_ops.cast::<LinuxBlkMqOps>() };
    if ops.is_null() {
        return 0;
    }
    let Some(poll) = (unsafe { (*ops).poll }) else {
        return 0;
    };
    let hctx = unsafe { (*rq).mq_hctx };
    if hctx.is_null() {
        return 0;
    }
    // Linux reference: `vendor/linux/block/blk-mq.c:5266` calls the
    // driver-provided `q->mq_ops->poll(hctx, iob)`.  Passing a null
    // io_comp_batch matches the non-batched completion fallback in
    // `vendor/linux/include/linux/blk-mq.h:895`.
    unsafe { poll(hctx, core::ptr::null_mut()) }
}

fn linux_request_targets_virtio_disk(rq: *mut LinuxRequest) -> bool {
    if rq.is_null() {
        return false;
    }
    let q = unsafe { (*rq).q };
    if q.is_null() {
        return false;
    }
    let disk = unsafe { (*q).disk };
    if disk.is_null() {
        return false;
    }
    let name = unsafe { &(*disk).disk_name };
    name.first() == Some(&b'v') && name.get(1) == Some(&b'd')
}

fn linux_request_wait_cpu_relax(iteration: usize) {
    linux_request_poll_cpu_relax(iteration);
    #[cfg(all(not(test), any(target_arch = "x86_64", target_arch = "x86")))]
    {
        // Cooperatively yield to the scheduler while waiting for I/O.
        //
        // The Linux SCSI/libata stack behind AHCI finishes a command only after
        // a *peer* task (its completion kthread / EH) runs, so a purely non-
        // yielding spin deadlocks. Yielding lets those peers run (and the idle
        // path pumps poll-based driver completions). Only yield in a non-atomic
        // context with a live current task — never while holding a spinlock or
        // before the scheduler is up.
        if crate::kernel::locking::preempt::preempt_count() == 0
            && !unsafe { crate::kernel::sched::get_current() }.is_null()
        {
            unsafe {
                crate::kernel::sched::schedule_with_irqs_enabled();
            }
        }
    }
}

fn linux_request_poll_cpu_relax(iteration: usize) {
    core::hint::spin_loop();
    #[cfg(all(not(test), any(target_arch = "x86_64", target_arch = "x86")))]
    {
        if iteration & 0xff == 0 {
            // The synchronous block facade can run before the vendor transport
            // IRQ path is useful. A periodic port-I/O delay gives QEMU's device
            // model a VM-exit opportunity.
            unsafe {
                crate::arch::x86::include::asm::io::native_io_delay();
            }
        }
    }
}

/// `blk_mq_alloc_request` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_alloc_request(
    q: *mut LinuxRequestQueue,
    op: u32,
    _flags: u32,
) -> *mut c_void {
    if q.is_null() {
        return linux_err_ptr(EINVAL);
    }

    let tag_set = unsafe { (*q).tag_set };
    let cmd_size = if tag_set.is_null() {
        0
    } else {
        unsafe { (*tag_set).cmd_size as usize }
    };
    let len = LINUX_REQUEST_SIZE.saturating_add(cmd_size);
    let words = len.div_ceil(core::mem::size_of::<usize>());
    let mut storage = Vec::new();
    storage.resize(words, 0usize);
    let rq = storage.as_mut_ptr().cast::<LinuxRequest>();
    let hctx = linux_request_hctx(q);
    unsafe {
        (*rq).q = q;
        (*rq).mq_hctx = hctx;
        (*rq).cmd_flags = op;
        (*rq).state = MQ_RQ_IDLE;
        if !q.is_null() {
            let disk = (*q).disk;
            if !disk.is_null() {
                (*rq).part = (*disk).part0.cast::<c_void>();
            }
        }
    }
    LINUX_REQUESTS.lock().push(LinuxRequestAllocation {
        ptr: rq as usize,
        len,
        bio: None,
        linux_bio: None,
        linux_bvecs: Vec::new(),
        _words: storage,
    });
    if !tag_set.is_null() {
        let ops = unsafe { (*tag_set).ops.cast::<LinuxBlkMqOps>() };
        if !ops.is_null() {
            if let Some(init_request) = unsafe { (*ops).init_request } {
                let ret = unsafe { init_request(tag_set, rq, 0, 0) };
                if ret != 0 {
                    LINUX_REQUESTS
                        .lock()
                        .retain(|allocation| allocation.ptr != rq as usize);
                    return linux_err_ptr(ret.unsigned_abs() as i32);
                }
            }
        }
    }
    rq.cast::<c_void>()
}

/// `blk_mq_free_request` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_free_request(rq: *mut LinuxRequest) {
    if rq.is_null() {
        return;
    }
    let q = unsafe { (*rq).q };
    if !q.is_null() {
        let tag_set = unsafe { (*q).tag_set };
        if !tag_set.is_null() {
            let ops = unsafe { (*tag_set).ops.cast::<LinuxBlkMqOps>() };
            if !ops.is_null() {
                if let Some(exit_request) = unsafe { (*ops).exit_request } {
                    unsafe {
                        exit_request(tag_set, rq, 0);
                    }
                }
            }
        }
    }
    LINUX_REQUESTS
        .lock()
        .retain(|allocation| allocation.ptr != rq as usize);
}

fn linux_cleanup_request_for_retry(rq: *mut LinuxRequest) {
    if rq.is_null() {
        return;
    }
    let q = unsafe { (*rq).q };
    if q.is_null() {
        return;
    }
    let tag_set = unsafe { (*q).tag_set };
    if tag_set.is_null() {
        return;
    }
    let ops = unsafe { (*tag_set).ops.cast::<LinuxBlkMqOps>() };
    if ops.is_null() {
        return;
    }
    if let Some(cleanup_rq) = unsafe { (*ops).cleanup_rq } {
        unsafe {
            cleanup_rq(rq);
        }
    }
}

fn linux_prepare_request_from_bio(
    q: *mut LinuxRequestQueue,
    bio: &BioRef,
) -> Result<*mut LinuxRequest, i32> {
    let vecs = bio.vecs.lock();
    let total_len = vecs
        .iter()
        .try_fold(0usize, |acc, vec| acc.checked_add(vec.len))
        .ok_or(EINVAL)?;
    let total_len = u32::try_from(total_len).map_err(|_| EINVAL)?;
    let nr_phys_segments = u16::try_from(vecs.len()).map_err(|_| EINVAL)?;
    let linux_bvecs = linux_bio_vecs_from_bio(&vecs)?;
    drop(vecs);

    let rq = unsafe { blk_mq_alloc_request(q, bio.op.0 as u32, 0) };
    if let Some(errno) = linux_ptr_errno(rq) {
        return Err(errno);
    }
    let rq = rq.cast::<LinuxRequest>();
    if rq.is_null() {
        return Err(ENOMEM);
    }

    unsafe {
        (*rq).cmd_flags = bio.op.0 as u32;
        (*rq).data_len = total_len;
        (*rq).sector = bio.sector;
        (*rq).nr_phys_segments = nr_phys_segments;
    }
    let mut allocations = LINUX_REQUESTS.lock();
    let Some(allocation) = allocations
        .iter_mut()
        .find(|allocation| allocation.ptr == rq as usize)
    else {
        drop(allocations);
        unsafe {
            blk_mq_free_request(rq);
        }
        return Err(EINVAL);
    };
    allocation.bio = Some(bio.clone());
    linux_install_request_bio_mirror(
        allocation,
        rq,
        linux_request_bdev(rq),
        bio.op.0 as u32,
        bio.sector,
        total_len,
        nr_phys_segments,
        linux_bvecs,
    );
    drop(allocations);
    Ok(rq)
}

/// Invoke a Linux `struct blk_mq_ops.queue_rq` callback.
///
/// This is generic block-core ABI glue: the request is executed by the
/// Linux-built driver that installed `q->mq_ops`, not by a local Rust driver.
pub unsafe fn linux_queue_request(
    rq: *mut LinuxRequest,
    last: bool,
) -> Result<LinuxBlkStatus, i32> {
    if rq.is_null() {
        return Err(EINVAL);
    }
    let q = unsafe { (*rq).q };
    if q.is_null() {
        return Err(EINVAL);
    }
    let ops = unsafe { (*q).mq_ops.cast::<LinuxBlkMqOps>() };
    if ops.is_null() {
        return Err(ENODEV);
    }
    let Some(queue_rq) = (unsafe { (*ops).queue_rq }) else {
        return Err(ENODEV);
    };
    let hctx = unsafe { (*rq).mq_hctx };
    if hctx.is_null() {
        return Err(ENODEV);
    }
    let bd = LinuxBlkMqQueueData { rq, last };
    Ok(unsafe { queue_rq(hctx, &bd) })
}

/// `blk_mq_start_request` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_start_request(rq: *mut c_void) {
    if !rq.is_null() {
        unsafe {
            (*rq.cast::<LinuxRequest>()).state = MQ_RQ_IN_FLIGHT;
        }
    }
}

/// `blk_mq_end_request` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_end_request(rq: *mut LinuxRequest, status: LinuxBlkStatus) {
    if rq.is_null() {
        return;
    }
    linux_request_mark_completed(rq, status);
}

/// `blk_mq_complete_request` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_complete_request(rq: *mut LinuxRequest) {
    if rq.is_null() {
        return;
    }
    if unsafe { blk_mq_complete_request_remote(rq) } {
        return;
    }
    let q = unsafe { (*rq).q };
    let ops = if q.is_null() {
        core::ptr::null()
    } else {
        unsafe { (*q).mq_ops.cast::<LinuxBlkMqOps>() }
    };
    if !ops.is_null() {
        if let Some(complete) = unsafe { (*ops).complete } {
            unsafe {
                complete(rq);
            }
            return;
        }
    }
    unsafe {
        blk_mq_end_request(rq, BLK_STS_OK);
    }
}

/// `blk_mq_complete_request_remote` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_complete_request_remote(rq: *mut LinuxRequest) -> bool {
    let _ = rq;
    false
}

/// `blk_mq_end_request_batch` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_end_request_batch(_iob: *mut c_void) {}

/// `blk_mq_requeue_request` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_requeue_request(_rq: *mut c_void, _kick_requeue_list: bool) {}

/// `blk_mq_stop_hw_queue` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_stop_hw_queue(_hctx: *mut c_void) {}

/// `blk_mq_start_stopped_hw_queues` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_start_stopped_hw_queues(_q: *mut LinuxRequestQueue, _async_: bool) {
}

/// `blk_mq_quiesce_queue_nowait` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_quiesce_queue_nowait(_q: *mut LinuxRequestQueue) {}

/// `blk_mq_unquiesce_queue` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_unquiesce_queue(_q: *mut LinuxRequestQueue) {}

/// `blk_mq_freeze_queue_nomemsave` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_freeze_queue_nomemsave(_q: *mut LinuxRequestQueue) {}

/// `blk_mq_unfreeze_queue_nomemrestore` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_unfreeze_queue_nomemrestore(_q: *mut LinuxRequestQueue) {}

/// `blk_mq_num_possible_queues` - `vendor/linux/block/blk-mq-cpumap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_num_possible_queues(max_queues: u32) -> u32 {
    if max_queues == 0 {
        1
    } else {
        max_queues.min(1)
    }
}

/// `blk_mq_map_queues` - `vendor/linux/block/blk-mq-cpumap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_map_queues(map: *mut LinuxBlkMqQueueMap) {
    if map.is_null() {
        return;
    }
    unsafe {
        if (*map).nr_queues == 0 {
            (*map).nr_queues = 1;
        }
        (*map).queue_offset = 0;
    }
}

/// `blk_mq_map_hw_queues` - `vendor/linux/block/blk-mq-cpumap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_mq_map_hw_queues(
    map: *mut LinuxBlkMqQueueMap,
    _mask: *const c_void,
    queue_offset: u32,
) {
    if map.is_null() {
        return;
    }
    unsafe {
        if (*map).nr_queues == 0 {
            (*map).nr_queues = 1;
        }
        (*map).queue_offset = queue_offset;
    }
}

/// `__blk_rq_map_sg` - `vendor/linux/block/blk-merge.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __blk_rq_map_sg(
    rq: *mut LinuxRequest,
    sglist: *mut crate::lib::scatterlist::LinuxScatterList,
    last_sg: *mut *mut crate::lib::scatterlist::LinuxScatterList,
) -> i32 {
    if rq.is_null() || sglist.is_null() {
        return 0;
    }
    if unsafe { (*rq).rq_flags & RQF_SPECIAL_PAYLOAD != 0 } {
        let ptr = unsafe { (*rq).special_vec_bv_page };
        let len = unsafe { (*rq).special_vec_bv_len };
        if ptr.is_null() || len == 0 {
            return 0;
        }
        unsafe {
            crate::lib::scatterlist::linux_sg_init_one(sglist, ptr, len);
            if !last_sg.is_null() {
                *last_sg = sglist;
            }
        }
        return 1;
    }
    let Some(bio) = linux_request_bio(rq) else {
        return 0;
    };
    let vecs = bio.vecs.lock();
    let mut mapped = 0usize;
    for (idx, vec) in vecs.iter().enumerate() {
        let mut data = vec.data.lock();
        if vec.off > data.len() || vec.len > data.len().saturating_sub(vec.off) {
            break;
        }
        let sg = unsafe { sglist.add(idx) };
        let ptr = unsafe { data.as_mut_ptr().add(vec.off).cast::<c_void>() };
        unsafe {
            crate::lib::scatterlist::linux_sg_init_one(sg, ptr, vec.len as u32);
            if idx + 1 != vecs.len() {
                (*sg).page_link &= !crate::lib::scatterlist::SG_END;
            }
        }
        mapped += 1;
    }
    if !last_sg.is_null() {
        unsafe {
            *last_sg = if mapped == 0 {
                core::ptr::null_mut()
            } else {
                sglist.add(mapped - 1)
            };
        }
    }
    mapped as i32
}

/// `blk_rq_map_kern` - `vendor/linux/block/blk-map.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_rq_map_kern(
    rq: *mut LinuxRequest,
    kbuf: *mut c_void,
    len: u32,
    _gfp_mask: u32,
) -> i32 {
    if rq.is_null() || kbuf.is_null() || len == 0 {
        return -EINVAL;
    }
    let mut linux_bvecs = Vec::new();
    if linux_bvecs.try_reserve(1).is_err() {
        return -ENOMEM;
    }
    linux_bvecs.push(LinuxBioVec {
        bv_page: core::ptr::null_mut(),
        bv_len: len,
        bv_offset: 0,
    });
    let bdev = linux_request_bdev(rq);
    let (opf, sector) = unsafe { ((*rq).cmd_flags, (*rq).sector) };
    unsafe {
        if (*rq).data_len != 0 {
            crate::log_warn!(
                "block",
                "blk_rq_map_kern: multiple appended buffers are not supported rq={:p}",
                rq
            );
            return -EINVAL;
        }
        (*rq).data_len = len;
        (*rq).nr_phys_segments = 1;
        (*rq).special_vec_bv_page = kbuf;
        (*rq).special_vec_bv_len = len;
        (*rq).special_vec_bv_offset = 0;
        (*rq).rq_flags |= RQF_SPECIAL_PAYLOAD;
    }
    let mut allocations = LINUX_REQUESTS.lock();
    let Some(allocation) = allocations
        .iter_mut()
        .find(|allocation| allocation.ptr == rq as usize)
    else {
        return -EINVAL;
    };
    linux_install_request_bio_mirror(allocation, rq, bdev, opf, sector, len, 1, linux_bvecs);
    0
}

/// `blk_execute_rq` - `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_execute_rq(rq: *mut LinuxRequest, _at_head: bool) -> LinuxBlkStatus {
    if rq.is_null() {
        return BLK_STS_IOERR;
    }
    linux_request_reset_completion(rq);
    let status = match unsafe { linux_queue_request(rq, true) } {
        Ok(status) => status,
        Err(err) => {
            crate::log_warn!("block", "blk_execute_rq: queue_rq failed errno={}", err);
            return BLK_STS_IOERR;
        }
    };
    if status != BLK_STS_OK {
        return status;
    }

    // `vendor/linux/block/blk-mq.c::blk_execute_rq()` waits on its completion
    // without a caller-side deadline.  The request and any caller-owned
    // passthrough buffer must remain live until `blk_mq_end_request()` runs.
    block_io_wait_for_completion(rq)
}

/// `blk_status_to_errno` - `vendor/linux/block/blk-core.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blk_status_to_errno(status: LinuxBlkStatus) -> i32 {
    if status == 0 { 0 } else { -EIO }
}

/// `queue_limits_commit_update_frozen` - `vendor/linux/block/blk-settings.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn queue_limits_commit_update_frozen(
    q: *mut LinuxRequestQueue,
    lim: *const LinuxQueueLimits,
) -> i32 {
    if q.is_null() || lim.is_null() {
        return -EINVAL;
    }
    unsafe {
        (*q).limits = *lim;
        crate::kernel::locking::mutex::linux_mutex_unlock(linux_request_queue_limits_lock(q));
    }
    0
}

/// Queue portion of `blk_mq_alloc_queue` — `vendor/linux/block/blk-mq.c`.
pub unsafe fn linux_blk_mq_alloc_queue(
    set: *mut LinuxBlkMqTagSet,
    lim: *const LinuxQueueLimits,
    queuedata: *mut c_void,
) -> *mut LinuxRequestQueue {
    if set.is_null() {
        return linux_err_ptr(EINVAL).cast::<LinuxRequestQueue>();
    }
    let mut limits = if lim.is_null() {
        unsafe { core::mem::zeroed::<LinuxQueueLimits>() }
    } else {
        unsafe { *lim }
    };
    if limits.logical_block_size == 0 {
        limits.logical_block_size = 512;
    }
    if limits.physical_block_size == 0 {
        limits.physical_block_size = limits.logical_block_size;
    }

    let queue = Box::into_raw(Box::new(LinuxRequestQueue {
        queuedata,
        elevator: core::ptr::null_mut(),
        mq_ops: unsafe { (*set).ops },
        queue_ctx: core::ptr::null_mut(),
        queue_flags: 0,
        rq_timeout: unsafe { (*set).timeout },
        queue_depth: unsafe { (*set).queue_depth },
        refs: 1,
        nr_hw_queues: unsafe { (*set).nr_hw_queues },
        queue_hw_ctx: core::ptr::null_mut(),
        _pad_after_queue_hw_ctx: [0; LINUX_REQUEST_QUEUE_DISK_OFFSET - 0x40],
        disk: core::ptr::null_mut(),
        mq_kobj: core::ptr::null_mut(),
        limits,
        _pad_after_limits: [0; LINUX_REQUEST_QUEUE_PM_ONLY_OFFSET
            - (0x70 + core::mem::size_of::<LinuxQueueLimits>())],
        pm_only: 0,
        _pad_after_pm_only: [0; LINUX_REQUEST_QUEUE_STATS_OFFSET
            - (LINUX_REQUEST_QUEUE_PM_ONLY_OFFSET + core::mem::size_of::<i32>())],
        stats: core::ptr::null_mut(),
        rq_qos: core::ptr::null_mut(),
        rq_qos_mutex: LinuxAbiMutex::new(),
        id: 0,
        nr_requests: unsafe { (*set).queue_depth },
        async_depth: unsafe { (*set).queue_depth },
        _pad_after_async_depth: [0; LINUX_REQUEST_QUEUE_LIMITS_LOCK_OFFSET
            - (LINUX_REQUEST_QUEUE_ASYNC_DEPTH_OFFSET + core::mem::size_of::<u32>())],
        limits_lock: LinuxAbiMutex::new(),
        _pad_after_limits_lock: [0; LINUX_REQUEST_QUEUE_TAG_SET_OFFSET
            - (LINUX_REQUEST_QUEUE_LIMITS_LOCK_OFFSET + LINUX_MUTEX_SIZE)],
        tag_set: set,
        _pad_after_tag_set: [0; LINUX_REQUEST_QUEUE_SIZE
            - (LINUX_REQUEST_QUEUE_TAG_SET_OFFSET + core::mem::size_of::<*mut LinuxBlkMqTagSet>())],
        hctx_table_storage: core::ptr::null_mut(),
    }));
    unsafe {
        crate::kernel::locking::mutex::linux_mutex_init_generic(
            linux_request_queue_limits_lock(queue),
            core::ptr::null(),
            core::ptr::null_mut(),
        );
    }
    crate::log_info!(
        "block",
        "blk_mq_alloc_queue: set={:p} ops={:p} queue_rq={:#x} queuedata={:p} cmd_size={} depth={}",
        set,
        unsafe { (*set).ops },
        if unsafe { (*set).ops }.is_null() {
            0
        } else {
            unsafe { (*unsafe { (*set).ops }.cast::<LinuxBlkMqOps>()).queue_rq }
                .map(|f| f as usize)
                .unwrap_or(0)
        },
        queuedata,
        unsafe { (*set).cmd_size },
        unsafe { (*set).queue_depth }
    );
    crate::log_warn!(
        "block",
        "blk_mq_alloc_queue: before hctx alloc set={:p} queue={:p} driver_data={:p}",
        set,
        queue,
        unsafe { (*set).driver_data }
    );
    let hctx = Box::into_raw(Box::new(LinuxBlkMqHwCtx {
        _pad_to_queue: [0; LINUX_BLK_MQ_HW_CTX_QUEUE_OFFSET],
        queue,
        _pad_to_driver_data: [0; LINUX_BLK_MQ_HW_CTX_DRIVER_DATA_OFFSET
            - (LINUX_BLK_MQ_HW_CTX_QUEUE_OFFSET + core::mem::size_of::<*mut LinuxRequestQueue>())],
        driver_data: core::ptr::null_mut(),
        _pad_to_queue_num: [0; LINUX_BLK_MQ_HW_CTX_QUEUE_NUM_OFFSET
            - (LINUX_BLK_MQ_HW_CTX_DRIVER_DATA_OFFSET + core::mem::size_of::<*mut c_void>())],
        queue_num: 0,
        _pad_after_queue_num: [0; LINUX_BLK_MQ_HW_CTX_SIZE
            - (LINUX_BLK_MQ_HW_CTX_QUEUE_NUM_OFFSET + 4)],
    }));
    crate::log_warn!(
        "block",
        "blk_mq_alloc_queue: after hctx alloc hctx={:p} queue={:p}",
        hctx,
        unsafe { (*hctx).queue }
    );
    if !unsafe { (*set).ops }.is_null() {
        let ops = unsafe { (*set).ops.cast::<LinuxBlkMqOps>() };
        if let Some(init_hctx) = unsafe { (*ops).init_hctx } {
            crate::log_warn!(
                "block",
                "blk_mq_alloc_queue: before init_hctx hctx={:p} data={:p} fn={:#x}",
                hctx,
                unsafe { (*set).driver_data },
                init_hctx as usize
            );
            let ret = unsafe { init_hctx(hctx, (*set).driver_data, 0) };
            crate::log_warn!(
                "block",
                "blk_mq_alloc_queue: after init_hctx hctx={:p} driver_data={:p} ret={}",
                hctx,
                unsafe { (*hctx).driver_data },
                ret
            );
            if ret != 0 {
                unsafe {
                    let _ = Box::from_raw(hctx);
                    let _ = Box::from_raw(queue);
                }
                let errno = if ret < 0 { ret.saturating_neg() } else { ret };
                return linux_err_ptr(errno).cast::<LinuxRequestQueue>();
            }
        }
    }
    crate::log_warn!(
        "block",
        "blk_mq_alloc_queue: before hctx table hctx={:p} queue={:p}",
        hctx,
        queue
    );
    unsafe {
        (*queue).hctx_table_storage = hctx;
        (*queue).queue_hw_ctx =
            core::ptr::addr_of_mut!((*queue).hctx_table_storage).cast::<c_void>();
    }
    crate::log_warn!(
        "block",
        "blk_mq_alloc_queue: after hctx table queue_hw_ctx={:p}",
        unsafe { (*queue).queue_hw_ctx }
    );
    queue
}

/// Disk allocation portion of `__alloc_disk_node` for an existing request queue.
pub unsafe fn linux_blk_mq_alloc_disk_for_queue(
    queue: *mut LinuxRequestQueue,
    private_data: *mut c_void,
    take_queue_ref: bool,
) -> *mut LinuxGendisk {
    if queue.is_null() || linux_ptr_errno(queue.cast::<c_void>()).is_some() {
        return core::ptr::null_mut();
    }
    if take_queue_ref {
        unsafe {
            (*queue).refs = (*queue).refs.saturating_add(1);
        }
    }
    crate::log_warn!(
        "block",
        "blk_mq_alloc_disk_for_queue: before disk alloc queue={:p}",
        queue
    );
    let disk = Box::into_raw(Box::new(LinuxGendisk {
        major: 0,
        first_minor: 0,
        minors: 0,
        disk_name: [0; DISK_NAME_LEN],
        events: 0,
        event_flags: 0,
        part_tbl: [0; 2],
        part0: core::ptr::null_mut(),
        fops: core::ptr::null(),
        queue,
        private_data,
        bio_split: [0; LINUX_BIO_SET_SIZE],
        flags: 0,
        _pad_after_flags: [0; LINUX_GENDISK_STATE_OFFSET
            - (LINUX_GENDISK_FLAGS_OFFSET + core::mem::size_of::<i32>())],
        state: 0,
        _pad_after_state: [0; LINUX_GENDISK_SIZE
            - (LINUX_GENDISK_STATE_OFFSET + core::mem::size_of::<usize>())],
    }));
    crate::log_warn!(
        "block",
        "blk_mq_alloc_disk_for_queue: after disk alloc disk={:p}",
        disk
    );
    crate::log_warn!("block", "blk_mq_alloc_disk_for_queue: before part0 alloc");
    let part0 = Box::into_raw(Box::new(LinuxBlockDevicePrefix {
        bd_start_sect: 0,
        bd_nr_sectors: 0,
        bd_disk: disk,
        bd_queue: queue,
        _pad_after_bd_queue: [0; LINUX_BLOCK_DEVICE_SIZE - 0x20],
    }));
    crate::log_warn!(
        "block",
        "blk_mq_alloc_disk_for_queue: after part0 alloc part0={:p}",
        part0
    );
    unsafe {
        (*disk).part0 = part0;
        (*queue).disk = disk;
    }
    crate::log_warn!("block", "__blk_mq_alloc_disk: complete disk={:p}", disk);
    disk
}

/// `__blk_mq_alloc_disk` — `vendor/linux/block/blk-mq.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __blk_mq_alloc_disk(
    set: *mut LinuxBlkMqTagSet,
    lim: *const LinuxQueueLimits,
    queuedata: *mut c_void,
    _lkclass: *mut c_void,
) -> *mut LinuxGendisk {
    let queue = unsafe { linux_blk_mq_alloc_queue(set, lim, queuedata) };
    if linux_ptr_errno(queue.cast::<c_void>()).is_some() {
        return queue.cast::<LinuxGendisk>();
    }
    let disk = unsafe { linux_blk_mq_alloc_disk_for_queue(queue, queuedata, false) };
    if disk.is_null() {
        unsafe {
            let hctx_table = (*queue).queue_hw_ctx as *mut *mut LinuxBlkMqHwCtx;
            if !hctx_table.is_null() {
                let hctx = *hctx_table;
                if !hctx.is_null() {
                    let _ = Box::from_raw(hctx);
                }
            }
            let _ = Box::from_raw(queue);
        }
        return linux_err_ptr(ENOMEM).cast::<LinuxGendisk>();
    }
    unsafe {
        (*disk).state |= 1usize << GD_OWNS_QUEUE;
    }
    disk
}

/// `device_add_disk` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub(crate) fn linux_disk_name(disk: *const LinuxGendisk) -> Result<String, i32> {
    if disk.is_null() {
        return Err(EINVAL);
    }
    disk_name_bytes_to_string(unsafe { &(*disk).disk_name })
}

fn linux_gendisk_backing(bdev: &BlockDeviceRef) -> Result<Arc<LinuxGendiskBlockDevice>, i32> {
    bdev.backing
        .lock()
        .as_ref()
        .cloned()
        .and_then(|backing| backing.downcast::<LinuxGendiskBlockDevice>().ok())
        .ok_or(ENODEV)
}

fn linux_gendisk_request_queue(
    backing: &LinuxGendiskBlockDevice,
) -> Result<*mut LinuxRequestQueue, i32> {
    let disk = backing.disk as *mut LinuxGendisk;
    if disk.is_null() || !linux_disk_registered(disk) {
        return Err(ENODEV);
    }
    let queue = unsafe { (*disk).queue };
    if queue.is_null() {
        Err(ENODEV)
    } else {
        Ok(queue)
    }
}

/// Owner of the single in-flight slot of the synchronous block facade, or 0 if
/// free. The facade does not allocate per-request blk_mq tags — every request
/// uses tag 0 — so concurrent submitters (possible because the completion wait
/// now yields) would collide on the same NCQ tag / AHCI command slot and lose
/// each other's completions. We therefore serialize: one command at a time.
///
/// The value is the owning task pointer so the guard is reentrant — a nested
/// block read from within an in-flight submission (e.g. a filesystem probe that
/// reads while servicing a read) proceeds instead of self-deadlocking.
static BLOCK_FACADE_OWNER: AtomicUsize = AtomicUsize::new(0);

/// Guard for the single-in-flight slot. `held` is true only for the outermost
/// acquisition that actually owns the slot; nested/bypass acquisitions carry
/// `held = false` and release nothing.
struct BlockFacadeGuard {
    held: bool,
}

impl Drop for BlockFacadeGuard {
    fn drop(&mut self) {
        if self.held {
            BLOCK_FACADE_OWNER.store(0, Ordering::Release);
        }
    }
}

/// Acquire the single-in-flight slot. Reentrant for the owning task; a no-op
/// when there is no live current task (early boot / non-task context), where
/// serialization is neither possible nor needed.
///
/// When the slot is busy the caller *sleeps* (`TASK_INTERRUPTIBLE`) rather than
/// busy-yielding, arming a ≤1-tick re-poll wakeup before each sleep so it
/// re-attempts the CAS within a jiffy of the holder releasing. This mirrors the
/// proven `block_io_wait_for_completion` pattern. It matters because systemd's
/// generators fire ~20 reads concurrently: a busy-yield leaves all 20 runnable,
/// so each release is only noticed on the next round-robin pass — invisible on
/// KVM but ~40ms/handoff under VirtualBox's VM-exit-heavy `schedule()`, i.e.
/// ~1s of serialized stall. Sleeping waiters keep the runqueue short (so the
/// holder's own completion poll advances faster) and bound the handoff to one
/// tick. A non-sleepable (atomic) context falls back to the bounded yield.
fn block_facade_acquire() -> Result<BlockFacadeGuard, i32> {
    let current = unsafe { crate::kernel::sched::get_current() };
    let cur_id = current as usize;
    if cur_id == 0 {
        return Ok(BlockFacadeGuard { held: false });
    }
    let mut spin = 0usize;
    loop {
        match BLOCK_FACADE_OWNER.compare_exchange(0, cur_id, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return Ok(BlockFacadeGuard { held: true }),
            // Reentrant: this task already owns the slot.
            Err(owner) if owner == cur_id => return Ok(BlockFacadeGuard { held: false }),
            Err(owner) => {
                if spin >= BLOCK_FACADE_ACQUIRE_SPIN_LIMIT {
                    crate::log_warn!(
                        "block",
                        "block_facade_acquire: timeout owner={:#x} current={:#x} spins={}",
                        owner,
                        cur_id,
                        spin
                    );
                    return Err(EIO);
                }
            }
        }
        if crate::kernel::locking::preempt::preempt_count() != 0 {
            // Atomic context can't sleep: bounded yield, then retry.
            linux_request_wait_cpu_relax(spin);
            spin = spin.wrapping_add(1);
            continue;
        }
        // Sleep until the holder releases. Mark interruptible, re-check the slot
        // (so a release racing in cannot be lost), arm a ≤1-tick re-poll wakeup,
        // then yield so the CPU can halt. On wake, retry the CAS.
        unsafe {
            (*current).__state.store(
                crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                Ordering::Release,
            );
        }
        if BLOCK_FACADE_OWNER.load(Ordering::Acquire) == 0 {
            unsafe {
                (*current).__state.store(
                    crate::kernel::task::task_state::TASK_RUNNING,
                    Ordering::Release,
                );
            }
            continue;
        }
        let wake_at = crate::kernel::time::jiffies::jiffies().saturating_add(1);
        crate::kernel::time::sleep_timeout::arm_wakeup(cur_id, wake_at);
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        crate::kernel::time::sleep_timeout::cancel_wakeup(cur_id);
        unsafe {
            (*current).__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                Ordering::Release,
            );
        }
        spin = spin.wrapping_add(1);
    }
}

#[cfg(not(test))]
const BLOCK_FACADE_ACQUIRE_SPIN_LIMIT: usize = 2048;
#[cfg(test)]
const BLOCK_FACADE_ACQUIRE_SPIN_LIMIT: usize = 64;
/// Poll a block request until it reaches `MQ_RQ_COMPLETE`.
///
/// The synchronous facade is used while discovering and mounting the root disk,
/// where sleeping for an interrupt is fragile: QEMU/VirtualBox can complete the
/// emulated request without delivering an IRQ path that wakes the submitting
/// task. Each pass drives the software reaper and the driver's `mq_ops::poll`,
/// then performs a CPU/VM-exit relax.
///
/// There is deliberately no production poll-count timeout. Vendor Linux keeps
/// an in-flight request and its driver PDU alive until the driver ends it. In
/// particular, virtio-blk has no `blk_mq_ops::timeout` callback, so
/// `blk_mq_rq_timed_out()` re-arms the request timer and continues waiting.
/// Returning merely because the host executed a chosen number of spins would
/// let the synchronous facade free memory still owned by the virtqueue, turning
/// a late `virtblk_done()` into a use-after-free.
fn block_io_wait_for_completion(rq: *mut LinuxRequest) -> LinuxBlkStatus {
    let current = unsafe { crate::kernel::sched::get_current() };
    let pump_global_driver_events = !linux_request_targets_virtio_disk(rq);
    let mut wait_polls = 0usize;
    if current.is_null() {
        // No task context (early boot): poll until completion.
        loop {
            if let Some(status) = linux_request_completed_status(rq) {
                return status;
            }
            if pump_global_driver_events {
                let _ = crate::linux_driver_abi::poll_driver_abi_events();
            }
            let _ = linux_poll_request_queue(rq);
            linux_request_poll_cpu_relax(wait_polls);
            wait_polls = wait_polls.saturating_add(1);
        }
    }

    let status = loop {
        if let Some(status) = linux_request_completed_status(rq) {
            break status;
        }
        if pump_global_driver_events {
            let _ = crate::linux_driver_abi::poll_driver_abi_events();
        }
        let _ = linux_poll_request_queue(rq);
        if pump_global_driver_events {
            // SCSI/libata completion can require a peer completion or error-
            // handling task to run. `blk_wait_io()` sleeps/yields in vendor
            // Linux; preserve that progress guarantee for task-context waits.
            linux_request_wait_cpu_relax(wait_polls);
        } else {
            // The virtio backend is polled directly above and does not need a
            // peer task; avoid adding a scheduler-tick delay to every root read.
            linux_request_poll_cpu_relax(wait_polls);
        }
        wait_polls = wait_polls.saturating_add(1);
    };
    unsafe {
        (*current).__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
    }
    status
}

/// Diagnostic: read the SCSI target/device busy counters behind a request queue
/// to see which scsi accounting is exhausted when queue_rq returns RESOURCE.
/// Offsets are the vendor-probed `struct scsi_device`/`scsi_target` layout
/// (ROADMAP 130A): scsi_device.sdev_target @0x128, scsi_target.target_busy
/// @0x1e8, target_blocked @0x1ec, can_queue @0x1f0. Removable.
unsafe fn dump_scsi_busy_state(q: *mut LinuxRequestQueue) {
    if q.is_null() {
        return;
    }
    let sdev = unsafe { (*q).queuedata } as *const u8;
    if sdev.is_null() {
        return;
    }
    let starget = unsafe { core::ptr::read_unaligned(sdev.add(0x128) as *const *const u8) };
    let (target_busy, target_blocked, can_queue) = if starget.is_null() {
        (0i32, 0i32, 0i32)
    } else {
        unsafe {
            (
                core::ptr::read_unaligned(starget.add(0x1e8) as *const i32),
                core::ptr::read_unaligned(starget.add(0x1ec) as *const i32),
                core::ptr::read_unaligned(starget.add(0x1f0) as *const i32),
            )
        }
    };
    crate::log_warn!(
        "block",
        "scsi-busy: sdev={:p} starget={:p} target_busy={} target_blocked={} can_queue={}",
        sdev,
        starget,
        target_busy,
        target_blocked,
        can_queue
    );
}

fn linux_queue_submit_bio(q: *mut LinuxRequestQueue, bio: &BioRef) -> Result<(), i32> {
    let _inflight = block_facade_acquire()?;
    let mut rq = linux_prepare_request_from_bio(q, bio)?;
    const COMPLETION_SPINS: usize = 1_000_000;
    let mut queue_retries = 0usize;
    let mut driver_event_polls = 0usize;
    let mut mq_polls = 0usize;

    loop {
        let queue_status = unsafe { linux_queue_request(rq, true) };
        let status = match queue_status {
            Ok(status) => status,
            Err(err) => {
                unsafe {
                    blk_mq_free_request(rq);
                }
                return Err(err);
            }
        };
        if status == BLK_STS_OK {
            break;
        }
        if !linux_blk_status_retryable(status) || queue_retries >= COMPLETION_SPINS {
            let hctx = unsafe { (*rq).mq_hctx };
            let queue_num = if hctx.is_null() {
                u32::MAX
            } else {
                unsafe { (*hctx).queue_num }
            };
            crate::log_warn!(
                "block",
                "linux_queue_submit_bio: queue_rq failed status={} op={} sector={} bytes={} segments={} hctx_queue={} retries={}",
                status,
                unsafe { (*rq).cmd_flags },
                unsafe { (*rq).sector },
                unsafe { (*rq).data_len },
                unsafe { (*rq).nr_phys_segments },
                queue_num,
                queue_retries
            );
            unsafe { dump_scsi_busy_state(q) };
            crate::linux_driver_abi::storage_core::debug_dump_ahci_bar5("queue_rq failed");
            linux_cleanup_request_for_retry(rq);
            unsafe {
                blk_mq_free_request(rq);
            }
            return linux_blk_status_result(status);
        }
        queue_retries = queue_retries.saturating_add(1);
        linux_cleanup_request_for_retry(rq);
        unsafe {
            blk_mq_free_request(rq);
        }
        driver_event_polls =
            driver_event_polls.saturating_add(crate::linux_driver_abi::poll_driver_abi_events());
        rq = linux_prepare_request_from_bio(q, bio)?;
        if linux_poll_request_queue(rq) > 0 {
            mq_polls = mq_polls.saturating_add(1);
        }
        linux_request_wait_cpu_relax(queue_retries);
    }

    // Event-driven completion wait: sleep until the request completes (woken by
    // the AHCI IRQ / reaper) instead of busy-spinning, so the CPU halts between
    // events rather than burning cycles (the cause of the slow, soft-locking
    // boot on real hardware that delivers completions with latency).
    let status = block_io_wait_for_completion(rq);
    let _ = (driver_event_polls, mq_polls);
    if status == BLK_STS_TIMEOUT {
        crate::log_warn!(
            "block",
            "linux_queue_submit_bio: timeout op={} sector={} bytes={} segments={} state={} queue_retries={}",
            unsafe { (*rq).cmd_flags },
            unsafe { (*rq).sector },
            unsafe { (*rq).data_len },
            unsafe { (*rq).nr_phys_segments },
            unsafe { (*rq).state },
            queue_retries
        );
        crate::linux_driver_abi::storage_core::debug_dump_ahci_bar5("request timeout");
        unsafe {
            blk_mq_free_request(rq);
        }
        return Err(EIO);
    }
    if status != BLK_STS_OK {
        crate::log_warn!(
            "block",
            "linux_queue_submit_bio: completed status={} op={} sector={} bytes={} segments={}",
            status,
            unsafe { (*rq).cmd_flags },
            unsafe { (*rq).sector },
            unsafe { (*rq).data_len },
            unsafe { (*rq).nr_phys_segments }
        );
    }
    unsafe {
        blk_mq_free_request(rq);
    }
    linux_blk_status_result(status)
}

fn linux_gendisk_submit_bio(bdev: &BlockDeviceRef, bio: &BioRef) -> Result<(), i32> {
    let backing = linux_gendisk_backing(bdev)?;
    let q = linux_gendisk_request_queue(&backing)?;
    linux_queue_submit_bio(q, bio)
}

fn linux_gendisk_get_capacity(bdev: &BlockDeviceRef) -> u64 {
    let Ok(backing) = linux_gendisk_backing(bdev) else {
        return 0;
    };
    let disk = backing.disk as *const LinuxGendisk;
    if disk.is_null() {
        0
    } else {
        unsafe { linux_gendisk_capacity_sectors(disk) }
    }
}

fn linux_gendisk_block_size(bdev: &BlockDeviceRef) -> u32 {
    let Ok(backing) = linux_gendisk_backing(bdev) else {
        return 512;
    };
    let disk = backing.disk as *const LinuxGendisk;
    if disk.is_null() {
        return 512;
    }
    let q = unsafe { (*disk).queue };
    if q.is_null() {
        return 512;
    }
    let size = unsafe { (*q).limits.logical_block_size };
    if size == 0 { 512 } else { size }
}

static LINUX_GENDISK_BLOCK_OPS: BlockDeviceOps = BlockDeviceOps {
    name: "linux_gendisk",
    submit_bio: linux_gendisk_submit_bio,
    get_capacity: linux_gendisk_get_capacity,
    block_size: linux_gendisk_block_size,
    ioctl: None,
};

fn register_linux_gendisk_block_device(disk: *mut LinuxGendisk) -> Result<(), i32> {
    let name = linux_disk_name(disk)?;
    let backing = Arc::new(LinuxGendiskBlockDevice {
        disk: disk as usize,
    });
    let bdev = BlockDevice::wrap(backing, &LINUX_GENDISK_BLOCK_OPS);
    register_block_device(&name, bdev.clone())?;
    register_gendisk(&name, bdev);
    Ok(())
}

fn unregister_linux_gendisk_block_device(disk: *mut LinuxGendisk) {
    if let Ok(name) = linux_disk_name(disk) {
        let _ = unregister_block_device(&name);
        let _ = unregister_gendisk(&name);
    }
}

pub unsafe extern "C" fn device_add_disk(
    parent: *mut c_void,
    disk: *mut LinuxGendisk,
    _groups: *const *const c_void,
) -> i32 {
    if disk.is_null() {
        return -EINVAL;
    }
    let queue = unsafe { (*disk).queue };
    if queue.is_null() {
        return -EINVAL;
    }
    let mut disks = LINUX_DISKS.lock();
    if disks
        .iter()
        .any(|registered| registered.disk == disk as usize)
    {
        return -EBUSY;
    }
    unsafe {
        (*disk).state |= 1usize << GD_ADDED;
    }
    if let Err(err) = register_linux_gendisk_block_device(disk) {
        unsafe {
            (*disk).state &= !(1usize << GD_ADDED);
        }
        return -err;
    }
    disks.push(LinuxDiskRegistration {
        disk: disk as usize,
        queue: queue as usize,
        parent: parent as usize,
        name: unsafe { disk_name_bytes(&*disk) },
    });
    0
}

/// `put_disk` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn put_disk(disk: *mut LinuxGendisk) {
    if disk.is_null() {
        return;
    }
    unregister_linux_gendisk_block_device(disk);
    LINUX_DISKS
        .lock()
        .retain(|registered| registered.disk != disk as usize);
    unsafe {
        let queue = (*disk).queue;
        let part0 = (*disk).part0;
        if !part0.is_null() {
            let _ = Box::from_raw(part0);
        }
        if !queue.is_null() {
            let hctx_table = (*queue).queue_hw_ctx as *mut *mut LinuxBlkMqHwCtx;
            if !hctx_table.is_null() {
                let hctx = *hctx_table;
                if !hctx.is_null() {
                    let set = (*queue).tag_set;
                    if !set.is_null() && !(*set).ops.is_null() {
                        let ops = (*set).ops.cast::<LinuxBlkMqOps>();
                        if let Some(exit_hctx) = (*ops).exit_hctx {
                            exit_hctx(hctx, 0);
                        }
                    }
                    let _ = Box::from_raw(hctx);
                }
            }
            let _ = Box::from_raw(queue);
        }
        let _ = Box::from_raw(disk);
    }
}

/// `del_gendisk` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn del_gendisk(disk: *mut LinuxGendisk) {
    if disk.is_null() {
        return;
    }
    unregister_linux_gendisk_block_device(disk);
    LINUX_DISKS
        .lock()
        .retain(|registered| registered.disk != disk as usize);
    unsafe {
        (*disk).state &= !(1usize << GD_ADDED);
    }
}

/// `set_disk_ro` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_disk_ro(disk: *mut LinuxGendisk, read_only: bool) {
    if disk.is_null() {
        return;
    }
    unsafe {
        if read_only {
            (*disk).state |= 1usize << GD_READ_ONLY;
        } else {
            (*disk).state &= !(1usize << GD_READ_ONLY);
        }
    }
}

unsafe fn linux_gendisk_capacity_sectors(disk: *const LinuxGendisk) -> u64 {
    if disk.is_null() {
        return 0;
    }
    let part0 = unsafe { (*disk).part0 };
    if part0.is_null() {
        0
    } else {
        unsafe { (*part0).bd_nr_sectors }
    }
}

unsafe fn linux_gendisk_set_capacity_sectors(disk: *mut LinuxGendisk, sectors: u64) {
    if disk.is_null() {
        return;
    }
    unsafe {
        let part0 = (*disk).part0;
        if !part0.is_null() {
            (*part0).bd_nr_sectors = sectors;
        }
    }
}

/// `set_capacity` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_capacity(disk: *mut LinuxGendisk, sectors: u64) {
    unsafe {
        linux_gendisk_set_capacity_sectors(disk, sectors);
    }
}

/// `set_capacity_and_notify` — `vendor/linux/block/genhd.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_capacity_and_notify(disk: *mut LinuxGendisk, sectors: u64) -> bool {
    if disk.is_null() {
        return false;
    }
    unsafe {
        let changed = linux_gendisk_capacity_sectors(disk) != sectors;
        linux_gendisk_set_capacity_sectors(disk, sectors);
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::bio::{BIO_OP_WRITE, BioOp, BioVec, bio_alloc, submit_bio};
    use crate::block::block_device::BlockDevice;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};
    use core::mem::{align_of, offset_of, size_of};
    use core::sync::atomic::{AtomicUsize, Ordering};

    static QUEUE_RQ_CALLS: AtomicUsize = AtomicUsize::new(0);
    static QUEUE_RQ_REQUEST: AtomicUsize = AtomicUsize::new(0);
    static QUEUE_RQ_HCTX: AtomicUsize = AtomicUsize::new(0);
    static INIT_HCTX_CALLS: AtomicUsize = AtomicUsize::new(0);
    static INIT_HCTX_HCTX: AtomicUsize = AtomicUsize::new(0);
    static INIT_HCTX_DATA: AtomicUsize = AtomicUsize::new(0);
    static INIT_HCTX_INDEX: AtomicUsize = AtomicUsize::new(0);
    static EXIT_HCTX_CALLS: AtomicUsize = AtomicUsize::new(0);
    static EXIT_HCTX_HCTX: AtomicUsize = AtomicUsize::new(0);
    static EXIT_HCTX_INDEX: AtomicUsize = AtomicUsize::new(0);
    static POLL_QUEUE_RQ_CALLS: AtomicUsize = AtomicUsize::new(0);
    static POLL_CALLS: AtomicUsize = AtomicUsize::new(0);
    static POLL_REQUEST: AtomicUsize = AtomicUsize::new(0);
    static RETRY_QUEUE_RQ_CALLS: AtomicUsize = AtomicUsize::new(0);
    static RETRY_CLEANUP_CALLS: AtomicUsize = AtomicUsize::new(0);
    static RETRY_FIRST_REQUEST: AtomicUsize = AtomicUsize::new(0);
    static RETRY_SECOND_REQUEST: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_queue_rq(
        hctx: *mut LinuxBlkMqHwCtx,
        bd: *const LinuxBlkMqQueueData,
    ) -> LinuxBlkStatus {
        QUEUE_RQ_CALLS.fetch_add(1, Ordering::AcqRel);
        QUEUE_RQ_HCTX.store(hctx as usize, Ordering::Release);
        unsafe {
            QUEUE_RQ_REQUEST.store((*bd).rq as usize, Ordering::Release);
            assert!((*bd).last);
            assert_eq!((*(*bd).rq).mq_hctx, hctx);
            assert_eq!((*hctx).queue, (*(*bd).rq).q);
            blk_mq_end_request((*bd).rq, BLK_STS_OK);
        }
        BLK_STS_OK
    }

    unsafe extern "C" fn test_init_hctx(
        hctx: *mut LinuxBlkMqHwCtx,
        data: *mut c_void,
        hctx_idx: u32,
    ) -> i32 {
        INIT_HCTX_CALLS.fetch_add(1, Ordering::AcqRel);
        INIT_HCTX_HCTX.store(hctx as usize, Ordering::Release);
        INIT_HCTX_DATA.store(data as usize, Ordering::Release);
        INIT_HCTX_INDEX.store(hctx_idx as usize, Ordering::Release);
        unsafe {
            (*hctx).driver_data = data;
        }
        0
    }

    unsafe extern "C" fn test_exit_hctx(hctx: *mut LinuxBlkMqHwCtx, hctx_idx: u32) {
        EXIT_HCTX_CALLS.fetch_add(1, Ordering::AcqRel);
        EXIT_HCTX_HCTX.store(hctx as usize, Ordering::Release);
        EXIT_HCTX_INDEX.store(hctx_idx as usize, Ordering::Release);
    }

    static TEST_MQ_OPS: LinuxBlkMqOps = LinuxBlkMqOps {
        queue_rq: Some(test_queue_rq),
        commit_rqs: None,
        queue_rqs: None,
        get_budget: None,
        put_budget: None,
        set_rq_budget_token: None,
        get_rq_budget_token: None,
        timeout: None,
        poll: None,
        complete: None,
        init_hctx: Some(test_init_hctx),
        exit_hctx: Some(test_exit_hctx),
        init_request: None,
        exit_request: None,
        cleanup_rq: None,
        busy: None,
        map_queues: None,
        show_rq: None,
    };

    unsafe extern "C" fn test_deferred_queue_rq(
        _hctx: *mut LinuxBlkMqHwCtx,
        bd: *const LinuxBlkMqQueueData,
    ) -> LinuxBlkStatus {
        POLL_QUEUE_RQ_CALLS.fetch_add(1, Ordering::AcqRel);
        unsafe {
            POLL_REQUEST.store((*bd).rq as usize, Ordering::Release);
            blk_mq_start_request((*bd).rq.cast());
        }
        BLK_STS_OK
    }

    unsafe extern "C" fn test_poll(hctx: *mut LinuxBlkMqHwCtx, iob: *mut c_void) -> i32 {
        assert!(!hctx.is_null());
        assert!(
            iob.is_null(),
            "non-batched fallback passes a null io_comp_batch"
        );
        POLL_CALLS.fetch_add(1, Ordering::AcqRel);
        let rq = POLL_REQUEST.load(Ordering::Acquire) as *mut LinuxRequest;
        if rq.is_null() {
            return 0;
        }
        unsafe {
            assert_eq!((*rq).mq_hctx, hctx);
            blk_mq_end_request(rq, BLK_STS_OK);
        }
        1
    }

    static POLL_MQ_OPS: LinuxBlkMqOps = LinuxBlkMqOps {
        queue_rq: Some(test_deferred_queue_rq),
        commit_rqs: None,
        queue_rqs: None,
        get_budget: None,
        put_budget: None,
        set_rq_budget_token: None,
        get_rq_budget_token: None,
        timeout: None,
        poll: Some(test_poll),
        complete: None,
        init_hctx: None,
        exit_hctx: None,
        init_request: None,
        exit_request: None,
        cleanup_rq: None,
        busy: None,
        map_queues: None,
        show_rq: None,
    };

    unsafe extern "C" fn test_retry_queue_rq(
        _hctx: *mut LinuxBlkMqHwCtx,
        bd: *const LinuxBlkMqQueueData,
    ) -> LinuxBlkStatus {
        let call = RETRY_QUEUE_RQ_CALLS.fetch_add(1, Ordering::AcqRel);
        unsafe {
            let rq = (*bd).rq;
            if call == 0 {
                RETRY_FIRST_REQUEST.store(rq as usize, Ordering::Release);
                (*rq).rq_flags |= RQF_DONTPREP;
                blk_mq_start_request(rq.cast());
                return BLK_STS_RESOURCE;
            }
            RETRY_SECOND_REQUEST.store(rq as usize, Ordering::Release);
            assert_eq!((*rq).rq_flags & RQF_DONTPREP, 0);
            blk_mq_end_request(rq, BLK_STS_OK);
        }
        BLK_STS_OK
    }

    unsafe extern "C" fn test_retry_cleanup_rq(rq: *mut LinuxRequest) {
        RETRY_CLEANUP_CALLS.fetch_add(1, Ordering::AcqRel);
        unsafe {
            assert_eq!((*rq).rq_flags & RQF_DONTPREP, RQF_DONTPREP);
            (*rq).rq_flags &= !RQF_DONTPREP;
        }
    }

    static RETRY_MQ_OPS: LinuxBlkMqOps = LinuxBlkMqOps {
        queue_rq: Some(test_retry_queue_rq),
        commit_rqs: None,
        queue_rqs: None,
        get_budget: None,
        put_budget: None,
        set_rq_budget_token: None,
        get_rq_budget_token: None,
        timeout: None,
        poll: None,
        complete: None,
        init_hctx: None,
        exit_hctx: None,
        init_request: None,
        exit_request: None,
        cleanup_rq: Some(test_retry_cleanup_rq),
        busy: None,
        map_queues: None,
        show_rq: None,
    };

    #[test]
    fn linux_block_layout_prefixes_match_vendor_headers() {
        assert_eq!(offset_of!(LinuxBlkMqQueueData, rq), 0);
        assert_eq!(offset_of!(LinuxBlkMqQueueData, last), 8);
        assert_eq!(size_of::<LinuxBlkMqQueueData>(), 16);

        assert_eq!(offset_of!(LinuxBlkMqOps, queue_rq), 0);
        assert_eq!(offset_of!(LinuxBlkMqOps, commit_rqs), 8);
        assert_eq!(offset_of!(LinuxBlkMqOps, queue_rqs), 0x10);
        assert_eq!(offset_of!(LinuxBlkMqOps, get_budget), 0x18);
        assert_eq!(offset_of!(LinuxBlkMqOps, put_budget), 0x20);
        assert_eq!(offset_of!(LinuxBlkMqOps, set_rq_budget_token), 0x28);
        assert_eq!(offset_of!(LinuxBlkMqOps, get_rq_budget_token), 0x30);
        assert_eq!(offset_of!(LinuxBlkMqOps, timeout), 0x38);
        assert_eq!(
            offset_of!(LinuxBlkMqOps, poll),
            LINUX_BLK_MQ_OPS_POLL_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqOps, complete),
            LINUX_BLK_MQ_OPS_COMPLETE_OFFSET
        );
        assert_eq!(offset_of!(LinuxBlkMqOps, init_hctx), 0x50);
        assert_eq!(offset_of!(LinuxBlkMqOps, exit_hctx), 0x58);
        assert_eq!(offset_of!(LinuxBlkMqOps, init_request), 0x60);
        assert_eq!(offset_of!(LinuxBlkMqOps, exit_request), 0x68);
        assert_eq!(offset_of!(LinuxBlkMqOps, cleanup_rq), 0x70);
        assert_eq!(offset_of!(LinuxBlkMqOps, busy), 0x78);
        assert_eq!(
            offset_of!(LinuxBlkMqOps, map_queues),
            LINUX_BLK_MQ_OPS_MAP_QUEUES_OFFSET
        );
        assert_eq!(offset_of!(LinuxBlkMqOps, show_rq), 0x88);
        assert_eq!(size_of::<LinuxBlkMqOps>(), 0x90);

        assert_eq!(
            offset_of!(LinuxBlkMqHwCtx, queue),
            LINUX_BLK_MQ_HW_CTX_QUEUE_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqHwCtx, _pad_to_driver_data),
            LINUX_BLK_MQ_HW_CTX_FQ_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqHwCtx, driver_data),
            LINUX_BLK_MQ_HW_CTX_DRIVER_DATA_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqHwCtx, _pad_to_queue_num),
            LINUX_BLK_MQ_HW_CTX_CTX_MAP_OFFSET
        );
        assert_eq!(LINUX_BLK_MQ_HW_CTX_DISPATCH_FROM_OFFSET, 0xf0);
        assert_eq!(LINUX_BLK_MQ_HW_CTX_DISPATCH_BUSY_OFFSET, 0xf8);
        assert_eq!(LINUX_BLK_MQ_HW_CTX_TYPE_OFFSET, 0xfc);
        assert_eq!(LINUX_BLK_MQ_HW_CTX_NR_CTX_OFFSET, 0xfe);
        assert_eq!(LINUX_BLK_MQ_HW_CTX_CTXS_OFFSET, 0x100);
        assert_eq!(LINUX_BLK_MQ_HW_CTX_TAGS_OFFSET, 0x140);
        assert_eq!(LINUX_BLK_MQ_HW_CTX_SCHED_TAGS_OFFSET, 0x148);
        assert_eq!(LINUX_BLK_MQ_HW_CTX_NUMA_NODE_OFFSET, 0x150);
        assert_eq!(
            offset_of!(LinuxBlkMqHwCtx, queue_num),
            LINUX_BLK_MQ_HW_CTX_QUEUE_NUM_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqHwCtx, _pad_after_queue_num),
            LINUX_BLK_MQ_HW_CTX_NR_ACTIVE_OFFSET
        );
        assert_eq!(size_of::<LinuxBlkMqHwCtx>(), LINUX_BLK_MQ_HW_CTX_SIZE);
        assert_eq!(align_of::<LinuxBlkMqHwCtx>(), 64);

        assert_eq!(offset_of!(LinuxRequest, q), 0);
        assert_eq!(offset_of!(LinuxRequest, mq_hctx), 0x10);
        assert_eq!(offset_of!(LinuxRequest, cmd_flags), 0x18);
        assert_eq!(offset_of!(LinuxRequest, rq_flags), 0x1c);
        assert_eq!(offset_of!(LinuxRequest, tag), LINUX_REQUEST_TAG_OFFSET);
        assert_eq!(
            offset_of!(LinuxRequest, internal_tag),
            LINUX_REQUEST_INTERNAL_TAG_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, timeout),
            LINUX_REQUEST_TIMEOUT_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, rq_flags) + 1,
            RQF_SPECIAL_PAYLOAD_BYTE_OFFSET
        );
        assert_eq!(RQF_SPECIAL_PAYLOAD_BYTE_MASK, 0x10);
        assert_eq!(
            offset_of!(LinuxRequest, data_len),
            LINUX_REQUEST_DATA_LEN_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, sector),
            LINUX_REQUEST_SECTOR_OFFSET
        );
        assert_eq!(offset_of!(LinuxRequest, bio), LINUX_REQUEST_BIO_OFFSET);
        assert_eq!(
            offset_of!(LinuxRequest, biotail),
            LINUX_REQUEST_BIOTAIL_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, rq_next),
            LINUX_REQUEST_RQ_NEXT_OFFSET
        );
        assert_eq!(offset_of!(LinuxRequest, _pad_queuelist_prev), 0x50);
        assert_eq!(offset_of!(LinuxRequest, part), LINUX_REQUEST_PART_OFFSET);
        assert_eq!(
            offset_of!(LinuxRequest, _pad_to_nr_phys_segments),
            LINUX_REQUEST_ALLOC_TIME_NS_OFFSET
        );
        assert_eq!(LINUX_REQUEST_START_TIME_NS_OFFSET, 0x68);
        assert_eq!(LINUX_REQUEST_IO_START_TIME_NS_OFFSET, 0x70);
        assert_eq!(LINUX_REQUEST_STATS_SECTORS_OFFSET, 0x78);
        assert_eq!(
            offset_of!(LinuxRequest, nr_phys_segments),
            LINUX_REQUEST_NR_PHYS_SEGMENTS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, nr_integrity_segments),
            LINUX_REQUEST_NR_INTEGRITY_SEGMENTS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, _pad_to_state),
            LINUX_REQUEST_PHYS_GAP_BIT_OFFSET
        );
        assert_eq!(offset_of!(LinuxRequest, state), LINUX_REQUEST_STATE_OFFSET);
        assert_eq!(
            offset_of!(LinuxRequest, _pad_to_special_vec),
            LINUX_REQUEST_REF_OFFSET
        );
        assert_eq!(LINUX_REQUEST_DEADLINE_OFFSET, 0x88);
        assert_eq!(LINUX_REQUEST_HASH_OFFSET, 0x90);
        assert_eq!(
            offset_of!(LinuxRequest, special_vec_bv_page),
            LINUX_REQUEST_SPECIAL_VEC_OFFSET
        );
        assert_eq!(offset_of!(LinuxRequest, special_vec_bv_len), 0xa8);
        assert_eq!(offset_of!(LinuxRequest, special_vec_bv_offset), 0xac);
        assert_eq!(offset_of!(LinuxRequest, _pad_after_special_vec), 0xb0);
        assert_eq!(offset_of!(LinuxRequest, elv), LINUX_REQUEST_ELV_OFFSET);
        assert_eq!(offset_of!(LinuxRequest, flush), LINUX_REQUEST_FLUSH_OFFSET);
        assert_eq!(
            offset_of!(LinuxRequest, fifo_time),
            LINUX_REQUEST_FIFO_TIME_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, fifo_time) + size_of::<u64>(),
            LINUX_REQUEST_END_IO_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequest, end_io),
            LINUX_REQUEST_END_IO_OFFSET
        );
        assert_eq!(offset_of!(LinuxRequest, end_io_data), 0xf0);
        assert_eq!(size_of::<LinuxRequest>(), LINUX_REQUEST_SIZE);
        assert_eq!(LINUX_REQUEST_PDU_OFFSET, 0xf8);

        assert_eq!(
            offset_of!(LinuxBioVec, bv_page),
            LINUX_BIO_VEC_BV_PAGE_OFFSET
        );
        assert_eq!(offset_of!(LinuxBioVec, bv_len), LINUX_BIO_VEC_BV_LEN_OFFSET);
        assert_eq!(
            offset_of!(LinuxBioVec, bv_offset),
            LINUX_BIO_VEC_BV_OFFSET_OFFSET
        );
        assert_eq!(size_of::<LinuxBioVec>(), LINUX_BIO_VEC_SIZE);

        assert_eq!(
            offset_of!(LinuxBvecIter, bi_sector),
            LINUX_BVEC_ITER_BI_SECTOR_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBvecIter, bi_size),
            LINUX_BVEC_ITER_BI_SIZE_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBvecIter, bi_idx),
            LINUX_BVEC_ITER_BI_IDX_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBvecIter, bi_bvec_done),
            LINUX_BVEC_ITER_BI_BVEC_DONE_OFFSET
        );
        assert_eq!(size_of::<LinuxBvecIter>(), LINUX_BVEC_ITER_SIZE);
        assert_eq!(align_of::<LinuxBvecIter>(), 4);

        assert_eq!(offset_of!(LinuxBio, bi_next), LINUX_BIO_BI_NEXT_OFFSET);
        assert_eq!(offset_of!(LinuxBio, bi_bdev), LINUX_BIO_BI_BDEV_OFFSET);
        assert_eq!(offset_of!(LinuxBio, bi_opf), LINUX_BIO_BI_OPF_OFFSET);
        assert_eq!(offset_of!(LinuxBio, bi_flags), LINUX_BIO_BI_FLAGS_OFFSET);
        assert_eq!(offset_of!(LinuxBio, bi_ioprio), LINUX_BIO_BI_IOPRIO_OFFSET);
        assert_eq!(
            offset_of!(LinuxBio, bi_write_hint),
            LINUX_BIO_BI_WRITE_HINT_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBio, bi_write_stream),
            LINUX_BIO_BI_WRITE_STREAM_OFFSET
        );
        assert_eq!(offset_of!(LinuxBio, bi_status), LINUX_BIO_BI_STATUS_OFFSET);
        assert_eq!(
            offset_of!(LinuxBio, bi_bvec_gap_bit),
            LINUX_BIO_BI_BVEC_GAP_BIT_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBio, bi_remaining),
            LINUX_BIO_BI_REMAINING_OFFSET
        );
        assert_eq!(offset_of!(LinuxBio, bi_io_vec), LINUX_BIO_BI_IO_VEC_OFFSET);
        assert_eq!(offset_of!(LinuxBio, bi_iter), LINUX_BIO_BI_ITER_OFFSET);
        assert_eq!(offset_of!(LinuxBio, bi_end_io), LINUX_BIO_BI_END_IO_OFFSET);
        assert_eq!(
            offset_of!(LinuxBio, bi_private),
            LINUX_BIO_BI_PRIVATE_OFFSET
        );
        assert_eq!(offset_of!(LinuxBio, bi_blkg), LINUX_BIO_BI_BLKG_OFFSET);
        assert_eq!(
            offset_of!(LinuxBio, issue_time_ns),
            LINUX_BIO_ISSUE_TIME_NS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBio, bi_iocost_cost),
            LINUX_BIO_BI_IOCOST_COST_OFFSET
        );
        assert_eq!(offset_of!(LinuxBio, bi_vcnt), LINUX_BIO_BI_VCNT_OFFSET);
        assert_eq!(
            offset_of!(LinuxBio, bi_max_vecs),
            LINUX_BIO_BI_MAX_VECS_OFFSET
        );
        assert_eq!(offset_of!(LinuxBio, bi_cnt), LINUX_BIO_BI_CNT_OFFSET);
        assert_eq!(offset_of!(LinuxBio, bi_pool), LINUX_BIO_BI_POOL_OFFSET);
        assert_eq!(size_of::<LinuxBio>(), LINUX_BIO_SIZE);

        assert_eq!(offset_of!(LinuxBlkMqQueueMap, mq_map), 0);
        assert_eq!(offset_of!(LinuxBlkMqQueueMap, nr_queues), 8);
        assert_eq!(offset_of!(LinuxBlkMqQueueMap, queue_offset), 12);
        assert_eq!(size_of::<LinuxBlkMqQueueMap>(), 16);

        assert_eq!(offset_of!(LinuxBlkMqTagSet, ops), 0);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, map), 8);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, nr_maps), 0x38);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, nr_hw_queues), 0x3c);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, queue_depth), 0x40);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, reserved_tags), 0x44);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, cmd_size), 0x48);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, numa_node), 0x4c);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, timeout), 0x50);
        assert_eq!(offset_of!(LinuxBlkMqTagSet, flags), 0x54);
        assert_eq!(
            offset_of!(LinuxBlkMqTagSet, driver_data),
            LINUX_BLK_MQ_TAG_SET_DRIVER_DATA_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqTagSet, tags),
            LINUX_BLK_MQ_TAG_SET_TAGS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqTagSet, shared_tags),
            LINUX_BLK_MQ_TAG_SET_SHARED_TAGS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxBlkMqTagSet, _pad_after_shared_tags),
            LINUX_BLK_MQ_TAG_SET_TAG_LIST_LOCK_OFFSET
        );
        assert_eq!(
            LINUX_BLK_MQ_TAG_SET_TAG_LIST_LOCK_OFFSET + LINUX_MUTEX_SIZE,
            LINUX_BLK_MQ_TAG_SET_TAG_LIST_OFFSET
        );
        assert_eq!(LINUX_BLK_MQ_TAG_SET_SRCU_OFFSET, 0x98);
        assert_eq!(LINUX_BLK_MQ_TAG_SET_TAGS_SRCU_OFFSET, 0xa0);
        assert_eq!(LINUX_BLK_MQ_TAG_SET_UPDATE_NR_HWQ_LOCK_OFFSET, 0xc0);
        assert_eq!(size_of::<LinuxBlkMqTagSet>(), LINUX_BLK_MQ_TAG_SET_SIZE);

        assert_eq!(offset_of!(LinuxQueueLimits, features), 0);
        assert_eq!(offset_of!(LinuxQueueLimits, flags), 0x4);
        assert_eq!(offset_of!(LinuxQueueLimits, seg_boundary_mask), 0x8);
        assert_eq!(offset_of!(LinuxQueueLimits, virt_boundary_mask), 0x10);
        assert_eq!(offset_of!(LinuxQueueLimits, max_hw_sectors), 0x18);
        assert_eq!(offset_of!(LinuxQueueLimits, max_dev_sectors), 0x1c);
        assert_eq!(offset_of!(LinuxQueueLimits, chunk_sectors), 0x20);
        assert_eq!(offset_of!(LinuxQueueLimits, max_sectors), 0x24);
        assert_eq!(offset_of!(LinuxQueueLimits, max_user_sectors), 0x28);
        assert_eq!(offset_of!(LinuxQueueLimits, max_segment_size), 0x2c);
        assert_eq!(offset_of!(LinuxQueueLimits, max_fast_segment_size), 0x30);
        assert_eq!(offset_of!(LinuxQueueLimits, physical_block_size), 0x34);
        assert_eq!(
            offset_of!(LinuxQueueLimits, logical_block_size),
            LINUX_QUEUE_LIMITS_LOGICAL_BLOCK_SIZE_OFFSET
        );
        assert_eq!(offset_of!(LinuxQueueLimits, alignment_offset), 0x3c);
        assert_eq!(offset_of!(LinuxQueueLimits, io_min), 0x40);
        assert_eq!(offset_of!(LinuxQueueLimits, io_opt), 0x44);
        assert_eq!(offset_of!(LinuxQueueLimits, max_discard_sectors), 0x48);
        assert_eq!(offset_of!(LinuxQueueLimits, max_hw_discard_sectors), 0x4c);
        assert_eq!(offset_of!(LinuxQueueLimits, max_user_discard_sectors), 0x50);
        assert_eq!(offset_of!(LinuxQueueLimits, max_secure_erase_sectors), 0x54);
        assert_eq!(offset_of!(LinuxQueueLimits, max_write_zeroes_sectors), 0x58);
        assert_eq!(
            offset_of!(LinuxQueueLimits, max_wzeroes_unmap_sectors),
            0x5c
        );
        assert_eq!(
            offset_of!(LinuxQueueLimits, max_hw_wzeroes_unmap_sectors),
            0x60
        );
        assert_eq!(
            offset_of!(LinuxQueueLimits, max_user_wzeroes_unmap_sectors),
            0x64
        );
        assert_eq!(
            offset_of!(LinuxQueueLimits, max_hw_zone_append_sectors),
            0x68
        );
        assert_eq!(offset_of!(LinuxQueueLimits, max_zone_append_sectors), 0x6c);
        assert_eq!(offset_of!(LinuxQueueLimits, discard_granularity), 0x70);
        assert_eq!(offset_of!(LinuxQueueLimits, discard_alignment), 0x74);
        assert_eq!(offset_of!(LinuxQueueLimits, zone_write_granularity), 0x78);
        assert_eq!(offset_of!(LinuxQueueLimits, atomic_write_hw_max), 0x7c);
        assert_eq!(offset_of!(LinuxQueueLimits, atomic_write_max_sectors), 0x80);
        assert_eq!(offset_of!(LinuxQueueLimits, atomic_write_hw_boundary), 0x84);
        assert_eq!(
            offset_of!(LinuxQueueLimits, atomic_write_boundary_sectors),
            0x88
        );
        assert_eq!(offset_of!(LinuxQueueLimits, atomic_write_hw_unit_min), 0x8c);
        assert_eq!(offset_of!(LinuxQueueLimits, atomic_write_unit_min), 0x90);
        assert_eq!(offset_of!(LinuxQueueLimits, atomic_write_hw_unit_max), 0x94);
        assert_eq!(offset_of!(LinuxQueueLimits, atomic_write_unit_max), 0x98);
        assert_eq!(offset_of!(LinuxQueueLimits, max_segments), 0x9c);
        assert_eq!(offset_of!(LinuxQueueLimits, max_integrity_segments), 0x9e);
        assert_eq!(offset_of!(LinuxQueueLimits, max_discard_segments), 0xa0);
        assert_eq!(offset_of!(LinuxQueueLimits, max_write_streams), 0xa2);
        assert_eq!(offset_of!(LinuxQueueLimits, write_stream_granularity), 0xa4);
        assert_eq!(offset_of!(LinuxQueueLimits, max_open_zones), 0xa8);
        assert_eq!(offset_of!(LinuxQueueLimits, max_active_zones), 0xac);
        assert_eq!(offset_of!(LinuxQueueLimits, dma_alignment), 0xb0);
        assert_eq!(offset_of!(LinuxQueueLimits, dma_pad_mask), 0xb4);
        assert_eq!(offset_of!(LinuxQueueLimits, integrity), 0xb8);
        assert_eq!(size_of::<LinuxQueueLimits>(), 0xc0);
        assert_eq!(size_of::<LinuxAbiMutex>(), LINUX_MUTEX_SIZE);
        assert_eq!(align_of::<LinuxAbiMutex>(), 8);

        assert_eq!(offset_of!(LinuxBlockDevicePrefix, bd_start_sect), 0);
        assert_eq!(offset_of!(LinuxBlockDevicePrefix, bd_nr_sectors), 8);
        assert_eq!(offset_of!(LinuxBlockDevicePrefix, bd_disk), 16);
        assert_eq!(offset_of!(LinuxBlockDevicePrefix, bd_queue), 24);
        assert_eq!(
            offset_of!(LinuxBlockDevicePrefix, _pad_after_bd_queue),
            LINUX_BLOCK_DEVICE_BD_STATS_OFFSET
        );
        assert_eq!(LINUX_BLOCK_DEVICE_BD_STATS_OFFSET, 0x20);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_STAMP_OFFSET, 0x28);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_FLAGS_OFFSET, 0x30);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_DEV_OFFSET, 0x34);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_MAPPING_OFFSET, 0x38);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_OPENERS_OFFSET, 0x40);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_SIZE_LOCK_OFFSET, 0x44);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_CLAIMING_OFFSET, 0x48);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_HOLDER_OFFSET, 0x50);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_HOLDER_OPS_OFFSET, 0x58);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_HOLDER_LOCK_OFFSET, 0x60);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_HOLDERS_OFFSET, 0x78);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_HOLDER_DIR_OFFSET, 0x80);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_FSFREEZE_COUNT_OFFSET, 0x88);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_FSFREEZE_MUTEX_OFFSET, 0x90);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_META_INFO_OFFSET, 0xa8);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_WRITERS_OFFSET, 0xb0);
        assert_eq!(LINUX_BLOCK_DEVICE_BD_DEVICE_OFFSET, 0xc0);
        assert_eq!(LINUX_ATOMIC_T_SIZE, 0x4);
        assert_eq!(LINUX_DEV_T_SIZE, 0x4);
        assert_eq!(LINUX_SPINLOCK_T_SIZE, 0x4);
        assert_eq!(LINUX_STRUCT_DEVICE_SIZE, 0x2f8);
        assert_eq!(
            LINUX_BLOCK_DEVICE_BD_FLAGS_OFFSET + LINUX_ATOMIC_T_SIZE,
            LINUX_BLOCK_DEVICE_BD_DEV_OFFSET
        );
        assert_eq!(
            LINUX_BLOCK_DEVICE_BD_DEV_OFFSET + LINUX_DEV_T_SIZE,
            LINUX_BLOCK_DEVICE_BD_MAPPING_OFFSET
        );
        assert_eq!(
            LINUX_BLOCK_DEVICE_BD_OPENERS_OFFSET + LINUX_ATOMIC_T_SIZE,
            LINUX_BLOCK_DEVICE_BD_SIZE_LOCK_OFFSET
        );
        assert_eq!(
            LINUX_BLOCK_DEVICE_BD_DEVICE_OFFSET + LINUX_STRUCT_DEVICE_SIZE,
            LINUX_BLOCK_DEVICE_SIZE
        );
        assert_eq!(size_of::<LinuxBlockDevicePrefix>(), LINUX_BLOCK_DEVICE_SIZE);

        assert_eq!(offset_of!(LinuxRequestQueue, queuedata), 0);
        assert_eq!(offset_of!(LinuxRequestQueue, elevator), 0x8);
        assert_eq!(offset_of!(LinuxRequestQueue, mq_ops), 0x10);
        assert_eq!(offset_of!(LinuxRequestQueue, queue_ctx), 0x18);
        assert_eq!(offset_of!(LinuxRequestQueue, queue_flags), 0x20);
        assert_eq!(offset_of!(LinuxRequestQueue, rq_timeout), 0x28);
        assert_eq!(offset_of!(LinuxRequestQueue, queue_depth), 0x2c);
        assert_eq!(offset_of!(LinuxRequestQueue, refs), 0x30);
        assert_eq!(offset_of!(LinuxRequestQueue, nr_hw_queues), 0x34);
        assert_eq!(offset_of!(LinuxRequestQueue, queue_hw_ctx), 0x38);
        assert_eq!(
            offset_of!(LinuxRequestQueue, disk),
            LINUX_REQUEST_QUEUE_DISK_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, mq_kobj),
            LINUX_REQUEST_QUEUE_MQ_KOBJ_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, limits),
            LINUX_REQUEST_QUEUE_LIMITS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, pm_only),
            LINUX_REQUEST_QUEUE_PM_ONLY_OFFSET
        );
        assert_eq!(offset_of!(LinuxRequestQueue, stats), 0x140);
        assert_eq!(offset_of!(LinuxRequestQueue, rq_qos), 0x148);
        assert_eq!(
            offset_of!(LinuxRequestQueue, rq_qos_mutex),
            LINUX_REQUEST_QUEUE_RQ_QOS_MUTEX_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, id),
            LINUX_REQUEST_QUEUE_ID_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, nr_requests),
            LINUX_REQUEST_QUEUE_NR_REQUESTS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, async_depth),
            LINUX_REQUEST_QUEUE_ASYNC_DEPTH_OFFSET
        );
        assert_eq!(offset_of!(LinuxRequestQueue, _pad_after_async_depth), 0x174);
        assert_eq!(LINUX_REQUEST_QUEUE_TIMEOUT_OFFSET, 0x178);
        assert_eq!(LINUX_REQUEST_QUEUE_TIMEOUT_WORK_OFFSET, 0x1a0);
        assert_eq!(
            LINUX_REQUEST_QUEUE_NR_ACTIVE_REQUESTS_SHARED_TAGS_OFFSET,
            0x1c0
        );
        assert_eq!(LINUX_REQUEST_QUEUE_SCHED_SHARED_TAGS_OFFSET, 0x1c8);
        assert_eq!(LINUX_REQUEST_QUEUE_ICQ_LIST_OFFSET, 0x1d0);
        assert_eq!(LINUX_REQUEST_QUEUE_NODE_OFFSET, 0x218);
        assert_eq!(LINUX_REQUEST_QUEUE_REQUEUE_LOCK_OFFSET, 0x21c);
        assert_eq!(LINUX_REQUEST_QUEUE_REQUEUE_LIST_OFFSET, 0x220);
        assert_eq!(LINUX_REQUEST_QUEUE_REQUEUE_WORK_OFFSET, 0x230);
        assert_eq!(LINUX_REQUEST_QUEUE_FQ_OFFSET, 0x288);
        assert_eq!(LINUX_REQUEST_QUEUE_FLUSH_LIST_OFFSET, 0x290);
        assert_eq!(LINUX_REQUEST_QUEUE_ELEVATOR_LOCK_OFFSET, 0x2a0);
        assert_eq!(LINUX_REQUEST_QUEUE_SYSFS_LOCK_OFFSET, 0x2b8);
        assert_eq!(
            offset_of!(LinuxRequestQueue, limits_lock),
            LINUX_REQUEST_QUEUE_LIMITS_LOCK_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, _pad_after_limits_lock),
            LINUX_REQUEST_QUEUE_UNUSED_HCTX_LIST_OFFSET
        );
        assert_eq!(LINUX_REQUEST_QUEUE_UNUSED_HCTX_LOCK_OFFSET, 0x2f8);
        assert_eq!(LINUX_REQUEST_QUEUE_MQ_FREEZE_DEPTH_OFFSET, 0x2fc);
        assert_eq!(LINUX_REQUEST_QUEUE_RCU_HEAD_OFFSET, 0x300);
        assert_eq!(LINUX_REQUEST_QUEUE_MQ_FREEZE_WQ_OFFSET, 0x310);
        assert_eq!(LINUX_REQUEST_QUEUE_MQ_FREEZE_LOCK_OFFSET, 0x328);
        assert_eq!(
            offset_of!(LinuxRequestQueue, tag_set),
            LINUX_REQUEST_QUEUE_TAG_SET_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, _pad_after_tag_set),
            LINUX_REQUEST_QUEUE_TAG_SET_LIST_OFFSET
        );
        assert_eq!(LINUX_REQUEST_QUEUE_DEBUGFS_DIR_OFFSET, 0x358);
        assert_eq!(LINUX_REQUEST_QUEUE_SCHED_DEBUGFS_DIR_OFFSET, 0x360);
        assert_eq!(LINUX_REQUEST_QUEUE_RQOS_DEBUGFS_DIR_OFFSET, 0x368);
        assert_eq!(LINUX_REQUEST_QUEUE_DEBUGFS_MUTEX_OFFSET, 0x370);
        assert_eq!(
            offset_of!(LinuxRequestQueue, hctx_table_storage),
            LINUX_REQUEST_QUEUE_SIZE
        );
        assert_eq!(
            size_of::<LinuxRequestQueue>(),
            LINUX_REQUEST_QUEUE_SIZE + core::mem::size_of::<*mut LinuxBlkMqHwCtx>()
        );
        assert_eq!(
            offset_of!(LinuxRequestQueue, limits)
                + offset_of!(LinuxQueueLimits, logical_block_size),
            0xa8
        );

        assert_eq!(offset_of!(LinuxGendisk, major), 0);
        assert_eq!(offset_of!(LinuxGendisk, first_minor), 4);
        assert_eq!(offset_of!(LinuxGendisk, minors), 8);
        assert_eq!(offset_of!(LinuxGendisk, disk_name), 12);
        assert_eq!(offset_of!(LinuxGendisk, events), 44);
        assert_eq!(offset_of!(LinuxGendisk, event_flags), 46);
        assert_eq!(offset_of!(LinuxGendisk, part_tbl), 48);
        assert_eq!(offset_of!(LinuxGendisk, part0), 64);
        assert_eq!(offset_of!(LinuxGendisk, fops), 72);
        assert_eq!(offset_of!(LinuxGendisk, queue), 80);
        assert_eq!(offset_of!(LinuxGendisk, private_data), 88);
        assert_eq!(
            offset_of!(LinuxGendisk, bio_split),
            LINUX_GENDISK_BIO_SPLIT_OFFSET
        );
        assert_eq!(LINUX_BIO_SET_SIZE, 0xf8);
        assert_eq!(offset_of!(LinuxGendisk, flags), LINUX_GENDISK_FLAGS_OFFSET);
        assert_eq!(offset_of!(LinuxGendisk, state), LINUX_GENDISK_STATE_OFFSET);
        assert_eq!(size_of::<LinuxGendisk>(), LINUX_GENDISK_SIZE);
    }

    #[test]
    fn linux_block_core_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("__register_blkdev"),
            Some(__register_blkdev as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("blk_mq_alloc_tag_set"),
            Some(blk_mq_alloc_tag_set as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__blk_mq_alloc_disk"),
            Some(__blk_mq_alloc_disk as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_add_disk"),
            Some(device_add_disk as usize)
        );
        for (name, addr) in [
            ("blk_mq_alloc_request", blk_mq_alloc_request as usize),
            ("blk_mq_complete_request", blk_mq_complete_request as usize),
            ("blk_mq_map_queues", blk_mq_map_queues as usize),
            ("__blk_rq_map_sg", __blk_rq_map_sg as usize),
            ("blk_rq_map_kern", blk_rq_map_kern as usize),
            ("blk_execute_rq", blk_execute_rq as usize),
            (
                "queue_limits_commit_update_frozen",
                queue_limits_commit_update_frozen as usize,
            ),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
        }
    }

    #[test]
    fn linux_block_core_fixed_major_registration_matches_linux_return_value() {
        unsafe {
            let name = b"sd\0";

            assert_eq!(
                __register_blkdev(231, name.as_ptr().cast::<c_char>(), core::ptr::null_mut()),
                0
            );
            assert_eq!(
                __register_blkdev(232, name.as_ptr().cast::<c_char>(), core::ptr::null_mut()),
                0
            );
            assert_eq!(
                __register_blkdev(
                    231,
                    b"sddup\0".as_ptr().cast::<c_char>(),
                    core::ptr::null_mut()
                ),
                -EBUSY
            );
            assert!(
                registered_linux_block_majors()
                    .iter()
                    .any(|registered| registered.major == 231 && registered.name == "sd")
            );
            assert!(
                registered_linux_block_majors()
                    .iter()
                    .any(|registered| registered.major == 232 && registered.name == "sd")
            );

            unregister_blkdev(231, name.as_ptr().cast::<c_char>());
            unregister_blkdev(232, name.as_ptr().cast::<c_char>());
        }
    }

    #[test]
    fn linux_block_core_tracks_module_allocated_major_tag_set_and_disk() {
        unsafe {
            let name = b"virtblk\0";
            INIT_HCTX_CALLS.store(0, Ordering::Release);
            INIT_HCTX_HCTX.store(0, Ordering::Release);
            INIT_HCTX_DATA.store(0, Ordering::Release);
            INIT_HCTX_INDEX.store(usize::MAX, Ordering::Release);
            EXIT_HCTX_CALLS.store(0, Ordering::Release);
            EXIT_HCTX_HCTX.store(0, Ordering::Release);
            EXIT_HCTX_INDEX.store(usize::MAX, Ordering::Release);

            let major = __register_blkdev(0, name.as_ptr().cast::<c_char>(), core::ptr::null_mut());
            assert!(major >= 240);
            assert!(
                registered_linux_block_majors()
                    .iter()
                    .any(|registered| registered.major == major as u32
                        && registered.name == "virtblk")
            );

            let mut set = core::mem::zeroed::<LinuxBlkMqTagSet>();
            let ops = &TEST_MQ_OPS as *const LinuxBlkMqOps as *const c_void;
            let queuedata = 0xfeedusize as *mut c_void;
            set.ops = ops;
            set.nr_hw_queues = 2;
            set.queue_depth = 64;
            set.cmd_size = 0x48;
            set.driver_data = queuedata;
            assert_eq!(blk_mq_alloc_tag_set(&mut set), 0);
            assert_eq!(set.nr_maps, 1);
            assert_eq!(set.map[0].nr_queues, 2);
            blk_mq_map_queues(&mut set.map[0]);
            assert_eq!(set.map[0].nr_queues, 2);
            assert_eq!(set.map[0].queue_offset, 0);
            assert_eq!(blk_mq_num_possible_queues(0), 1);
            assert_eq!(blk_mq_num_possible_queues(8), 1);

            let mut limits = core::mem::zeroed::<LinuxQueueLimits>();
            limits.features = BLK_FEAT_ROTATIONAL;
            limits.logical_block_size = 512;
            let disk = __blk_mq_alloc_disk(&mut set, &limits, queuedata, core::ptr::null_mut());
            assert!(!disk.is_null());
            assert_eq!((*disk).private_data, queuedata);
            assert!(!(*disk).part0.is_null());
            assert_eq!((*(*disk).part0).bd_disk, disk);
            assert_eq!((*(*disk).part0).bd_queue, (*disk).queue);
            assert!(!(*disk).queue.is_null());
            assert_eq!((*(*disk).queue).queuedata, queuedata);
            assert_eq!((*(*disk).queue).mq_ops, ops);
            assert!(!(*(*disk).queue).queue_hw_ctx.is_null());
            let hctx_table = (*(*disk).queue).queue_hw_ctx as *mut *mut LinuxBlkMqHwCtx;
            assert!(!(*hctx_table).is_null());
            assert_eq!((**hctx_table).queue, (*disk).queue);
            assert_eq!((**hctx_table).driver_data, queuedata);
            assert_eq!((**hctx_table).queue_num, 0);
            assert_eq!(INIT_HCTX_CALLS.load(Ordering::Acquire), 1);
            assert_eq!(INIT_HCTX_HCTX.load(Ordering::Acquire), *hctx_table as usize);
            assert_eq!(INIT_HCTX_DATA.load(Ordering::Acquire), queuedata as usize);
            assert_eq!(INIT_HCTX_INDEX.load(Ordering::Acquire), 0);
            assert_eq!((*(*disk).queue).limits.logical_block_size, 512);
            limits.logical_block_size = 4096;
            assert_eq!(queue_limits_commit_update_frozen((*disk).queue, &limits), 0);
            assert_eq!((*(*disk).queue).limits.logical_block_size, 4096);
            assert_eq!(blk_status_to_errno(0), 0);
            assert_eq!(blk_status_to_errno(1), -EIO);
            assert!(!blk_mq_complete_request_remote(core::ptr::null_mut()));

            let mem = MemBlockDevice::new("linux-rq-bio", 1 << 20);
            let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
            let bio = bio_alloc(bdev, BioOp(BIO_OP_WRITE), 8);
            bio.add_vec(BioVec::new(alloc::vec![0x11; 512]));
            bio.add_vec(BioVec::new(alloc::vec![0x22; 1024]));

            let rq = linux_prepare_request_from_bio((*disk).queue, &bio).unwrap();
            assert_eq!((*rq).q, (*disk).queue);
            assert_eq!((*rq).mq_hctx, *hctx_table);
            assert_eq!((*rq).cmd_flags, 1);
            assert_eq!((*rq).data_len, 1536);
            assert_eq!((*rq).sector, 8);
            assert_eq!((*rq).nr_phys_segments, 2);
            assert!(!(*rq).bio.is_null());
            assert_eq!((*rq).biotail, (*rq).bio);
            let linux_bio = (*rq).bio.cast::<LinuxBio>();
            assert_eq!((*linux_bio).bi_next, core::ptr::null_mut());
            assert_eq!((*linux_bio).bi_bdev, (*disk).part0.cast::<c_void>());
            assert_eq!((*linux_bio).bi_opf, BIO_OP_WRITE as u32);
            assert_eq!((*linux_bio).bi_ioprio, 0);
            assert_eq!((*linux_bio).bi_write_hint, 0);
            assert_eq!((*linux_bio).bi_vcnt, 2);
            assert_eq!((*linux_bio).bi_max_vecs, 2);
            assert_eq!((*(*linux_bio).bi_io_vec).bv_len, 512);
            assert_eq!((*(*linux_bio).bi_io_vec.add(1)).bv_len, 1024);
            assert_eq!(
                core::ptr::addr_of!((*linux_bio).bi_iter.bi_sector).read_unaligned(),
                8
            );
            assert_eq!(
                core::ptr::addr_of!((*linux_bio).bi_iter.bi_size).read_unaligned(),
                1536
            );
            assert_eq!((*rq).state, 0);
            assert_eq!(
                LINUX_REQUESTS
                    .lock()
                    .iter()
                    .find(|allocation| allocation.ptr == rq as usize)
                    .map(|allocation| allocation.len),
                Some(LINUX_REQUEST_SIZE + set.cmd_size as usize)
            );
            let mut sg: [crate::lib::scatterlist::LinuxScatterList; 2] = core::mem::zeroed();
            let mut last_sg = core::ptr::null_mut();
            assert_eq!(__blk_rq_map_sg(rq, sg.as_mut_ptr(), &mut last_sg), 2);
            assert_eq!(last_sg, sg.as_mut_ptr().add(1));
            assert_eq!(sg[0].length, 512);
            assert_eq!(sg[1].length, 1024);
            assert_eq!(
                sg[0].page_link & crate::lib::scatterlist::SG_END,
                0,
                "only the final mapped SG entry should carry SG_END"
            );
            assert_eq!(
                sg[1].page_link & crate::lib::scatterlist::SG_END,
                crate::lib::scatterlist::SG_END
            );
            QUEUE_RQ_CALLS.store(0, Ordering::Release);
            assert_eq!(linux_queue_request(rq, true).unwrap(), BLK_STS_OK);
            assert_eq!(QUEUE_RQ_CALLS.load(Ordering::Acquire), 1);
            assert_eq!(QUEUE_RQ_REQUEST.load(Ordering::Acquire), rq as usize);
            assert_eq!(
                QUEUE_RQ_HCTX.load(Ordering::Acquire),
                (*rq).mq_hctx as usize
            );
            blk_mq_free_request(rq);
            assert!(
                LINUX_REQUESTS
                    .lock()
                    .iter()
                    .all(|allocation| allocation.ptr != rq as usize)
            );

            let original_cmd_size = set.cmd_size;
            let scsi_ahci_cmd_size =
                crate::linux_driver_abi::storage_core::LINUX_SCSI_AHCI_CMD_SIZE as u32;
            set.cmd_size = scsi_ahci_cmd_size;
            let scsi_rq = blk_mq_alloc_request((*disk).queue, 0, 0).cast::<LinuxRequest>();
            assert!(!scsi_rq.is_null());
            assert_eq!(
                LINUX_REQUESTS
                    .lock()
                    .iter()
                    .find(|allocation| allocation.ptr == scsi_rq as usize)
                    .map(|allocation| allocation.len),
                Some(LINUX_REQUEST_SIZE + scsi_ahci_cmd_size as usize)
            );
            blk_mq_free_request(scsi_rq);
            set.cmd_size = original_cmd_size;

            let pt_rq = blk_mq_alloc_request((*disk).queue, 34, 0).cast::<LinuxRequest>();
            assert!(!pt_rq.is_null());
            let mut inquiry = [0u8; 36];
            assert_eq!(
                blk_rq_map_kern(
                    pt_rq,
                    inquiry.as_mut_ptr().cast::<c_void>(),
                    inquiry.len() as u32,
                    0
                ),
                0
            );
            assert_eq!((*pt_rq).data_len, inquiry.len() as u32);
            assert_eq!((*pt_rq).nr_phys_segments, 1);
            assert_eq!((*pt_rq).rq_flags & RQF_SPECIAL_PAYLOAD, RQF_SPECIAL_PAYLOAD);
            assert!(!(*pt_rq).bio.is_null());
            assert_eq!((*pt_rq).biotail, (*pt_rq).bio);
            let pt_linux_bio = (*pt_rq).bio.cast::<LinuxBio>();
            assert_eq!((*pt_linux_bio).bi_opf, 34);
            assert_eq!((*pt_linux_bio).bi_vcnt, 1);
            assert_eq!((*(*pt_linux_bio).bi_io_vec).bv_len, inquiry.len() as u32);
            assert_eq!(
                core::ptr::addr_of!((*pt_linux_bio).bi_iter.bi_size).read_unaligned(),
                inquiry.len() as u32
            );
            assert_eq!(
                (*pt_rq).special_vec_bv_page,
                inquiry.as_mut_ptr().cast::<c_void>()
            );
            let mut passthrough_sg: [crate::lib::scatterlist::LinuxScatterList; 1] =
                core::mem::zeroed();
            let mut passthrough_last_sg = core::ptr::null_mut();
            assert_eq!(
                __blk_rq_map_sg(pt_rq, passthrough_sg.as_mut_ptr(), &mut passthrough_last_sg),
                1
            );
            assert_eq!(passthrough_last_sg, passthrough_sg.as_mut_ptr());
            assert_eq!(passthrough_sg[0].length, inquiry.len() as u32);
            assert_eq!(
                passthrough_sg[0].page_link & !crate::lib::scatterlist::SG_PAGE_LINK_MASK,
                inquiry.as_mut_ptr() as usize & !crate::lib::scatterlist::SG_PAGE_LINK_MASK
            );
            QUEUE_RQ_CALLS.store(0, Ordering::Release);
            assert_eq!(blk_execute_rq(pt_rq, true), BLK_STS_OK);
            assert_eq!(QUEUE_RQ_CALLS.load(Ordering::Acquire), 1);
            blk_mq_free_request(pt_rq);

            (&mut (*disk).disk_name)[..6].copy_from_slice(b"vdabi0");
            (*disk).major = major;
            (*disk).minors = 16;
            assert!(set_capacity_and_notify(disk, 4096));
            assert_eq!((*(*disk).part0).bd_nr_sectors, 4096);
            assert_eq!(
                device_add_disk(core::ptr::null_mut(), disk, core::ptr::null()),
                0
            );
            assert!(linux_disk_registered(disk));
            let linux_bdev = crate::block::block_device::lookup_block_device("/dev/vdabi0")
                .expect("device_add_disk should publish Linux gendisk through block-core glue");
            assert_eq!(linux_bdev.ops.name, "linux_gendisk");
            assert_eq!(linux_bdev.capacity_sectors(), 4096);
            assert_eq!((linux_bdev.ops.block_size)(&linux_bdev), 4096);
            assert!(
                crate::block::gendisk::lookup_gendisk("vdabi0").is_some(),
                "device_add_disk should publish the Linux gendisk in the generic gendisk registry"
            );

            QUEUE_RQ_CALLS.store(0, Ordering::Release);
            let linux_bio = bio_alloc(linux_bdev, BioOp(BIO_OP_WRITE), 12);
            linux_bio.add_vec(BioVec::new(alloc::vec![0x33; 512]));
            submit_bio(linux_bio).expect("Linux gendisk block-core queue_rq");
            assert_eq!(QUEUE_RQ_CALLS.load(Ordering::Acquire), 1);

            set_disk_ro(disk, true);
            assert_ne!((*disk).state & (1usize << GD_READ_ONLY), 0);
            assert_eq!(linux_gendisk_capacity_sectors(disk), 4096);
            assert_eq!((*(*disk).part0).bd_nr_sectors, 4096);
            assert!(!set_capacity_and_notify(disk, 4096));

            del_gendisk(disk);
            assert!(!linux_disk_registered(disk));
            assert!(crate::block::block_device::lookup_block_device("/dev/vdabi0").is_none());
            assert!(crate::block::gendisk::lookup_gendisk("vdabi0").is_none());
            let hctx = *hctx_table;
            put_disk(disk);
            assert_eq!(EXIT_HCTX_CALLS.load(Ordering::Acquire), 1);
            assert_eq!(EXIT_HCTX_HCTX.load(Ordering::Acquire), hctx as usize);
            assert_eq!(EXIT_HCTX_INDEX.load(Ordering::Acquire), 0);
            unregister_blkdev(major as u32, name.as_ptr().cast::<c_char>());
        }
    }

    #[test]
    fn linux_queue_submit_bio_polls_driver_mq_ops_until_completion() {
        unsafe {
            POLL_QUEUE_RQ_CALLS.store(0, Ordering::Release);
            POLL_CALLS.store(0, Ordering::Release);
            POLL_REQUEST.store(0, Ordering::Release);

            let mut set = core::mem::zeroed::<LinuxBlkMqTagSet>();
            set.ops = (&POLL_MQ_OPS as *const LinuxBlkMqOps).cast::<c_void>();
            set.nr_hw_queues = 1;
            set.queue_depth = 16;
            set.cmd_size = 0x20;
            assert_eq!(blk_mq_alloc_tag_set(&mut set), 0);

            let mut limits = core::mem::zeroed::<LinuxQueueLimits>();
            limits.logical_block_size = 512;
            let disk = __blk_mq_alloc_disk(
                &mut set,
                &limits,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
            assert!(!disk.is_null());

            let mem = MemBlockDevice::new("linux-rq-poll-bio", 1 << 20);
            let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
            let bio = bio_alloc(bdev, BioOp(BIO_OP_WRITE), 4);
            bio.add_vec(BioVec::new(alloc::vec![0x44; 512]));

            linux_queue_submit_bio((*disk).queue, &bio)
                .expect("blk-mq sync bridge should call the Linux driver poll callback");
            assert_eq!(POLL_QUEUE_RQ_CALLS.load(Ordering::Acquire), 1);
            assert!(
                POLL_CALLS.load(Ordering::Acquire) >= 1,
                "driver-provided mq_ops->poll should be reached before timeout"
            );
            assert!(
                LINUX_REQUESTS
                    .lock()
                    .iter()
                    .all(|allocation| allocation.ptr != POLL_REQUEST.load(Ordering::Acquire))
            );

            put_disk(disk);
            blk_mq_free_tag_set(&mut set);
        }
    }

    #[test]
    fn linux_queue_submit_bio_retries_with_clean_request_after_resource_status() {
        unsafe {
            RETRY_QUEUE_RQ_CALLS.store(0, Ordering::Release);
            RETRY_CLEANUP_CALLS.store(0, Ordering::Release);
            RETRY_FIRST_REQUEST.store(0, Ordering::Release);
            RETRY_SECOND_REQUEST.store(0, Ordering::Release);

            let mut set = core::mem::zeroed::<LinuxBlkMqTagSet>();
            set.ops = (&RETRY_MQ_OPS as *const LinuxBlkMqOps).cast::<c_void>();
            set.nr_hw_queues = 1;
            set.queue_depth = 16;
            assert_eq!(blk_mq_alloc_tag_set(&mut set), 0);

            let mut limits = core::mem::zeroed::<LinuxQueueLimits>();
            limits.logical_block_size = 512;
            let disk = __blk_mq_alloc_disk(
                &mut set,
                &limits,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
            assert!(!disk.is_null());

            let mem = MemBlockDevice::new("linux-rq-retry-bio", 1 << 20);
            let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
            let bio = bio_alloc(bdev, BioOp(BIO_OP_WRITE), 4);
            bio.add_vec(BioVec::new(alloc::vec![0x55; 512]));

            linux_queue_submit_bio((*disk).queue, &bio)
                .expect("retryable queue_rq status should be retried with a clean request");
            let first = RETRY_FIRST_REQUEST.load(Ordering::Acquire);
            let second = RETRY_SECOND_REQUEST.load(Ordering::Acquire);
            assert_ne!(first, 0);
            assert_ne!(second, 0);
            assert_eq!(RETRY_QUEUE_RQ_CALLS.load(Ordering::Acquire), 2);
            assert_eq!(RETRY_CLEANUP_CALLS.load(Ordering::Acquire), 1);
            assert!(
                LINUX_REQUESTS
                    .lock()
                    .iter()
                    .all(|allocation| allocation.ptr != first && allocation.ptr != second)
            );

            put_disk(disk);
            blk_mq_free_tag_set(&mut set);
        }
    }
}
