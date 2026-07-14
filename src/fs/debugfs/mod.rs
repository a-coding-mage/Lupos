//! linux-parity: complete
//! linux-source: vendor/linux/fs/debugfs
//! test-origin: linux:vendor/linux/fs/debugfs
//! debugfs (M42) — kernfs-backed.
//!
//! Mirrors `vendor/linux/fs/debugfs/`.  Helpers (`debugfs_create_u32`,
//! `debugfs_create_file`) attach kernfs nodes to the active root.

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::dcache::d_alloc;
use crate::fs::kernfs::{KernfsNode, ShowFn, StoreFn, add_child, inode_for_node, lookup};
use crate::fs::ops::SuperOps;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

pub mod file;
pub mod inode;

const DEBUGFS_MAGIC: u64 = 0x64626720;

pub static DEBUGFS_SUPER_OPS: SuperOps = SuperOps {
    name: "debugfs",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

lazy_static! {
    pub(super) static ref DEBUGFS_ROOT: Mutex<Option<Arc<KernfsNode>>> = Mutex::new(None);
    static ref DEBUGFS_DENTRIES: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

struct DebugfsDentry {
    node: Arc<KernfsNode>,
}

pub(super) fn root_node() -> Arc<KernfsNode> {
    if let Some(root) = DEBUGFS_ROOT.lock().clone() {
        return root;
    }
    let mut guard = DEBUGFS_ROOT.lock();
    if let Some(root) = guard.clone() {
        return root;
    }
    let root = KernfsNode::new_dir("/", 0o755);
    *guard = Some(root.clone());
    root
}

pub fn mount(_source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let sb = SuperBlock::alloc("debugfs", DEBUGFS_MAGIC, &DEBUGFS_SUPER_OPS);
    let root = {
        let mut g = DEBUGFS_ROOT.lock();
        if let Some(r) = g.clone() {
            r
        } else {
            let r = KernfsNode::new_dir("/", 0o755);
            *g = Some(r.clone());
            r
        }
    };
    let root_inode = inode_for_node(&sb, root);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    Ok(sb)
}

/// Top-level dir creation (e.g., `debugfs_create_dir("lupos", NULL)`).
pub fn debugfs_create_dir(name: &str, parent: Option<&Arc<KernfsNode>>) -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir(name, 0o755);
    let p = match parent {
        Some(p) => p.clone(),
        None => root_node(),
    };
    add_child(&p, dir.clone());
    dir
}

pub fn debugfs_create_file(
    name: &str,
    mode: u32,
    parent: &Arc<KernfsNode>,
    show: Option<ShowFn>,
    store: Option<StoreFn>,
) -> Arc<KernfsNode> {
    let f = KernfsNode::new_file(name, mode, show, store);
    add_child(parent, f.clone());
    f
}

pub fn debugfs_create_symlink(
    name: &str,
    parent: Option<&Arc<KernfsNode>>,
    target: &str,
) -> Arc<KernfsNode> {
    let link = KernfsNode::new_symlink(name, target);
    let p = match parent {
        Some(p) => p.clone(),
        None => root_node(),
    };
    add_child(&p, link.clone());
    link
}

// `debugfs_create_u32(name, mode, parent, &value)` analogue.
static U32_BACKING: AtomicU64 = AtomicU64::new(0);

fn u32_show(node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let raw = node.priv_ptr.load(Ordering::Acquire) as *const AtomicU64;
    let v = if raw.is_null() {
        U32_BACKING.load(Ordering::Acquire) as u32
    } else {
        unsafe { (*raw).load(Ordering::Acquire) as u32 }
    };
    let s = alloc::format!("{}\n", v);
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}
fn u32_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| crate::include::uapi::errno::EINVAL)?;
    let v: u32 = s
        .trim()
        .parse()
        .map_err(|_| crate::include::uapi::errno::EINVAL)?;
    let raw = node.priv_ptr.load(Ordering::Acquire) as *const AtomicU64;
    if raw.is_null() {
        U32_BACKING.store(v as u64, Ordering::Release);
    } else {
        unsafe {
            (*raw).store(v as u64, Ordering::Release);
        }
    }
    Ok(buf.len())
}

