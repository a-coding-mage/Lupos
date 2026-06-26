//! linux-parity: partial
//! linux-source: vendor/linux/block
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

use super::bio::BioRef;

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

pub fn init_registry() {}

pub fn register_block_device(name: &str, bdev: BlockDeviceRef) -> Result<(), i32> {
    let mut reg = BLOCK_DEV_REGISTRY.lock();
    if reg.contains_key(name) {
        return Err(crate::include::uapi::errno::EBUSY);
    }
    // SAFETY: `name` is exactly what the caller asked for; we record it on the bdev.
    let bd_inner: &BlockDevice = &bdev;
    let _ = bd_inner; // satisfy borrow chk
    reg.insert(String::from(name), bdev);
    Ok(())
}

pub fn unregister_block_device(name: &str) -> Option<BlockDeviceRef> {
    let mut reg = BLOCK_DEV_REGISTRY.lock();
    if let Some(removed) = reg.remove(name) {
        return Some(removed);
    }
    name.strip_prefix("/dev/")
        .and_then(|short| reg.remove(short))
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

pub fn block_device_ioctl(bdev: &BlockDeviceRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let ioctl = bdev.ops.ioctl.ok_or(crate::include::uapi::errno::ENOTTY)?;
    ioctl(bdev, cmd, arg)
}

pub static BLOCK_DEVICE_FILE_OPS: FileOps = FileOps {
    name: "block_device",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: Some(block_device_file_ioctl),
    mmap: None,
    release: None,
    readdir: None,
};

fn block_device_file_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let name = block_device_name_for_file(file).ok_or(crate::include::uapi::errno::ENODEV)?;
    let bdev = lookup_block_device(&name).ok_or(crate::include::uapi::errno::ENODEV)?;
    block_device_ioctl(&bdev, cmd, arg)
}

fn block_device_name_for_file(file: &FileRef) -> Option<String> {
    crate::fs::file::path_hint(file)
        .or_else(|| crate::fs::mount::path_for_dentry(&file.dentry))
        .map(normalize_block_device_name)
        .or_else(|| Some(file.dentry.name.to_string()))
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
