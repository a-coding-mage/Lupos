//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs/anon_inodes.c
//! Minimal anon-inode file allocation for fd-backed kernel objects.
//!
//! Ref: `vendor/linux/fs/anon_inodes.c`.  Lupos keeps the object pointer as a
//! small registry token in `file.private`; each subsystem owns the registry.

extern crate alloc;

use alloc::collections::BTreeSet;
use core::ffi::{c_char, c_void};

use crate::fs::dcache::d_alloc;
use crate::fs::file::alloc_file;
use crate::fs::ops::{FileOps, NOOP_INODE_OPS};
use crate::fs::types::{FileRef, Inode, InodeKind, InodePrivate};
use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::include::uapi::fcntl::{O_ACCMODE, O_NONBLOCK};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};
use lazy_static::lazy_static;
use spin::Mutex;

const LINUX_FILE_F_MODE_OFFSET: usize = 4;
const LINUX_FILE_F_OP_OFFSET: usize = 8;
const LINUX_FILE_F_MAPPING_OFFSET: usize = 16;
const LINUX_FILE_PRIVATE_DATA_OFFSET: usize = 24;
const LINUX_FILE_F_INODE_OFFSET: usize = 32;
const LINUX_FILE_F_FLAGS_OFFSET: usize = 40;
const LINUX_FILE_SIZE: usize = 176;
const LINUX_INODE_I_MAPPING_OFFSET: usize = 48;
const LINUX_INODE_SIZE: usize = 544;
const LINUX_ADDRESS_SPACE_HOST_OFFSET: usize = 0;
const LINUX_ADDRESS_SPACE_SIZE: usize = 152;
const FMODE_OPENED: u32 = 1 << 19;
const FMODE_NONOTIFY: u32 = 1 << 25;

lazy_static! {
    static ref LINUX_STANDALONE_ANON_INODES: Mutex<BTreeSet<usize>> = Mutex::new(BTreeSet::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("alloc_anon_inode", linux_alloc_anon_inode as usize, false);
    export_symbol_once("iput", linux_iput as usize, false);
    export_symbol_once(
        "anon_inode_getfile",
        linux_anon_inode_getfile as usize,
        true,
    );
    export_symbol_once("anon_inode_getfd", linux_anon_inode_getfd as usize, true);
}

pub fn alloc_anon_file(name: &str, fops: &'static FileOps, token: usize) -> FileRef {
    alloc_anon_file_with_kind(name, fops, token, InodeKind::Socket, 0o600)
}

pub fn alloc_anon_file_with_kind(
    name: &str,
    fops: &'static FileOps,
    token: usize,
    kind: InodeKind,
    mode: u32,
) -> FileRef {
    alloc_anon_file_with_ino_and_kind(name, fops, token, token as u64, kind, mode)
}

pub fn alloc_anon_file_with_ino(
    name: &str,
    fops: &'static FileOps,
    token: usize,
    ino: u64,
) -> FileRef {
    alloc_anon_file_with_ino_and_kind(name, fops, token, ino, InodeKind::Socket, 0o600)
}

pub fn alloc_anon_file_with_ino_and_kind(
    name: &str,
    fops: &'static FileOps,
    token: usize,
    ino: u64,
    kind: InodeKind,
    mode: u32,
) -> FileRef {
    let dentry = d_alloc(name);
    let inode = Inode::new(
        ino,
        kind,
        mode,
        &NOOP_INODE_OPS,
        fops,
        InodePrivate::Opaque(token),
    );
    dentry.instantiate(inode);
    let file = alloc_file(dentry, 0, 0, fops);
    *file.private.lock() = token;
    file
}

fn err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as *mut c_void
}

fn is_err_ptr(ptr: *const c_void) -> bool {
    let value = ptr as usize;
    value >= usize::MAX - 4095 + 1
}

fn ptr_err(ptr: *const c_void) -> i32 {
    -((usize::MAX - ptr as usize + 1) as i32)
}

fn kzalloc(size: usize) -> *mut u8 {
    unsafe { crate::mm::slab::kmalloc(size, GFP_KERNEL | __GFP_ZERO) }
}

unsafe fn write_usize(addr: usize, value: usize) {
    unsafe { (addr as *mut usize).write(value) };
}

unsafe fn write_u32(addr: usize, value: u32) {
    unsafe { (addr as *mut u32).write(value) };
}

unsafe fn wire_linux_inode_mapping(inode: usize, mapping: usize) {
    unsafe {
        write_usize(inode + LINUX_INODE_I_MAPPING_OFFSET, mapping);
        write_usize(mapping + LINUX_ADDRESS_SPACE_HOST_OFFSET, inode);
    }
}

unsafe extern "C" fn linux_alloc_anon_inode(_sb: *mut c_void) -> *mut c_void {
    let block = kzalloc(LINUX_INODE_SIZE + LINUX_ADDRESS_SPACE_SIZE);
    if block.is_null() {
        return err_ptr(ENOMEM);
    }
    let inode = block as usize;
    let mapping = inode + LINUX_INODE_SIZE;
    unsafe {
        wire_linux_inode_mapping(inode, mapping);
    }
    LINUX_STANDALONE_ANON_INODES.lock().insert(inode);
    block.cast()
}

