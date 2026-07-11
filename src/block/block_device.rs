//! linux-parity: partial
//! linux-source: vendor/linux/block/fops.c
//! linux-source: vendor/linux/fs/read_write.c
//! linux-source: vendor/linux/drivers/base/core.c
//! `struct block_device` (bdev) — registry + driver vtable.
//!
//! Mirrors `vendor/linux/include/linux/blkdev.h::struct block_device`.  M43
//! keeps a flat name → Arc<BlockDevice> registry instead of a bdev pseudo-
//! filesystem; that lands with M54 device-model integration.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::ops::FileOps;
use crate::fs::types::FileRef;

use super::bio::{BIO_OP_READ, BioOp, BioRef, BioVec, bio_alloc, submit_bio};

pub type BlockDeviceRef = Arc<BlockDevice>;

/// Driver-supplied vtable.  `submit_bio` runs the I/O synchronously in M43.
pub struct BlockDeviceOps {
    pub name: &'static str,
    pub submit_bio: fn(&BlockDeviceRef, &BioRef) -> Result<(), i32>,
    pub get_capacity: fn(&BlockDeviceRef) -> u64, // in 512-byte sectors
    pub block_size: fn(&BlockDeviceRef) -> u32,   // logical block size
    pub ioctl: Option<fn(&BlockDeviceRef, u32, u64) -> Result<i64, i32>>,
}

pub struct BlockDevice {
    pub id: u64,
    pub name: String,
    pub ops: &'static BlockDeviceOps,
    pub backing: Mutex<Option<Arc<dyn Any + Send + Sync>>>,
    pub capacity_sectors: AtomicU64,
}

impl BlockDevice {
    pub fn wrap<T: Any + Send + Sync>(
        backing: Arc<T>,
        ops: &'static BlockDeviceOps,
    ) -> BlockDeviceRef {
        let bd = Arc::new(Self {
            id: NEXT_BDEV_ID.fetch_add(1, Ordering::AcqRel),
            name: alloc::string::String::new(),
            ops,
            backing: Mutex::new(Some(backing as Arc<dyn Any + Send + Sync>)),
            capacity_sectors: AtomicU64::new(0),
        });
        bd.capacity_sectors
            .store((ops.get_capacity)(&bd), Ordering::Release);
        bd
    }
    pub fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors.load(Ordering::Acquire)
    }
    pub fn capacity_bytes(&self) -> u64 {
        self.capacity_sectors() * 512
    }
}

static NEXT_BDEV_ID: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref BLOCK_DEV_REGISTRY: Mutex<BTreeMap<String, BlockDeviceRef>> =
        Mutex::new(BTreeMap::new());
}
static BLOCK_DEV_EVENT_LOCK: Mutex<()> = Mutex::new(());

pub fn init_registry() {}

pub fn register_block_device(name: &str, bdev: BlockDeviceRef) -> Result<(), i32> {
    let _event = BLOCK_DEV_EVENT_LOCK.lock();
    let mut reg = BLOCK_DEV_REGISTRY.lock();
    if reg.contains_key(name) {
        return Err(crate::include::uapi::errno::EBUSY);
    }
    // SAFETY: `name` is exactly what the caller asked for; we record it on the bdev.
    let bd_inner: &BlockDevice = &bdev;
    let _ = bd_inner; // satisfy borrow chk
    reg.insert(String::from(name), bdev);
    drop(reg);

    // Linux device_add() asks devtmpfs to create the device node after the
    // device is visible to the core.  devtmpfs creation failure does not roll
    // back device registration, so the Lupos hook intentionally has no error
    // result here either.
    crate::mm::shmem::publish_devtmpfs_block_device(name);
    Ok(())
}