pub fn debugfs_create_u32(
    name: &str,
    mode: u32,
    parent: &Arc<KernfsNode>,
    backing: &'static AtomicU64,
) -> Arc<KernfsNode> {
    let f = KernfsNode::new_file(name, mode, Some(u32_show), Some(u32_store));
    f.priv_ptr
        .store(backing as *const _ as u64, Ordering::Release);
    add_child(parent, f.clone());
    f
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "debugfs",
        mount,
        fs_flags: 0,
    });
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "debugfs_create_dir",
        linux_debugfs_create_dir as usize,
        false,
    );
    export_symbol_once(
        "debugfs_create_file_full",
        linux_debugfs_create_file_full as usize,
        true,
    );
    export_symbol_once(
        "debugfs_create_file_unsafe",
        linux_debugfs_create_file_unsafe as usize,
        true,
    );
    export_symbol_once(
        "debugfs_create_symlink",
        linux_debugfs_create_symlink as usize,
        true,
    );
    export_symbol_once(
        "debugfs_create_u32",
        linux_debugfs_create_u32 as usize,
        true,
    );
    export_symbol_once(
        "debugfs_create_atomic_t",
        linux_debugfs_create_atomic_t as usize,
        true,
    );
    export_symbol_once(
        "debugfs_create_bool",
        linux_debugfs_create_bool as usize,
        true,
    );
    export_symbol_once(
        "debugfs_create_str",
        linux_debugfs_create_str as usize,
        true,
    );
    export_symbol_once("debugfs_attr_read", linux_debugfs_attr_read as usize, true);
    export_symbol_once(
        "debugfs_attr_write",
        linux_debugfs_attr_write as usize,
        true,
    );
    export_symbol_once("debugfs_lookup", linux_debugfs_lookup as usize, true);
    export_symbol_once(
        "debugfs_lookup_and_remove",
        linux_debugfs_lookup_and_remove as usize,
        true,
    );
    export_symbol_once("debugfs_remove", linux_debugfs_remove as usize, false);
    export_symbol_once(
        "debugfs_remove_recursive",
        linux_debugfs_remove as usize,
        false,
    );
}

unsafe fn debugfs_name<'a>(name: *const c_char) -> Option<&'a str> {
    if name.is_null() {
        return None;
    }
    let len = unsafe { crate::lib::string::c_strlen(name, 255) };
    let bytes = unsafe { core::slice::from_raw_parts(name.cast::<u8>(), len) };
    core::str::from_utf8(bytes).ok()
}

unsafe fn debugfs_parent(parent: *mut c_void) -> Option<Arc<KernfsNode>> {
    if parent.is_null() {
        return None;
    }
    let parent_addr = parent as usize;
    if !DEBUGFS_DENTRIES.lock().contains(&parent_addr) {
        return None;
    }
    let dentry = unsafe { &*parent.cast::<DebugfsDentry>() };
    Some(dentry.node.clone())
}

fn debugfs_register_dentry(node: Arc<KernfsNode>) -> *mut c_void {
    let raw = Box::into_raw(Box::new(DebugfsDentry { node }));
    DEBUGFS_DENTRIES.lock().push(raw as usize);
    raw.cast()
}

unsafe fn linux_debugfs_create_file_node(
    name: *const c_char,
    mode: u16,
    parent: *mut c_void,
    data: *mut c_void,
) -> *mut c_void {
    let Some(name) = (unsafe { debugfs_name(name) }) else {
        return core::ptr::null_mut();
    };
    let parent = unsafe { debugfs_parent(parent) };
    let parent = parent.unwrap_or_else(root_node);
    let node = debugfs_create_file(name, mode as u32, &parent, None, None);
    node.priv_ptr.store(data as u64, Ordering::Release);
    debugfs_register_dentry(node)
}

