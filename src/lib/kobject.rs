//! linux-parity: partial
//! linux-source: vendor/linux/lib/kobject.c
//! test-origin: linux:vendor/linux/lib/kobject.c
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
use core::ffi::{c_char, c_void};
use core::ptr;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::kernfs::{KernfsNode, ShowFn, StoreFn, add_child};
use crate::include::uapi::errno::{EINVAL, EIO, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::GFP_KERNEL;

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

/// `struct list_head` — `vendor/linux/include/linux/types.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxListHead {
    pub next: *mut c_void,
    pub prev: *mut c_void,
}

/// Prefix of `struct kobject` — `vendor/linux/include/linux/kobject.h`.
#[repr(C)]
pub struct LinuxKObject {
    name: *const c_char,
    entry: LinuxListHead,
    parent: *mut LinuxKObject,
    kset: *mut c_void,
    ktype: *const c_void,
    sd: *mut c_void,
    kref: i32,
    state_flags: u32,
}

type LinuxKObjectRelease = unsafe extern "C" fn(*mut LinuxKObject);

/// `struct attribute` — `vendor/linux/include/linux/sysfs.h`.
#[repr(C)]
struct LinuxAttribute {
    name: *const c_char,
    mode: u16,
}

type LinuxKObjAttrShow =
    unsafe extern "C" fn(*mut LinuxKObject, *mut LinuxKObjAttribute, *mut c_char) -> isize;
type LinuxKObjAttrStore =
    unsafe extern "C" fn(*mut LinuxKObject, *mut LinuxKObjAttribute, *const c_char, usize) -> isize;

/// `struct kobj_attribute` — `vendor/linux/include/linux/kobject.h`.
#[repr(C)]
struct LinuxKObjAttribute {
    attr: LinuxAttribute,
    show: Option<LinuxKObjAttrShow>,
    store: Option<LinuxKObjAttrStore>,
}

/// `struct sysfs_ops` — `vendor/linux/include/linux/sysfs.h`.
#[repr(C)]
pub struct LinuxSysfsOps {
    show:
        Option<unsafe extern "C" fn(*mut LinuxKObject, *mut LinuxAttribute, *mut c_char) -> isize>,
    store: Option<
        unsafe extern "C" fn(*mut LinuxKObject, *mut LinuxAttribute, *const c_char, usize) -> isize,
    >,
}

/// `kobj_sysfs_ops` - `vendor/linux/lib/kobject.c:844`.
#[unsafe(export_name = "kobj_sysfs_ops")]
pub static LINUX_KOBJ_SYSFS_OPS: LinuxSysfsOps = LinuxSysfsOps {
    show: Some(linux_kobj_attr_show),
    store: Some(linux_kobj_attr_store),
};

/// Prefix of `struct kobj_type` — `vendor/linux/include/linux/kobject.h`.
#[repr(C)]
struct LinuxKObjType {
    release: Option<LinuxKObjectRelease>,
    sysfs_ops: *const c_void,
    default_groups: *const c_void,
    child_ns_type: *const c_void,
    namespace: *const c_void,
    get_ownership: *const c_void,
}

unsafe impl Sync for LinuxKObjType {}

const LINUX_KOBJ_STATE_INITIALIZED: u32 = 1 << 0;
const LINUX_KOBJ_STATE_IN_SYSFS: u32 = 1 << 1;
const LINUX_KOBJ_STATE_ADD_UEVENT_SENT: u32 = 1 << 2;
const LINUX_KOBJ_STATE_REMOVE_UEVENT_SENT: u32 = 1 << 3;
const LINUX_KOBJ_STATE_UEVENT_SUPPRESS: u32 = 1 << 4;

static LINUX_DYNAMIC_KOBJ_TYPE: LinuxKObjType = LinuxKObjType {
    release: Some(linux_dynamic_kobject_release),
    sysfs_ops: &LINUX_KOBJ_SYSFS_OPS as *const LinuxSysfsOps as *const c_void,
    default_groups: ptr::null(),
    child_ns_type: ptr::null(),
    namespace: ptr::null(),
    get_ownership: ptr::null(),
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("kobject_init", linux_kobject_init as usize, false);
    export_symbol_once("kobject_add", linux_kobject_add as usize, false);
    export_symbol_once(
        "kobject_init_and_add",
        linux_kobject_init_and_add as usize,
        true,
    );
    export_symbol_once(
        "kobject_create_and_add",
        linux_kobject_create_and_add as usize,
        false,
    );
    export_symbol_once("kobject_get", linux_kobject_get as usize, false);
    export_symbol_once(
        "kobject_get_unless_zero",
        linux_kobject_get_unless_zero as usize,
        false,
    );
    export_symbol_once("kobject_put", linux_kobject_put as usize, false);
    export_symbol_once("kobject_set_name", linux_kobject_set_name as usize, false);
    export_symbol_once("kobject_uevent", linux_kobject_uevent as usize, true);
    export_symbol_once(
        "kobj_sysfs_ops",
        core::ptr::addr_of!(LINUX_KOBJ_SYSFS_OPS) as usize,
        true,
    );
}

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

