//! linux-parity: partial
//! linux-source: vendor/linux/block
//! Block layer — M43–M44.
//!
//! Mirrors `vendor/linux/block/`.  M43 lands the core:
//!   * `bio`           — `struct Bio`, `BioVec`, op codes, segment iteration
//!   * `request`       — `struct Request`, FIFO chain
//!   * `blk_mq`        — software / hardware queue, dispatch
//!   * `sched`         — I/O scheduler vtable + `mq-deadline`
//!   * `gendisk`       — disk descriptor + sysfs registry
//!   * `block_device`  — bdev anchor (no pseudo-fs at this scale)
//!   * `mem`           — `MemBlockDevice` (RAM backing)
//!   * `ioctl`         — BLK* ioctl numbers
//!
//! M44 adds: `partitions/{mbr,gpt}`, `loop_dev`, and drivers/block glue.

pub mod badblocks;
pub mod bcache;
pub mod bdev;
pub mod bfq_cgroup;
pub mod bfq_iosched;
pub mod bfq_wf2q;
pub mod bio;
pub mod bio_integrity;
pub mod bio_integrity_auto;
pub mod bio_integrity_fs;
pub mod blk_cgroup;
pub mod blk_cgroup_fc_appid;
pub mod blk_cgroup_rwstat;
pub mod blk_mq;
pub mod blk_settings;
pub mod block_device;
pub mod dm;
pub mod gendisk;
pub mod ioctl;
pub mod mem;
pub mod request;
pub mod sched;

pub mod loop_dev;
pub mod partitions;

pub use bio::{
    BIO_OP_DISCARD, BIO_OP_FLUSH, BIO_OP_READ, BIO_OP_WRITE, Bio, BioOp, BioRef, BioVec, bio_alloc,
    bio_endio, submit_bio,
};
pub use block_device::{
    BlockDevice, BlockDeviceOps, BlockDeviceRef, block_device_ioctl, lookup_block_device,
    register_block_device, registered_block_devices,
};
pub use gendisk::{GenDisk, register_gendisk};
pub use mem::MemBlockDevice;

/// Initialize the block subsystem.
pub fn init() {
    block_device::init_registry();
    gendisk::init_registry();
}