/// `iput` - `vendor/linux/fs/inode.c`.
///
/// Module callers pass Linux-layout `struct inode *` values. Lupos-owned VFS
/// inodes use `Arc<Inode>` internally, while `alloc_anon_inode` fabricates raw
/// C-layout anon inodes for vendor modules. Release only those standalone raw
/// blocks here; embedded anon-file inodes are owned by their containing
/// Linux-layout `struct file` block and are intentionally ignored by `fput`.
unsafe extern "C" fn linux_iput(inode: *mut c_void) {
    if inode.is_null() {
        return;
    }
    let key = inode as usize;
    if LINUX_STANDALONE_ANON_INODES.lock().remove(&key) {
        unsafe { crate::mm::slab::kfree(inode.cast()) };
    }
}

unsafe extern "C" fn linux_anon_inode_getfile(
    name: *const c_char,
    fops: *const c_void,
    priv_data: *mut c_void,
    flags: i32,
) -> *mut c_void {
    if name.is_null() || fops.is_null() {
        return err_ptr(EINVAL);
    }
    let block = kzalloc(LINUX_FILE_SIZE + LINUX_INODE_SIZE + LINUX_ADDRESS_SPACE_SIZE);
    if block.is_null() {
        return err_ptr(ENOMEM);
    }

    let file = block as usize;
    let inode = file + LINUX_FILE_SIZE;
    let mapping = inode + LINUX_INODE_SIZE;
    let open_flags = (flags as u32) & (O_ACCMODE | O_NONBLOCK);
    let f_mode = ((open_flags + 1) & O_ACCMODE) | FMODE_OPENED | FMODE_NONOTIFY;
    unsafe {
        write_u32(file + LINUX_FILE_F_MODE_OFFSET, f_mode);
        write_usize(file + LINUX_FILE_F_OP_OFFSET, fops as usize);
        write_usize(file + LINUX_FILE_F_MAPPING_OFFSET, mapping);
        write_usize(file + LINUX_FILE_PRIVATE_DATA_OFFSET, priv_data as usize);
        write_usize(file + LINUX_FILE_F_INODE_OFFSET, inode);
        write_u32(file + LINUX_FILE_F_FLAGS_OFFSET, open_flags);
        wire_linux_inode_mapping(inode, mapping);
    }
    file as *mut c_void
}

unsafe extern "C" fn linux_anon_inode_getfd(
    name: *const c_char,
    fops: *const c_void,
    priv_data: *mut c_void,
    flags: i32,
) -> i32 {
    let file = unsafe { linux_anon_inode_getfile(name, fops, priv_data, flags) };
    if is_err_ptr(file.cast_const()) {
        return ptr_err(file.cast_const());
    }

    let fd = unsafe { crate::fs::file::linux_get_unused_fd_flags(flags as u32) };
    if fd < 0 {
        unsafe { crate::fs::file::linux_fput(file) };
        return fd;
    }

    unsafe { crate::fs::file::linux_fd_install(fd as u32, file) };
    fd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::fcntl::O_NONBLOCK;

    #[test]
    fn anon_inode_getfd_matches_vendor_contract_and_installs_file() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/anon_inodes.c"
        ));
        assert!(source.contains("int anon_inode_getfd("));
        assert!(source.contains("EXPORT_SYMBOL_GPL(anon_inode_getfd);"));

        crate::fs::file::register_module_exports();
        register_module_exports();
        assert_eq!(
            find_symbol("anon_inode_getfd"),
            Some(linux_anon_inode_getfd as usize)
        );

        static NAME: &[u8] = b"[lupos-test]\0";
        let fops = 0x1234usize as *const c_void;
        let priv_data = 0x5678usize as *mut c_void;
        let fd = unsafe {
            linux_anon_inode_getfd(
                NAME.as_ptr().cast::<c_char>(),
                fops,
                priv_data,
                O_NONBLOCK as i32,
            )
        };
        assert!(fd >= 0);

        let file = unsafe { crate::fs::file::linux_fdget(fd as u32).word };
        assert_ne!(file, 0);
        unsafe {
            assert_eq!(
                ((file + LINUX_FILE_F_OP_OFFSET) as *const usize).read(),
                fops as usize
            );
            assert_eq!(
                ((file + LINUX_FILE_PRIVATE_DATA_OFFSET) as *const usize).read(),
                priv_data as usize
            );
            assert_eq!(
                ((file + LINUX_FILE_F_FLAGS_OFFSET) as *const u32).read(),
                O_NONBLOCK
            );
            crate::fs::file::linux_put_unused_fd(fd as u32);
        }
    }

    #[test]
    fn anon_inode_getfd_rejects_invalid_arguments() {
        assert_eq!(
            unsafe {
                linux_anon_inode_getfd(
                    core::ptr::null(),
                    0x1234usize as *const c_void,
                    core::ptr::null_mut(),
                    0,
                )
            },
            -EINVAL
        );
    }
}
