//! linux-parity: partial
//! linux-source: vendor/linux/block
//! `struct gendisk` — disk descriptor + sysfs hookup.
//!
//! Mirrors `vendor/linux/include/linux/{genhd,blkdev}.h`.  Each gendisk
//! registers a kobject under `/sys/block/<name>` so userspace tools can
//! discover the device once sysfs is mounted.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;

use lazy_static::lazy_static;
use spin::Mutex;

use super::block_device::BlockDeviceRef;

pub struct GenDisk {
    pub name: String,
    pub bdev: BlockDeviceRef,
    pub minor: u32,
    pub capacity_sectors: u64,
}

lazy_static! {
    static ref GENDISK_REGISTRY: Mutex<BTreeMap<String, Arc<GenDisk>>> =
        Mutex::new(BTreeMap::new());
}

pub fn init_registry() {}

pub fn register_gendisk(name: &str, bdev: BlockDeviceRef) -> Arc<GenDisk> {
    let cap = bdev.capacity_sectors();
    let g = Arc::new(GenDisk {
        name: String::from(name),
        bdev,
        minor: 0,
        capacity_sectors: cap,
    });
    GENDISK_REGISTRY
        .lock()
        .insert(String::from(name), g.clone());
    g
}

pub fn lookup_gendisk(name: &str) -> Option<Arc<GenDisk>> {
    GENDISK_REGISTRY.lock().get(name).cloned()
}

pub fn unregister_gendisk(name: &str) -> Option<Arc<GenDisk>> {
    GENDISK_REGISTRY.lock().remove(name)
}

pub fn registered_count() -> usize {
    GENDISK_REGISTRY.lock().len()
}

pub fn for_each<F: FnMut(&Arc<GenDisk>)>(mut f: F) {
    for (_n, g) in GENDISK_REGISTRY.lock().iter() {
        f(g);
    }
}
