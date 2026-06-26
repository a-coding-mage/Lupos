//! linux-parity: partial
//! linux-source: vendor/linux/block
//! Loop device — wraps a byte buffer (or any `MemBlockDevice`) as a block dev.
//!
//! Mirrors `vendor/linux/drivers/block/loop.c`.  M44 lands the in-kernel
//! object + `loop_configure_inplace` helper; full `LOOP_CONFIGURE` ioctl
//! marshalling waits on the userspace ABI surface (M67).

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENOENT};

use super::block_device::{BlockDevice, BlockDeviceRef};
use super::mem::{MemBlockDevice, mem_block_device_ops};

pub struct LoopDevice {
    pub minor: u32,
    pub bdev: Mutex<Option<BlockDeviceRef>>,
}

const NR_LOOP: usize = 8;

lazy_static! {
    static ref LOOPS: [Arc<LoopDevice>; NR_LOOP] = {
        let mut v: alloc::vec::Vec<Arc<LoopDevice>> = alloc::vec::Vec::with_capacity(NR_LOOP);
        for i in 0..NR_LOOP {
            v.push(Arc::new(LoopDevice {
                minor: i as u32,
                bdev: Mutex::new(None),
            }));
        }
        let a: [Arc<LoopDevice>; NR_LOOP] = core::array::from_fn(|i| v[i].clone());
        a
    };
}

static NEXT_CONFIG_GEN: AtomicU32 = AtomicU32::new(0);

/// LOOP_CTL_GET_FREE — return the smallest unconfigured minor.
pub fn loop_ctl_get_free() -> Result<u32, i32> {
    for ld in LOOPS.iter() {
        if ld.bdev.lock().is_none() {
            return Ok(ld.minor);
        }
    }
    Err(EBUSY)
}

/// Configure loop device `minor` to back onto the bytes in `data`.  Returns
/// the resulting BlockDevice.  Equivalent to `LOOP_CONFIGURE` with an
/// underlying memfd / file.
pub fn loop_configure_from_bytes(minor: u32, data: Vec<u8>) -> Result<BlockDeviceRef, i32> {
    let ld = LOOPS.get(minor as usize).ok_or(EINVAL)?;
    let mut g = ld.bdev.lock();
    if g.is_some() {
        return Err(EBUSY);
    }
    let mem = MemBlockDevice::new(&alloc::format!("loop{}", minor), data.len());
    {
        let mut buf = mem.data.lock();
        *buf = data;
    }
    let bd = BlockDevice::wrap(mem, mem_block_device_ops());
    *g = Some(bd.clone());
    NEXT_CONFIG_GEN.fetch_add(1, Ordering::AcqRel);
    Ok(bd)
}

/// LOOP_CLR_FD — detach the backing.
pub fn loop_clear(minor: u32) -> Result<(), i32> {
    let ld = LOOPS.get(minor as usize).ok_or(EINVAL)?;
    let mut g = ld.bdev.lock();
    if g.is_none() {
        return Err(ENOENT);
    }
    *g = None;
    Ok(())
}

pub fn loop_get(minor: u32) -> Option<BlockDeviceRef> {
    LOOPS.get(minor as usize)?.bdev.lock().clone()
}