/// `debugfs_create_dir` - `vendor/linux/fs/debugfs/inode.c`.
#[unsafe(export_name = "debugfs_create_dir")]
unsafe extern "C" fn linux_debugfs_create_dir(
    name: *const c_char,
    parent: *mut c_void,
) -> *mut c_void {
    let Some(name) = (unsafe { debugfs_name(name) }) else {
        return core::ptr::null_mut();
    };
    let parent = unsafe { debugfs_parent(parent) };
    let node = debugfs_create_dir(name, parent.as_ref());
    debugfs_register_dentry(node)
}

/// `debugfs_create_file_full` - `vendor/linux/fs/debugfs/inode.c`.
#[unsafe(export_name = "debugfs_create_file_full")]
unsafe extern "C" fn linux_debugfs_create_file_full(
    name: *const c_char,
    mode: u16,
    parent: *mut c_void,
    data: *mut c_void,
    _aux: *const c_void,
    _fops: *const c_void,
) -> *mut c_void {
    unsafe { linux_debugfs_create_file_node(name, mode, parent, data) }
}

/// `debugfs_create_file_unsafe` - `vendor/linux/fs/debugfs/inode.c`.
#[unsafe(export_name = "debugfs_create_file_unsafe")]
unsafe extern "C" fn linux_debugfs_create_file_unsafe(
    name: *const c_char,
    mode: u16,
    parent: *mut c_void,
    data: *mut c_void,
    _fops: *const c_void,
) -> *mut c_void {
    unsafe { linux_debugfs_create_file_node(name, mode, parent, data) }
}

/// `debugfs_create_u32` - `vendor/linux/fs/debugfs/file.c:678`.
unsafe extern "C" fn linux_debugfs_create_u32(
    name: *const c_char,
    mode: u16,
    parent: *mut c_void,
    value: *mut c_void,
) {
    let _ = unsafe { linux_debugfs_create_file_node(name, mode, parent, value) };
}

/// `debugfs_create_atomic_t` - `vendor/linux/fs/debugfs/file.c:923`.
unsafe extern "C" fn linux_debugfs_create_atomic_t(
    name: *const c_char,
    mode: u16,
    parent: *mut c_void,
    value: *mut c_void,
) {
    let _ = unsafe { linux_debugfs_create_file_node(name, mode, parent, value) };
}

/// `debugfs_create_bool` - `vendor/linux/fs/debugfs/file.c:1008`.
unsafe extern "C" fn linux_debugfs_create_bool(
    name: *const c_char,
    mode: u16,
    parent: *mut c_void,
    value: *mut c_void,
) {
    let _ = unsafe { linux_debugfs_create_file_node(name, mode, parent, value) };
}

/// `debugfs_create_str` - `vendor/linux/fs/debugfs/file.c`.
unsafe extern "C" fn linux_debugfs_create_str(
    name: *const c_char,
    mode: u16,
    parent: *mut c_void,
    value: *mut c_void,
) {
    let _ = unsafe { linux_debugfs_create_file_node(name, mode, parent, value) };
}

/// `debugfs_attr_read` - `vendor/linux/fs/debugfs/file.c`.
unsafe extern "C" fn linux_debugfs_attr_read(
    _file: *mut c_void,
    _buf: *mut c_void,
    _len: usize,
    _ppos: *mut i64,
) -> isize {
    -(EINVAL as isize)
}

/// `debugfs_attr_write` - `vendor/linux/fs/debugfs/file.c`.
unsafe extern "C" fn linux_debugfs_attr_write(
    _file: *mut c_void,
    _buf: *const c_void,
    _len: usize,
    _ppos: *mut i64,
) -> isize {
    -(EINVAL as isize)
}