/// Raw field initialization shared by `kobject_init()` and modeled
/// `device_initialize()`.
pub unsafe fn init_linux_kobject_raw(kobj: *mut c_void, ktype: *const c_void) {
    if kobj.is_null() {
        return;
    }

    let kobj = kobj.cast::<LinuxKObject>();
    let entry = unsafe { core::ptr::addr_of_mut!((*kobj).entry) };
    unsafe {
        (*kobj).kref = 1;
        (*entry).next = entry.cast();
        (*entry).prev = entry.cast();
        (*kobj).state_flags &= LINUX_KOBJ_STATE_UEVENT_SUPPRESS;
        (*kobj).state_flags |= LINUX_KOBJ_STATE_INITIALIZED;
        (*kobj).ktype = ktype;
    }
}

unsafe extern "C" fn linux_dynamic_kobject_release(kobj: *mut LinuxKObject) {
    unsafe { crate::mm::slab::kfree(kobj.cast()) };
}

unsafe fn linux_kobject_name_is_empty(kobj: *const LinuxKObject) -> bool {
    if kobj.is_null() {
        return true;
    }
    let name = unsafe { (*kobj).name };
    name.is_null() || unsafe { *name.cast::<u8>() == 0 }
}

/// `kobject_init` - `vendor/linux/lib/kobject.c`.
#[unsafe(export_name = "kobject_init")]
pub unsafe extern "C" fn linux_kobject_init(kobj: *mut LinuxKObject, ktype: *const c_void) {
    if kobj.is_null() {
        crate::log_warn!("kobject", "kobject_init: null kobject");
        return;
    }
    if ktype.is_null() {
        crate::log_warn!("kobject", "kobject_init: null ktype for {:p}", kobj);
        return;
    }

    let was_initialized = unsafe { (*kobj).state_flags & LINUX_KOBJ_STATE_INITIALIZED != 0 };
    if was_initialized {
        crate::log_warn!(
            "kobject",
            "kobject_init: reinitializing initialized object {:p}",
            kobj
        );
    }

    unsafe { init_linux_kobject_raw(kobj.cast(), ktype) };
}

/// `kobject_add` - `vendor/linux/lib/kobject.c`.
unsafe extern "C" fn linux_kobject_add(
    kobj: *mut LinuxKObject,
    parent: *mut LinuxKObject,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> i32 {
    if kobj.is_null() {
        return -EINVAL;
    }
    if unsafe { (*kobj).state_flags & LINUX_KOBJ_STATE_INITIALIZED == 0 } {
        crate::log_warn!("kobject", "kobject_add: uninitialized object {:p}", kobj);
        return -EINVAL;
    }

    let retval = unsafe { linux_kobject_set_name(kobj, fmt, arg0, arg1, arg2, arg3) };
    if retval != 0 {
        return retval;
    }
    if unsafe { linux_kobject_name_is_empty(kobj) } {
        crate::log_warn!("kobject", "kobject_add: object {:p} has empty name", kobj);
        return -EINVAL;
    }

    unsafe {
        (*kobj).parent = linux_kobject_get(parent);
        (*kobj).state_flags |= LINUX_KOBJ_STATE_IN_SYSFS;
    }
    0
}

/// `kobject_init_and_add` - `vendor/linux/lib/kobject.c`.
#[unsafe(export_name = "kobject_init_and_add")]
unsafe extern "C" fn linux_kobject_init_and_add(
    kobj: *mut LinuxKObject,
    ktype: *const c_void,
    parent: *mut LinuxKObject,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> i32 {
    unsafe {
        linux_kobject_init(kobj, ktype);
        linux_kobject_add(kobj, parent, fmt, arg0, arg1, arg2, arg3)
    }
}

/// `kobject_create_and_add` - `vendor/linux/lib/kobject.c`.
unsafe extern "C" fn linux_kobject_create_and_add(
    name: *const c_char,
    parent: *mut LinuxKObject,
) -> *mut LinuxKObject {
    if name.is_null() {
        return ptr::null_mut();
    }

    let kobj = unsafe {
        crate::mm::slab::kzalloc_noprof(core::mem::size_of::<LinuxKObject>(), GFP_KERNEL)
            .cast::<LinuxKObject>()
    };
    if kobj.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        init_linux_kobject_raw(
            kobj.cast(),
            ptr::addr_of!(LINUX_DYNAMIC_KOBJ_TYPE).cast::<c_void>(),
        );
    }
    let retval = unsafe { linux_kobject_add(kobj, parent, c"%s".as_ptr(), name as usize, 0, 0, 0) };
    if retval != 0 {
        unsafe { linux_kobject_put(kobj) };
        ptr::null_mut()
    } else {
        kobj
    }
}

/// `kobject_get` - `vendor/linux/lib/kobject.c`.
unsafe extern "C" fn linux_kobject_get(kobj: *mut LinuxKObject) -> *mut LinuxKObject {
    if !kobj.is_null() {
        unsafe {
            if (*kobj).state_flags & LINUX_KOBJ_STATE_INITIALIZED == 0 {
                crate::log_warn!("kobject", "kobject_get: uninitialized object {:p}", kobj);
            }
            (*kobj).kref = (*kobj).kref.saturating_add(1);
        }
    }
    kobj
}

/// `kobject_get_unless_zero` - `vendor/linux/lib/kobject.c`.
unsafe extern "C" fn linux_kobject_get_unless_zero(kobj: *mut LinuxKObject) -> *mut LinuxKObject {
    if kobj.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        if (*kobj).kref <= 0 {
            ptr::null_mut()
        } else {
            (*kobj).kref = (*kobj).kref.saturating_add(1);
            kobj
        }
    }
}