pub fn unregister_block_device(name: &str) -> Option<BlockDeviceRef> {
    let _event = BLOCK_DEV_EVENT_LOCK.lock();
    let mut reg = BLOCK_DEV_REGISTRY.lock();
    let removed = reg.remove(name).or_else(|| {
        name.strip_prefix("/dev/")
            .and_then(|short| reg.remove(short))
    });
    drop(reg);
    if removed.is_some() {
        crate::mm::shmem::unpublish_devtmpfs_block_device(name);
    }
    removed
}

pub fn lookup_block_device(name: &str) -> Option<BlockDeviceRef> {
    let reg = BLOCK_DEV_REGISTRY.lock();
    reg.get(name).cloned().or_else(|| {
        name.strip_prefix("/dev/")
            .and_then(|short| reg.get(short).cloned())
    })
}

pub fn registered_block_devices() -> Vec<(String, BlockDeviceRef)> {
    BLOCK_DEV_REGISTRY
        .lock()
        .iter()
        .map(|(name, bdev)| (name.clone(), bdev.clone()))
        .collect()
}

/// Publish a coherent snapshot when devtmpfs first becomes available.  The
/// event lock orders this snapshot against registration and teardown, avoiding
/// a stale node when a disk disappears while devtmpfs is being mounted.
pub fn publish_registered_block_devices_to_devtmpfs() {
    let _event = BLOCK_DEV_EVENT_LOCK.lock();
    let names: Vec<String> = BLOCK_DEV_REGISTRY.lock().keys().cloned().collect();
    for name in names {
        crate::mm::shmem::publish_devtmpfs_block_device(&name);
    }
}

pub fn block_device_ioctl(bdev: &BlockDeviceRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let ioctl = bdev.ops.ioctl.ok_or(crate::include::uapi::errno::ENOTTY)?;
    ioctl(bdev, cmd, arg)
}

pub static BLOCK_DEVICE_FILE_OPS: FileOps = FileOps {
    name: "block_device",
    read: Some(block_device_file_read),
    write: None,
    llseek: Some(block_device_file_llseek),
    fsync: None,
    poll: None,
    ioctl: Some(block_device_file_ioctl),
    mmap: None,
    release: None,
    readdir: None,
};

/// Linux's buffered block-device path (`blkdev_read_iter` -> `filemap_read`)
/// accepts byte-granular reads even though requests reaching the block driver
/// must be aligned to its logical block size.  Lupos has no block-device page
/// cache mapping yet, so perform the equivalent read/modify-copy operation
/// through bounded, logical-block-aligned BIO bounce buffers.
fn block_device_file_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    use crate::include::uapi::errno::{EIO, ENODEV};

    let bdev = block_device_for_file(file).ok_or(ENODEV)?;
    let capacity = bdev.capacity_sectors().checked_mul(512).ok_or(EIO)?;

    // `blkdev_read_iter()` truncates the iterator at bdev_nr_bytes() and
    // returns zero when ki_pos is already at or beyond the end of the device.
    if buf.is_empty() || *pos >= capacity {
        return Ok(0);
    }
    let count = ((capacity - *pos).min(buf.len() as u64)) as usize;

    // Linux validates queue logical block sizes before a disk is registered.
    // Keep the same invariant at this Rust-native boundary before using it for
    // BIO alignment.
    let logical_block_size = (bdev.ops.block_size)(&bdev) as u64;
    if logical_block_size < 512
        || !logical_block_size.is_power_of_two()
        || logical_block_size % 512 != 0
    {
        return Err(EIO);
    }

    // A page is the normal unit of buffered block-device reads on x86_64.
    // Keep each BIO bounded to one page while allowing an edge BIO to cover a
    // whole logical block when that is the larger alignment unit.
    const BUFFERED_READ_CHUNK: u64 = 4096;
    let max_io = BUFFERED_READ_CHUNK.max(logical_block_size);
    let start = *pos;
    let mut done = 0usize;

    while done < count {
        let absolute = start.checked_add(done as u64).ok_or(EIO)?;
        let block_start = absolute / logical_block_size * logical_block_size;
        let offset_in_block = (absolute - block_start) as usize;
        let remaining = count - done;

        // Batch the aligned middle of a read, but read one complete logical
        // block for an unaligned head or tail.  This is the bounce-buffer
        // equivalent of Linux filling a folio and copying only the requested
        // iterator range.
        let io_len = if offset_in_block == 0 {
            let aligned = (remaining as u64 / logical_block_size) * logical_block_size;
            if aligned == 0 {
                logical_block_size
            } else {
                aligned.min(max_io)
            }
        } else {
            logical_block_size
        };
        let io_end = block_start.checked_add(io_len).ok_or(EIO)?;
        if io_end > capacity {
            // A registered Linux block device has capacity aligned to its
            // logical block size.  Reaching this branch means the native
            // BlockDeviceOps contract is malformed, so surface an I/O error.
            if done == 0 {
                return Err(EIO);
            }
            break;
        }

        let io_len = io_len as usize;
        let segment = BioVec::new(alloc::vec![0u8; io_len]);
        let segment_data = segment.data.clone();
        let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_READ), block_start / 512);
        bio.add_vec(segment);
        if let Err(errno) = submit_bio(bio) {
            // Linux's filemap read path returns bytes already copied before a
            // later I/O error; only an error before progress is reported.
            if done == 0 {
                return Err(errno);
            }
            break;
        }

        let copied = remaining.min(io_len - offset_in_block);
        let data = segment_data.lock();
        buf[done..done + copied].copy_from_slice(&data[offset_in_block..offset_in_block + copied]);
        done += copied;
    }

    *pos = start.checked_add(done as u64).ok_or(EIO)?;
    Ok(done)
}