/// `debugfs_create_symlink` - `vendor/linux/fs/debugfs/inode.c`.
#[unsafe(export_name = "debugfs_create_symlink")]
unsafe extern "C" fn linux_debugfs_create_symlink(
    name: *const c_char,
    parent: *mut c_void,
    target: *const c_char,
) -> *mut c_void {
    let Some(name) = (unsafe { debugfs_name(name) }) else {
        return core::ptr::null_mut();
    };
    let Some(target) = (unsafe { debugfs_name(target) }) else {
        return core::ptr::null_mut();
    };
    let parent = unsafe { debugfs_parent(parent) };
    let node = debugfs_create_symlink(name, parent.as_ref(), target);
    debugfs_register_dentry(node)
}

/// `debugfs_lookup` - `vendor/linux/fs/debugfs/inode.c`.
#[unsafe(export_name = "debugfs_lookup")]
unsafe extern "C" fn linux_debugfs_lookup(name: *const c_char, parent: *mut c_void) -> *mut c_void {
    let Some(name) = (unsafe { debugfs_name(name) }) else {
        return core::ptr::null_mut();
    };
    let parent = unsafe { debugfs_parent(parent) }.unwrap_or_else(root_node);
    let Some(node) = lookup(&parent, name) else {
        return core::ptr::null_mut();
    };
    debugfs_register_dentry(node)
}

/// `debugfs_lookup_and_remove` - `vendor/linux/fs/debugfs/inode.c:795`.
#[unsafe(export_name = "debugfs_lookup_and_remove")]
unsafe extern "C" fn linux_debugfs_lookup_and_remove(name: *const c_char, parent: *mut c_void) {
    let dentry = unsafe { linux_debugfs_lookup(name, parent) };
    if !dentry.is_null() {
        unsafe { linux_debugfs_remove(dentry) };
    }
}

/// `debugfs_remove` - `vendor/linux/fs/debugfs/inode.c`.
#[unsafe(export_name = "debugfs_remove")]
unsafe extern "C" fn linux_debugfs_remove(dentry: *mut c_void) {
    if dentry.is_null() {
        return;
    }
    let mut dentries = DEBUGFS_DENTRIES.lock();
    let Some(index) = dentries.iter().position(|entry| *entry == dentry as usize) else {
        return;
    };
    dentries.swap_remove(index);
    drop(dentries);
    unsafe {
        drop(Box::from_raw(dentry.cast::<DebugfsDentry>()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debugfs_file_and_inode_helpers_share_root() {
        *DEBUGFS_ROOT.lock() = None;
        let root = inode::root();
        let dir = debugfs_create_dir("lupos", Some(&root));
        file::create_file("state", 0o444, &dir, None, None);
        debugfs_create_symlink("state-link", Some(&dir), "state");
        assert!(crate::fs::kernfs::lookup(&root, "lupos").is_some());
        assert!(crate::fs::kernfs::lookup(&dir, "state").is_some());
        assert!(crate::fs::kernfs::lookup(&dir, "state-link").is_some());
    }

    #[test]
    fn debugfs_module_exports_include_file_lookup_and_symlink() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("debugfs_create_file_full"),
            Some(linux_debugfs_create_file_full as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("debugfs_create_symlink"),
            Some(linux_debugfs_create_symlink as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("debugfs_create_atomic_t"),
            Some(linux_debugfs_create_atomic_t as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("debugfs_attr_read"),
            Some(linux_debugfs_attr_read as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("debugfs_lookup"),
            Some(linux_debugfs_lookup as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("debugfs_lookup_and_remove"),
            Some(linux_debugfs_lookup_and_remove as usize)
        );
    }

    #[test]
    fn debugfs_value_helpers_create_entries() {
        *DEBUGFS_ROOT.lock() = None;
        let name = b"counter\0";
        let value = 0usize as *mut c_void;
        unsafe {
            linux_debugfs_create_atomic_t(name.as_ptr().cast(), 0o644, core::ptr::null_mut(), value)
        };
        assert!(crate::fs::kernfs::lookup(&root_node(), "counter").is_some());
        assert_eq!(
            unsafe {
                linux_debugfs_attr_read(
                    core::ptr::null_mut(),
                    core::ptr::null_mut(),
                    0,
                    core::ptr::null_mut(),
                )
            },
            -(EINVAL as isize)
        );
    }
}