/// `kobject_put` - `vendor/linux/lib/kobject.c`.
unsafe extern "C" fn linux_kobject_put(kobj: *mut LinuxKObject) {
    if kobj.is_null() {
        return;
    }

    let release;
    let parent;
    unsafe {
        if (*kobj).state_flags & LINUX_KOBJ_STATE_INITIALIZED == 0 {
            crate::log_warn!("kobject", "kobject_put: uninitialized object {:p}", kobj);
        }
        if (*kobj).kref <= 0 {
            return;
        }
        (*kobj).kref -= 1;
        if (*kobj).kref != 0 {
            return;
        }

        (*kobj).state_flags &= !LINUX_KOBJ_STATE_IN_SYSFS;
        parent = (*kobj).parent;
        (*kobj).parent = ptr::null_mut();
        release = if (*kobj).ktype.is_null() {
            None
        } else {
            (*((*kobj).ktype.cast::<LinuxKObjType>())).release
        };
    }

    if let Some(release) = release {
        unsafe { release(kobj) };
    }
    unsafe { linux_kobject_put(parent) };
}

/// `kobject_uevent` - `vendor/linux/lib/kobject_uevent.c`.
unsafe extern "C" fn linux_kobject_uevent(_kobj: *mut LinuxKObject, _action: i32) -> i32 {
    0
}

unsafe extern "C" fn linux_kobj_attr_show(
    kobj: *mut LinuxKObject,
    attr: *mut LinuxAttribute,
    buf: *mut c_char,
) -> isize {
    if attr.is_null() {
        return -(EIO as isize);
    }
    let kattr = attr.cast::<LinuxKObjAttribute>();
    match unsafe { (*kattr).show } {
        Some(show) => unsafe { show(kobj, kattr, buf) },
        None => -(EIO as isize),
    }
}

unsafe extern "C" fn linux_kobj_attr_store(
    kobj: *mut LinuxKObject,
    attr: *mut LinuxAttribute,
    buf: *const c_char,
    count: usize,
) -> isize {
    if attr.is_null() {
        return -(EIO as isize);
    }
    let kattr = attr.cast::<LinuxKObjAttribute>();
    match unsafe { (*kattr).store } {
        Some(store) => unsafe { store(kobj, kattr, buf, count) },
        None => -(EIO as isize),
    }
}

