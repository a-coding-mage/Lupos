//! linux-parity: partial
//! linux-source: vendor/linux/lib/kobject.c
//! kobject / kset / sysfs attribute model — M41.
//!
//! Mirrors `vendor/linux/lib/kobject.c` and `vendor/linux/include/linux/kobject.h`.
//! Refcounted kernel objects exposed through sysfs.  Devtmpfs auto-population
//! is deferred to M54 (needs the device-model bus/class glue).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::kernfs::{KernfsNode, ShowFn, StoreFn, add_child};

pub struct Attribute {
    pub name: &'static str,
    pub mode: u32,
    pub show: Option<ShowFn>,
    pub store: Option<StoreFn>,
}

pub struct BinAttribute {
    pub name: &'static str,
    pub mode: u32,
    pub size: usize,
    pub data: Mutex<Vec<u8>>,
}

pub struct KType {
    pub name: &'static str,
    pub release: Option<fn(&KObject)>,
    pub default_attrs: &'static [&'static Attribute],
}

pub struct KObject {
    pub name: String,
    pub kref: AtomicU64,
    pub state: AtomicU32,
    pub parent: Mutex<Option<Arc<KObject>>>,
    pub ktype: Option<&'static KType>,
    pub kset: Mutex<Option<Arc<KSet>>>,
    pub kn: Mutex<Option<Arc<KernfsNode>>>,
    pub attrs: Mutex<Vec<&'static Attribute>>,
    pub bin_attrs: Mutex<Vec<&'static BinAttribute>>,
}

pub const KOBJECT_STATE_INITIALIZED: u32 = 1 << 0;
pub const KOBJECT_STATE_IN_SYSFS: u32 = 1 << 1;

impl KObject {
    pub fn new(name: &str, ktype: Option<&'static KType>) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            kref: AtomicU64::new(1),
            state: AtomicU32::new(KOBJECT_STATE_INITIALIZED),
            parent: Mutex::new(None),
            ktype,
            kset: Mutex::new(None),
            kn: Mutex::new(None),
            attrs: Mutex::new(Vec::new()),
            bin_attrs: Mutex::new(Vec::new()),
        })
    }
    pub fn add_attribute(&self, a: &'static Attribute) {
        self.attrs.lock().push(a);
    }
    pub fn add_bin_attribute(&self, a: &'static BinAttribute) {
        self.bin_attrs.lock().push(a);
    }
}

pub struct KSet {
    pub kobj: Arc<KObject>,
    pub list: Mutex<Vec<Arc<KObject>>>,
}

impl KSet {
    pub fn new(name: &str) -> Arc<Self> {
        Arc::new(Self {
            kobj: KObject::new(name, None),
            list: Mutex::new(Vec::new()),
        })
    }
}

// ── Registry — populated by `kobject_add`, consumed by sysfs mount ────────

lazy_static! {
    static ref ROOT_OBJECTS: Mutex<BTreeMap<String, Arc<KObject>>> = Mutex::new(BTreeMap::new());
}

/// Register `kobj` under `parent`.  M41: parent is currently always
/// `/sys/kernel/`; subdirectories will land with the device model in M54.
pub fn kobject_add(kobj: Arc<KObject>) -> Result<(), i32> {
    let cur = kobj.state.load(Ordering::Acquire);
    kobj.state
        .store(cur | KOBJECT_STATE_IN_SYSFS, Ordering::Release);
    ROOT_OBJECTS.lock().insert(kobj.name.clone(), kobj);
    Ok(())
}

/// Show callback that reads a `BinAttribute` payload.
fn binattr_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let raw = node.priv_ptr.load(Ordering::Acquire) as *const BinAttribute;
    if raw.is_null() {
        return Err(crate::include::uapi::errno::EINVAL);
    }
    let ba = unsafe { &*raw };
    let g = ba.data.lock();
    let n = g.len().min(buf.len());
    buf[..n].copy_from_slice(&g[..n]);
    Ok(n)
}

fn binattr_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let raw = node.priv_ptr.load(Ordering::Acquire) as *const BinAttribute;
    if raw.is_null() {
        return Err(crate::include::uapi::errno::EINVAL);
    }
    let ba = unsafe { &*raw };
    let mut g = ba.data.lock();
    g.clear();
    g.extend_from_slice(buf);
    Ok(buf.len())
}

/// Called by sysfs::mount — bolt the registered kobjects onto the kernfs
/// hierarchy at `/sys/kernel/`.
pub fn sysfs_attach_root(kernel_dir: &Arc<KernfsNode>) {
    for (_name, kobj) in ROOT_OBJECTS.lock().iter() {
        let kdir = KernfsNode::new_dir(&kobj.name, 0o555);
        for a in kobj.attrs.lock().iter() {
            let f = KernfsNode::new_file(a.name, a.mode, a.show, a.store);
            add_child(&kdir, f);
        }
        for ba in kobj.bin_attrs.lock().iter() {
            let f = KernfsNode::new_file(ba.name, ba.mode, Some(binattr_show), Some(binattr_store));
            f.priv_ptr.store(*ba as *const _ as u64, Ordering::Release);
            add_child(&kdir, f);
        }
        *kobj.kn.lock() = Some(kdir.clone());
        add_child(kernel_dir, kdir);
    }
}

/// Diagnostic.
pub fn registered_count() -> usize {
    ROOT_OBJECTS.lock().len()
}
