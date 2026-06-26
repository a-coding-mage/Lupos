//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/class.c
//! `struct class` — `vendor/linux/include/linux/device/class.h`.
//!
//! A class is a logical grouping of devices that share a userspace contract
//! (e.g. `block`, `net`, `tty`, `input`).  Each registered class appears as
//! `/sys/class/<name>/`; member devices are linked under it.
//!
//! Mirrors `drivers/base/class.c:178` (`class_register`).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::EEXIST;
use crate::linux_driver_abi::base::device::Device;

pub struct Class {
    pub name: &'static str,
    pub devices: Mutex<Vec<Arc<Device>>>,
}

impl Class {
    pub fn new(name: &'static str) -> Arc<Self> {
        Arc::new(Self {
            name,
            devices: Mutex::new(Vec::new()),
        })
    }
}

lazy_static! {
    pub(crate) static ref CLASSES: Mutex<BTreeMap<String, Arc<Class>>> =
        Mutex::new(BTreeMap::new());
}

/// `class_register` — `drivers/base/class.c:178`.
pub fn class_register(class: Arc<Class>) -> Result<(), i32> {
    let mut g = CLASSES.lock();
    if g.contains_key(class.name) {
        return Err(EEXIST);
    }
    g.insert(String::from(class.name), class);
    Ok(())
}

pub fn registered_classes() -> Vec<&'static str> {
    CLASSES.lock().values().map(|c| c.name).collect()
}

pub fn find_class(name: &str) -> Option<Arc<Class>> {
    CLASSES.lock().get(name).cloned()
}