/// `kobject_set_name` - `vendor/linux/lib/kobject.c`.
unsafe extern "C" fn linux_kobject_set_name(
    kobj: *mut LinuxKObject,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> i32 {
    if kobj.is_null() {
        return -EINVAL;
    }
    if fmt.is_null() {
        return if unsafe { (*kobj).name.is_null() } {
            -EINVAL
        } else {
            0
        };
    }

    let name =
        unsafe { crate::lib::kasprintf::linux_kasprintf(GFP_KERNEL, fmt, arg0, arg1, arg2, arg3) };
    if name.is_null() {
        return -ENOMEM;
    }

    let mut cursor = name.cast::<u8>();
    loop {
        let byte = unsafe { cursor.read() };
        if byte == 0 {
            break;
        }
        if byte == b'/' {
            unsafe { cursor.write(b'!') };
        }
        cursor = unsafe { cursor.add(1) };
    }
    unsafe { (*kobj).name = name };
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn linux_kobject_c_layout_prefix_matches_vendor_header() {
        assert_eq!(offset_of!(LinuxKObject, name), 0);
        assert_eq!(offset_of!(LinuxKObject, entry), 8);
        assert_eq!(offset_of!(LinuxKObject, parent), 24);
        assert_eq!(offset_of!(LinuxKObject, kset), 32);
        assert_eq!(offset_of!(LinuxKObject, ktype), 40);
        assert_eq!(offset_of!(LinuxKObject, sd), 48);
        assert_eq!(offset_of!(LinuxKObject, kref), 56);
        assert_eq!(offset_of!(LinuxKObject, state_flags), 60);
        assert_eq!(size_of::<LinuxKObject>(), 64);
        assert_eq!(offset_of!(LinuxAttribute, name), 0);
        assert_eq!(offset_of!(LinuxAttribute, mode), 8);
        assert_eq!(size_of::<LinuxAttribute>(), 16);
        assert_eq!(offset_of!(LinuxKObjAttribute, attr), 0);
        assert_eq!(offset_of!(LinuxKObjAttribute, show), 16);
        assert_eq!(offset_of!(LinuxKObjAttribute, store), 24);
        assert_eq!(size_of::<LinuxKObjAttribute>(), 32);
        assert_eq!(offset_of!(LinuxSysfsOps, show), 0);
        assert_eq!(offset_of!(LinuxSysfsOps, store), 8);
        assert_eq!(size_of::<LinuxSysfsOps>(), 16);
    }

    #[test]
    fn kobj_sysfs_ops_matches_vendor_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/kobject.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/kobject.h"
        ));
        assert!(source.contains("const struct sysfs_ops kobj_sysfs_ops"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(kobj_sysfs_ops);"));
        assert!(header.contains("extern const struct sysfs_ops kobj_sysfs_ops;"));
    }

    #[test]
    fn linux_kobject_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("kobject_init"),
            Some(linux_kobject_init as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kobject_add"),
            Some(linux_kobject_add as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kobject_init_and_add"),
            Some(linux_kobject_init_and_add as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kobject_create_and_add"),
            Some(linux_kobject_create_and_add as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kobject_put"),
            Some(linux_kobject_put as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kobject_set_name"),
            Some(linux_kobject_set_name as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("kobj_sysfs_ops"),
            Some(core::ptr::addr_of!(LINUX_KOBJ_SYSFS_OPS) as usize)
        );
    }

    #[test]
    fn kobj_sysfs_ops_dispatches_kobj_attribute_callbacks() {
        unsafe extern "C" fn show(
            _kobj: *mut LinuxKObject,
            _attr: *mut LinuxKObjAttribute,
            buf: *mut c_char,
        ) -> isize {
            if !buf.is_null() {
                unsafe {
                    *buf = b'x' as c_char;
                }
            }
            1
        }

        unsafe extern "C" fn store(
            _kobj: *mut LinuxKObject,
            _attr: *mut LinuxKObjAttribute,
            _buf: *const c_char,
            count: usize,
        ) -> isize {
            count as isize
        }

        let mut kobj = LinuxKObject {
            name: core::ptr::null(),
            entry: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
            parent: core::ptr::null_mut(),
            kset: core::ptr::null_mut(),
            ktype: core::ptr::null(),
            sd: core::ptr::null_mut(),
            kref: 1,
            state_flags: 0,
        };
        let mut attr = LinuxKObjAttribute {
            attr: LinuxAttribute {
                name: c"status".as_ptr(),
                mode: 0o644,
            },
            show: Some(show),
            store: Some(store),
        };
        let mut out = [0 as c_char; 2];

        unsafe {
            assert_eq!(
                linux_kobj_attr_show(&mut kobj, &mut attr.attr, out.as_mut_ptr()),
                1
            );
            assert_eq!(out[0], b'x' as c_char);
            assert_eq!(
                linux_kobj_attr_store(&mut kobj, &mut attr.attr, c"new".as_ptr(), 3),
                3
            );

            attr.show = None;
            attr.store = None;
            assert_eq!(
                linux_kobj_attr_show(&mut kobj, &mut attr.attr, out.as_mut_ptr()),
                -(EIO as isize)
            );
            assert_eq!(
                linux_kobj_attr_store(&mut kobj, &mut attr.attr, c"new".as_ptr(), 3),
                -(EIO as isize)
            );
        }
    }

    #[test]
    fn linux_kobject_init_sets_ref_list_type_and_state() {
        unsafe {
            let mut kobj = core::mem::zeroed::<LinuxKObject>();
            let ktype = 0x1000usize as *const c_void;
            kobj.kref = 7;
            kobj.state_flags = LINUX_KOBJ_STATE_IN_SYSFS
                | LINUX_KOBJ_STATE_ADD_UEVENT_SENT
                | LINUX_KOBJ_STATE_REMOVE_UEVENT_SENT
                | LINUX_KOBJ_STATE_UEVENT_SUPPRESS;

            linux_kobject_init(&mut kobj, ktype);

            let entry = core::ptr::addr_of!(kobj.entry).cast::<c_void>() as *mut c_void;
            assert_eq!(kobj.kref, 1);
            assert_eq!(kobj.entry.next, entry);
            assert_eq!(kobj.entry.prev, entry);
            assert_eq!(kobj.ktype, ktype);
            assert_eq!(
                kobj.state_flags,
                LINUX_KOBJ_STATE_INITIALIZED | LINUX_KOBJ_STATE_UEVENT_SUPPRESS
            );
        }
    }

    #[test]
    fn linux_kobject_init_rejects_null_ktype_without_mutating() {
        unsafe {
            let mut kobj = core::mem::zeroed::<LinuxKObject>();
            kobj.kref = 7;
            kobj.state_flags = LINUX_KOBJ_STATE_IN_SYSFS;

            linux_kobject_init(&mut kobj, core::ptr::null());

            assert_eq!(kobj.kref, 7);
            assert!(kobj.entry.next.is_null());
            assert!(kobj.entry.prev.is_null());
            assert_eq!(kobj.state_flags, LINUX_KOBJ_STATE_IN_SYSFS);
            assert!(kobj.ktype.is_null());
        }
    }

    #[test]
    fn linux_kobject_add_sets_name_parent_and_sysfs_state() {
        unsafe {
            let mut parent = core::mem::zeroed::<LinuxKObject>();
            let mut child = core::mem::zeroed::<LinuxKObject>();
            let ktype_storage = LinuxKObjType {
                release: None,
                sysfs_ops: core::ptr::null(),
                default_groups: core::ptr::null(),
                child_ns_type: core::ptr::null(),
                namespace: core::ptr::null(),
                get_ownership: core::ptr::null(),
            };
            let ktype = core::ptr::addr_of!(ktype_storage).cast::<c_void>();
            linux_kobject_init(&mut parent, ktype);
            linux_kobject_init(&mut child, ktype);

            assert_eq!(
                linux_kobject_add(
                    &mut child,
                    &mut parent,
                    c"node-%02x".as_ptr(),
                    0x2a,
                    0,
                    0,
                    0,
                ),
                0
            );

            assert_eq!(child.parent, &mut parent as *mut _);
            assert_eq!(parent.kref, 2);
            assert!(child.state_flags & LINUX_KOBJ_STATE_IN_SYSFS != 0);
            assert!(!child.name.is_null());
            assert_eq!(crate::lib::string::c_strlen(child.name, 32), 7);
            linux_kobject_put(&mut child);
            assert_eq!(parent.kref, 1);
        }
    }

    #[test]
    fn linux_kobject_init_and_add_combines_init_name_and_parent() {
        unsafe {
            let mut parent = core::mem::zeroed::<LinuxKObject>();
            let mut child = core::mem::zeroed::<LinuxKObject>();
            let ktype_storage = LinuxKObjType {
                release: None,
                sysfs_ops: core::ptr::null(),
                default_groups: core::ptr::null(),
                child_ns_type: core::ptr::null(),
                namespace: core::ptr::null(),
                get_ownership: core::ptr::null(),
            };
            let ktype = core::ptr::addr_of!(ktype_storage).cast::<c_void>();
            linux_kobject_init(&mut parent, ktype);

            assert_eq!(
                linux_kobject_init_and_add(
                    &mut child,
                    ktype,
                    &mut parent,
                    c"combined-%u".as_ptr(),
                    7,
                    0,
                    0,
                    0,
                ),
                0
            );

            assert_eq!(child.parent, &mut parent as *mut _);
            assert_eq!(child.kref, 1);
            assert_eq!(child.ktype, ktype);
            assert!(child.state_flags & LINUX_KOBJ_STATE_INITIALIZED != 0);
            assert!(child.state_flags & LINUX_KOBJ_STATE_IN_SYSFS != 0);
            assert_eq!(parent.kref, 2);
            assert!(!child.name.is_null());
            linux_kobject_put(&mut child);
            assert_eq!(parent.kref, 1);
        }
    }
}