/// `blkdev_llseek()` delegates to `fixed_size_llseek()` with the live block
/// device capacity, rather than the size (normally zero) of the special-file
/// inode used to open it.
fn block_device_file_llseek(file: &FileRef, off: i64, whence: i32) -> Result<u64, i32> {
    use crate::include::uapi::errno::{EINVAL, EIO, ENODEV};

    const SEEK_SET: i32 = 0;
    const SEEK_CUR: i32 = 1;
    const SEEK_END: i32 = 2;

    let bdev = block_device_for_file(file).ok_or(ENODEV)?;
    let capacity = bdev.capacity_sectors().checked_mul(512).ok_or(EIO)?;
    let mut position = file.pos.lock();
    let new_position = match whence {
        SEEK_SET => off as i128,
        SEEK_CUR => *position as i128 + off as i128,
        SEEK_END => capacity as i128 + off as i128,
        _ => return Err(EINVAL),
    };
    if new_position < 0 || new_position > capacity as i128 {
        return Err(EINVAL);
    }
    *position = new_position as u64;
    Ok(*position)
}

fn block_device_file_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let bdev = block_device_for_file(file).ok_or(crate::include::uapi::errno::ENODEV)?;
    block_device_ioctl(&bdev, cmd, arg)
}

fn block_device_for_file(file: &FileRef) -> Option<BlockDeviceRef> {
    // Linux binds an opened block-special inode through i_rdev.  Lupos' native
    // registry is name based, so try the requested path first, then the final
    // resolved dentry.  The latter is essential when the file was opened via a
    // symlink such as /dev/root.
    crate::fs::file::path_hint(file)
        .and_then(|path| lookup_block_device(&normalize_block_device_name(path)))
        .or_else(|| {
            crate::fs::mount::path_for_dentry(&file.dentry)
                .and_then(|path| lookup_block_device(&normalize_block_device_name(path)))
        })
        .or_else(|| lookup_block_device(&file.dentry.name))
}

fn normalize_block_device_name(path: String) -> String {
    path.strip_prefix("/dev/")
        .or_else(|| path.strip_prefix("dev/"))
        .unwrap_or(path.as_str())
        .trim_start_matches('/')
        .to_string()
}

pub fn registered_count() -> usize {
    BLOCK_DEV_REGISTRY.lock().len()
}
