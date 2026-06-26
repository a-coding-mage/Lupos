//! linux-parity: partial
//! linux-source: vendor/linux/block
//! linux-source: vendor/linux/drivers/block/brd.c
//! `MemBlockDevice` — RAM-backed block device.
//!
//! Minimal RAM-backed `BlockDeviceOps` (read/write/flush/discard) used as a
//! reference target and loop-device backing. Linux's full equivalent is the
//! `brd` driver (drivers/block/brd.c). Remaining work vs Linux for `complete`:
//! radix-tree page store, partitioning, configurable sector size, and discard
//! that frees backing pages.
//!
//! Acts as the M43 reference target until VirtIO-blk lands in M57.  Used
//! by the loop device (M44) to back a "file" that's really a Vec<u8>.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use spin::Mutex;

use super::bio::{BIO_OP_DISCARD, BIO_OP_FLUSH, BIO_OP_READ, BIO_OP_WRITE, BioRef};
use super::block_device::{BlockDeviceOps, BlockDeviceRef};
use crate::include::uapi::errno::{EIO, ENOSYS};

pub struct MemBlockDevice {
    pub name: alloc::string::String,
    pub data: Mutex<Vec<u8>>,
}

impl MemBlockDevice {
    pub fn new(name: &str, size_bytes: usize) -> Arc<Self> {
        let mut v = Vec::with_capacity(size_bytes);
        v.resize(size_bytes, 0);
        Arc::new(Self {
            name: alloc::string::String::from(name),
            data: Mutex::new(v),
        })
    }
}

fn mem_submit_bio(bdev: &BlockDeviceRef, bio: &BioRef) -> Result<(), i32> {
    let backing = bdev.backing.lock().clone().ok_or(EIO)?;
    let mem = backing.downcast::<MemBlockDevice>().map_err(|_| EIO)?;
    let mut data = mem.data.lock();
    let mut byte_off = (bio.sector as usize) * 512;
    let op = bio.op.0;
    let vecs = bio.vecs.lock();
    for v in vecs.iter() {
        let mut buf = v.data.lock();
        let end = byte_off + v.len;
        if end > data.len() {
            return Err(EIO);
        }
        match op {
            BIO_OP_READ => {
                buf[v.off..v.off + v.len].copy_from_slice(&data[byte_off..end]);
            }
            BIO_OP_WRITE => {
                data[byte_off..end].copy_from_slice(&buf[v.off..v.off + v.len]);
            }
            BIO_OP_FLUSH | BIO_OP_DISCARD => {
                // No-op for RAM backing.
            }
            _ => return Err(ENOSYS),
        }
        byte_off += v.len;
    }
    bio.status.store(0, Ordering::Release);
    Ok(())
}

fn mem_get_capacity(bdev: &BlockDeviceRef) -> u64 {
    let backing = match bdev.backing.lock().clone() {
        Some(b) => b,
        None => return 0,
    };
    let mem = match backing.downcast::<MemBlockDevice>() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    (mem.data.lock().len() / 512) as u64
}

fn mem_block_size(_bdev: &BlockDeviceRef) -> u32 {
    512
}

pub static MEM_OPS: BlockDeviceOps = BlockDeviceOps {
    name: "mem",
    submit_bio: mem_submit_bio,
    get_capacity: mem_get_capacity,
    block_size: mem_block_size,
    ioctl: None,
};

pub fn mem_block_device_ops() -> &'static BlockDeviceOps {
    &MEM_OPS
}
