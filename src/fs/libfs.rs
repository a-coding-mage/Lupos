//! linux-parity: partial
//! linux-source: vendor/linux/fs/libfs.c
//! Generic filesystem helpers — ports of `vendor/linux/fs/libfs.c`.
//!
//! `simple_*` routines that any in-memory filesystem (ramfs, tmpfs, debugfs,
//! kernfs) can wire into its op vtable.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::sync::atomic::Ordering;

use spin::Mutex;

use crate::include::uapi::errno::{
    EACCES, EFAULT, EFBIG, EINVAL, EISDIR, ENOENT, ENOMEM, ENOSYS, ENOTEMPTY, EPERM, ERANGE, EROFS,
};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};
use crate::mm::slab::{kfree, kmalloc};

use super::types::{FileRef, Inode, InodeKind, InodePrivate, InodeRef, touch_inode_now};

const LINUX_FILE_PRIVATE_DATA_OFFSET: usize = 24;
const LINUX_INODE_I_PRIVATE_OFFSET: usize = 536;
const LINUX_VFSMOUNT_MNT_SB_OFFSET: usize = 8;
const LINUX_VFSMOUNT_STANDIN_SIZE: usize = 256;
const LINUX_SUPER_BLOCK_STANDIN_SIZE: usize = 512;
const LINUX_FS_CONTEXT_OPS_OFFSET: usize = 0;
const LINUX_FS_CONTEXT_FS_PRIVATE_OFFSET: usize = 40;
const LINUX_FS_CONTEXT_SB_FLAGS_OFFSET: usize = 128;
const LINUX_FS_CONTEXT_S_IFLAGS_OFFSET: usize = 136;
const LINUX_FS_CONTEXT_GLOBAL_BYTE_OFFSET: usize = 142;
const LINUX_FS_CONTEXT_GLOBAL_BIT: u8 = 0x02;
const LINUX_SIMPLE_ATTR_BUF_SIZE: usize = 24;
const SB_NOUSER: u32 = 1 << 31;
const SB_I_NOEXEC: u32 = 0x0000_0002;
const SB_I_NODEV: u32 = 0x0000_0004;
const SIMPLE_ATTR_DEFAULT_FMT: &[u8] = b"%llu\n\0";

type LinuxSimpleAttrGet = unsafe extern "C" fn(*mut c_void, *mut u64) -> i32;
type LinuxSimpleAttrSet = unsafe extern "C" fn(*mut c_void, u64) -> i32;

#[repr(C)]
pub struct LinuxPseudoFsContext {
    ops: *const c_void,
    eops: *const c_void,
    xattr: *const *const c_void,
    dops: *const c_void,
    magic: usize,
    s_d_flags: u32,
}

#[repr(C)]
struct LinuxFsContextOperations {
    free: Option<unsafe extern "C" fn(*mut c_void)>,
    dup: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    parse_param: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    parse_monolithic: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    get_tree: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    reconfigure: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
}

#[repr(C)]
struct LinuxSimpleAttr {
    get: Option<LinuxSimpleAttrGet>,
    set: Option<LinuxSimpleAttrSet>,
    data: *mut c_void,
    fmt: *const c_char,
    get_buf: [u8; LINUX_SIMPLE_ATTR_BUF_SIZE],
    set_buf: [u8; LINUX_SIMPLE_ATTR_BUF_SIZE],
}

static PSEUDO_FS_CONTEXT_OPS: LinuxFsContextOperations = LinuxFsContextOperations {
    free: Some(linux_pseudo_fs_free),
    dup: None,
    parse_param: None,
    parse_monolithic: None,
    get_tree: Some(linux_pseudo_fs_get_tree),
    reconfigure: None,
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("init_pseudo", linux_init_pseudo as usize, false);
    export_symbol_once("simple_pin_fs", linux_simple_pin_fs as usize, false);
    export_symbol_once("simple_release_fs", linux_simple_release_fs as usize, false);
    export_symbol_once(
        "simple_read_from_buffer",
        linux_simple_read_from_buffer as usize,
        false,
    );
    export_symbol_once("simple_open", linux_simple_open as usize, false);
    export_symbol_once("simple_attr_open", linux_simple_attr_open as usize, true);
    export_symbol_once(
        "simple_attr_release",
        linux_simple_attr_release as usize,
        true,
    );
    export_symbol_once("simple_attr_read", linux_simple_attr_read as usize, true);
    export_symbol_once("simple_attr_write", linux_simple_attr_write as usize, true);
    export_symbol_once(
        "simple_attr_write_signed",
        linux_simple_attr_write_signed as usize,
        true,
    );
}

unsafe fn read_usize(addr: usize) -> usize {
    unsafe { (addr as *const usize).read() }
}

unsafe fn write_usize(addr: usize, value: usize) {
    unsafe { (addr as *mut usize).write(value) };
}

unsafe fn read_u32(addr: usize) -> u32 {
    unsafe { (addr as *const u32).read() }
}

unsafe fn write_u32(addr: usize, value: u32) {
    unsafe { (addr as *mut u32).write(value) };
}

unsafe fn read_u8(addr: usize) -> u8 {
    unsafe { (addr as *const u8).read() }
}

unsafe fn write_u8(addr: usize, value: u8) {
    unsafe { (addr as *mut u8).write(value) };
}

/// `init_pseudo` - `vendor/linux/fs/libfs.c`.
pub unsafe extern "C" fn linux_init_pseudo(
    fc: *mut c_void,
    magic: usize,
) -> *mut LinuxPseudoFsContext {
    if fc.is_null() {
        return core::ptr::null_mut();
    }

    let ctx = unsafe {
        kmalloc(size_of::<LinuxPseudoFsContext>(), GFP_KERNEL | __GFP_ZERO)
            as *mut LinuxPseudoFsContext
    };
    if ctx.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        (*ctx).magic = magic;
    }

    let fc = fc as usize;
    unsafe {
        write_usize(LINUX_FS_CONTEXT_FS_PRIVATE_OFFSET + fc, ctx as usize);
        write_usize(
            LINUX_FS_CONTEXT_OPS_OFFSET + fc,
            core::ptr::addr_of!(PSEUDO_FS_CONTEXT_OPS) as usize,
        );

        let sb_flags = read_u32(LINUX_FS_CONTEXT_SB_FLAGS_OFFSET + fc);
        write_u32(LINUX_FS_CONTEXT_SB_FLAGS_OFFSET + fc, sb_flags | SB_NOUSER);

        let s_iflags = read_u32(LINUX_FS_CONTEXT_S_IFLAGS_OFFSET + fc);
        write_u32(
            LINUX_FS_CONTEXT_S_IFLAGS_OFFSET + fc,
            s_iflags | SB_I_NOEXEC | SB_I_NODEV,
        );

        let global = read_u8(LINUX_FS_CONTEXT_GLOBAL_BYTE_OFFSET + fc);
        write_u8(
            LINUX_FS_CONTEXT_GLOBAL_BYTE_OFFSET + fc,
            global | LINUX_FS_CONTEXT_GLOBAL_BIT,
        );
    }

    ctx
}

unsafe extern "C" fn linux_pseudo_fs_free(fc: *mut c_void) {
    if fc.is_null() {
        return;
    }
    let private_addr = fc as usize + LINUX_FS_CONTEXT_FS_PRIVATE_OFFSET;
    let private = unsafe { read_usize(private_addr) };
    if private != 0 {
        unsafe {
            kfree(private as *mut u8);
            write_usize(private_addr, 0);
        }
    }
}

unsafe extern "C" fn linux_pseudo_fs_get_tree(_fc: *mut c_void) -> i32 {
    -ENOSYS
}

/// `simple_pin_fs` - `vendor/linux/fs/libfs.c`.
pub unsafe extern "C" fn linux_simple_pin_fs(
    _fs_type: *mut c_void,
    mount: *mut usize,
    count: *mut i32,
) -> i32 {
    if mount.is_null() {
        return -EINVAL;
    }

    unsafe {
        if *mount == 0 {
            let block = kmalloc(
                LINUX_VFSMOUNT_STANDIN_SIZE + LINUX_SUPER_BLOCK_STANDIN_SIZE,
                GFP_KERNEL | __GFP_ZERO,
            );
            if block.is_null() {
                return -ENOMEM;
            }
            let mnt = block as usize;
            let sb = mnt + LINUX_VFSMOUNT_STANDIN_SIZE;
            write_usize(mnt + LINUX_VFSMOUNT_MNT_SB_OFFSET, sb);
            *mount = mnt;
        }
        if !count.is_null() {
            *count += 1;
        }
    }

    0
}

/// `simple_release_fs` - `vendor/linux/fs/libfs.c`.
pub unsafe extern "C" fn linux_simple_release_fs(mount: *mut usize, count: *mut i32) {
    unsafe {
        if !count.is_null() {
            *count -= 1;
            if *count <= 0 {
                *count = 0;
                if !mount.is_null() {
                    *mount = 0;
                }
            }
        }
    }
}

/// `simple_open` - `vendor/linux/fs/libfs.c`.
pub unsafe extern "C" fn linux_simple_open(inode: *mut c_void, file: *mut c_void) -> i32 {
    if inode.is_null() || file.is_null() {
        return 0;
    }
    let inode = inode as usize;
    let file = file as usize;
    let private = unsafe { read_usize(inode + LINUX_INODE_I_PRIVATE_OFFSET) };
    if private != 0 {
        unsafe { write_usize(file + LINUX_FILE_PRIVATE_DATA_OFFSET, private) };
    }
    0
}

fn digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some((byte - b'0') as u32),
        b'a'..=b'z' => Some((byte - b'a') as u32 + 10),
        b'A'..=b'Z' => Some((byte - b'A') as u32 + 10),
        _ => None,
    }
}

fn trim_one_trailing_newline(bytes: &[u8]) -> &[u8] {
    match bytes.last() {
        Some(b'\n') => &bytes[..bytes.len() - 1],
        _ => bytes,
    }
}

fn parse_simple_attr_u64(mut bytes: &[u8], mut base: u32) -> Result<u64, i32> {
    bytes = trim_one_trailing_newline(bytes);
    if bytes.is_empty() || (base != 0 && !(2..=16).contains(&base)) {
        return Err(-EINVAL);
    }
    if bytes[0] == b'+' {
        bytes = &bytes[1..];
    }
    if bytes.is_empty() {
        return Err(-EINVAL);
    }

    if base == 0 {
        base = if bytes.len() >= 3
            && bytes[0] == b'0'
            && matches!(bytes[1], b'x' | b'X')
            && digit_value(bytes[2]).is_some()
        {
            16
        } else if bytes[0] == b'0' {
            8
        } else {
            10
        };
    }
    if base == 16 && bytes.len() >= 2 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'X') {
        bytes = &bytes[2..];
    }

    let mut value = 0u64;
    let mut digits = 0usize;
    for byte in bytes {
        let Some(digit) = digit_value(*byte) else {
            return Err(-EINVAL);
        };
        if digit >= base {
            return Err(-EINVAL);
        }
        value = value
            .checked_mul(base as u64)
            .and_then(|value| value.checked_add(digit as u64))
            .ok_or(-ERANGE)?;
        digits += 1;
    }
    if digits == 0 {
        return Err(-EINVAL);
    }
    Ok(value)
}

fn parse_simple_attr_i64(bytes: &[u8], base: u32) -> Result<u64, i32> {
    let bytes = trim_one_trailing_newline(bytes);
    if let Some(rest) = bytes.strip_prefix(b"-") {
        let value = parse_simple_attr_u64(rest, base)?;
        if value > i64::MAX as u64 + 1 {
            return Err(-ERANGE);
        }
        if value == i64::MAX as u64 + 1 {
            Ok(i64::MIN as u64)
        } else {
            Ok((-(value as i64)) as u64)
        }
    } else {
        let value = parse_simple_attr_u64(bytes, base)?;
        if value > i64::MAX as u64 {
            Err(-ERANGE)
        } else {
            Ok(value)
        }
    }
}

unsafe fn simple_attr_from_file(file: *mut c_void) -> *mut LinuxSimpleAttr {
    if file.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { read_usize(file as usize + LINUX_FILE_PRIVATE_DATA_OFFSET) as *mut LinuxSimpleAttr }
}

/// `simple_attr_open` - `vendor/linux/fs/libfs.c:1316`.
pub unsafe extern "C" fn linux_simple_attr_open(
    inode: *mut c_void,
    file: *mut c_void,
    get: Option<LinuxSimpleAttrGet>,
    set: Option<LinuxSimpleAttrSet>,
    fmt: *const c_char,
) -> i32 {
    if inode.is_null() || file.is_null() {
        return -EINVAL;
    }

    let attr = unsafe {
        kmalloc(size_of::<LinuxSimpleAttr>(), GFP_KERNEL | __GFP_ZERO) as *mut LinuxSimpleAttr
    };
    if attr.is_null() {
        return -ENOMEM;
    }

    let data = unsafe { read_usize(inode as usize + LINUX_INODE_I_PRIVATE_OFFSET) as *mut c_void };
    unsafe {
        attr.write(LinuxSimpleAttr {
            get,
            set,
            data,
            fmt: if fmt.is_null() {
                SIMPLE_ATTR_DEFAULT_FMT.as_ptr().cast()
            } else {
                fmt
            },
            get_buf: [0; LINUX_SIMPLE_ATTR_BUF_SIZE],
            set_buf: [0; LINUX_SIMPLE_ATTR_BUF_SIZE],
        });
        write_usize(
            file as usize + LINUX_FILE_PRIVATE_DATA_OFFSET,
            attr as usize,
        );
    }

    0
}

/// `simple_attr_release` - `vendor/linux/fs/libfs.c:1338`.
pub unsafe extern "C" fn linux_simple_attr_release(_inode: *mut c_void, file: *mut c_void) -> i32 {
    let attr = unsafe { simple_attr_from_file(file) };
    if !attr.is_null() {
        unsafe {
            kfree(attr.cast());
            write_usize(file as usize + LINUX_FILE_PRIVATE_DATA_OFFSET, 0);
        }
    }
    0
}

/// `simple_attr_read` - `vendor/linux/fs/libfs.c:1346`.
pub unsafe extern "C" fn linux_simple_attr_read(
    file: *mut c_void,
    buf: *mut c_char,
    len: usize,
    ppos: *mut i64,
) -> isize {
    if ppos.is_null() {
        return -(EINVAL as isize);
    }
    let attr = unsafe { simple_attr_from_file(file) };
    if attr.is_null() {
        return -(EINVAL as isize);
    }
    let attr = unsafe { &mut *attr };
    let Some(get) = attr.get else {
        return -(EACCES as isize);
    };

    let size = if unsafe { *ppos } != 0 && attr.get_buf[0] != 0 {
        unsafe { crate::lib::string::c_strlen(attr.get_buf.as_ptr().cast(), attr.get_buf.len()) }
    } else {
        let mut value = 0u64;
        let ret = unsafe { get(attr.data, &mut value) };
        if ret != 0 {
            return ret as isize;
        }
        let args = [value as usize];
        let stack = [0usize];
        unsafe {
            crate::linux_driver_abi::base::printf::vscnprintf_n(
                attr.get_buf.as_mut_ptr(),
                attr.get_buf.len(),
                attr.fmt,
                args.as_ptr(),
                args.len(),
                stack.as_ptr(),
            )
        }
    };

    unsafe {
        linux_simple_read_from_buffer(buf.cast(), len, ppos, attr.get_buf.as_ptr().cast(), size)
    }
}

unsafe fn linux_simple_attr_write_xsigned(
    file: *mut c_void,
    buf: *const c_char,
    len: usize,
    _ppos: *mut i64,
    is_signed: bool,
) -> isize {
    let attr = unsafe { simple_attr_from_file(file) };
    if attr.is_null() {
        return -(EINVAL as isize);
    }
    let attr = unsafe { &mut *attr };
    let Some(set) = attr.set else {
        return -(EACCES as isize);
    };

    let size = core::cmp::min(attr.set_buf.len() - 1, len);
    let not_copied = unsafe {
        crate::lib::usercopy::_copy_from_user(attr.set_buf.as_mut_ptr(), buf.cast(), size)
    };
    if not_copied != 0 {
        return -(EFAULT as isize);
    }
    attr.set_buf[size] = 0;

    let bytes = &attr.set_buf[..size];
    let value = match if is_signed {
        parse_simple_attr_i64(bytes, 0)
    } else {
        parse_simple_attr_u64(bytes, 0)
    } {
        Ok(value) => value,
        Err(err) => return err as isize,
    };
    let ret = unsafe { set(attr.data, value) };
    if ret == 0 { len as isize } else { ret as isize }
}

/// `simple_attr_write` - `vendor/linux/fs/libfs.c:1420`.
pub unsafe extern "C" fn linux_simple_attr_write(
    file: *mut c_void,
    buf: *const c_char,
    len: usize,
    ppos: *mut i64,
) -> isize {
    unsafe { linux_simple_attr_write_xsigned(file, buf, len, ppos, false) }
}

/// `simple_attr_write_signed` - `vendor/linux/fs/libfs.c:1427`.
pub unsafe extern "C" fn linux_simple_attr_write_signed(
    file: *mut c_void,
    buf: *const c_char,
    len: usize,
    ppos: *mut i64,
) -> isize {
    unsafe { linux_simple_attr_write_xsigned(file, buf, len, ppos, true) }
}

/// `simple_read_from_buffer` - `vendor/linux/fs/libfs.c`.
pub unsafe extern "C" fn linux_simple_read_from_buffer(
    to: *mut c_void,
    count: usize,
    ppos: *mut i64,
    from: *const c_void,
    available: usize,
) -> isize {
    if ppos.is_null() {
        return -(EINVAL as isize);
    }
    let pos = unsafe { *ppos };
    if pos < 0 {
        return -(EINVAL as isize);
    }
    let pos = pos as usize;
    if pos >= available || count == 0 {
        return 0;
    }

    let count = count.min(available - pos);
    if from.is_null() {
        return -(EFAULT as isize);
    }

    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(
            to.cast::<u8>(),
            from.cast::<u8>().add(pos),
            count,
        )
    };
    if not_copied == count {
        return -(EFAULT as isize);
    }

    let copied = count - not_copied;
    unsafe {
        *ppos += copied as i64;
    }
    copied as isize
}

/// `simple_lookup` — search the in-memory `RamDir` table.
pub fn simple_lookup(dir: &InodeRef, name: &str) -> Result<InodeRef, i32> {
    let map = match &dir.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    map.lock()
        .iter()
        .find(|(child_name, _)| names_eq(child_name.as_str(), name))
        .map(|(_, inode)| inode.clone())
        .ok_or(ENOENT)
}

/// `simple_unlink` — remove a non-directory entry from a `RamDir`.
pub fn simple_unlink(dir: &InodeRef, name: &str) -> Result<(), i32> {
    let map = match &dir.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    let mut g = map.lock();
    let key = g
        .keys()
        .find(|child_name| names_eq(child_name.as_str(), name))
        .cloned()
        .ok_or(ENOENT)?;
    let child = g.get(&key).cloned().ok_or(ENOENT)?;
    if child.kind == InodeKind::Directory {
        return Err(EISDIR);
    }
    g.remove(&key);
    let nlink = child.nlink.fetch_sub(1, Ordering::AcqRel);
    drop(nlink);
    touch_inode_now(dir);
    touch_inode_now(&child);
    Ok(())
}

/// `simple_rmdir` — remove an empty directory entry from a `RamDir`.
pub fn simple_rmdir(dir: &InodeRef, name: &str) -> Result<(), i32> {
    let map = match &dir.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    let mut g = map.lock();
    let key = g
        .keys()
        .find(|child_name| names_eq(child_name.as_str(), name))
        .cloned()
        .ok_or(ENOENT)?;
    let child = g.get(&key).cloned().ok_or(ENOENT)?;
    if child.kind != InodeKind::Directory {
        return Err(ENOSYS);
    }
    if let InodePrivate::RamDir(cm) = &child.private {
        if !cm.lock().is_empty() {
            return Err(ENOTEMPTY);
        }
    }
    g.remove(&key);
    child.nlink.store(0, Ordering::Release);
    let parent_links = dir.nlink.load(Ordering::Acquire);
    if parent_links > 0 {
        dir.nlink.fetch_sub(1, Ordering::AcqRel);
    }
    touch_inode_now(dir);
    touch_inode_now(&child);
    Ok(())
}

fn names_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Emit Linux's synthetic `.` / `..` directory entries for in-memory
/// filesystem iterators.  `file.private` mirrors `dir_context::pos`.
pub fn synthetic_readdir_dot_entry(
    file: &FileRef,
) -> Result<Option<(String, u64, InodeKind)>, i32> {
    let mut pos = file.private.lock();
    let name = match *pos {
        0 => ".",
        1 => "..",
        _ => return Ok(None),
    };
    let ino = if *pos == 0 {
        file.inode().ok_or(EINVAL)?.ino
    } else {
        let parent = file
            .dentry
            .parent
            .lock()
            .clone()
            .unwrap_or_else(|| file.dentry.clone());
        parent.inode().or_else(|| file.inode()).ok_or(EINVAL)?.ino
    };
    *pos += 1;
    Ok(Some((String::from(name), ino, InodeKind::Directory)))
}

/// Generic readdir cursor — `file.private` holds a Linux-style directory
/// position: 0/1 for dot entries, 2+ for entries in the BTreeMap.
pub fn simple_readdir(file: &FileRef) -> Result<Option<(String, u64, InodeKind)>, i32> {
    if let Some(dot) = synthetic_readdir_dot_entry(file)? {
        return Ok(Some(dot));
    }
    let inode = file.inode().ok_or(EINVAL)?;
    let map = match &inode.private {
        InodePrivate::RamDir(m) => m,
        _ => return Err(EINVAL),
    };
    let mut idx = file.private.lock();
    let g = map.lock();
    let child_idx = idx.saturating_sub(2);
    if child_idx >= g.len() {
        return Ok(None);
    }
    let (k, v) = g.iter().nth(child_idx).unwrap();
    let out = (k.clone(), v.ino, v.kind);
    *idx += 1;
    Ok(Some(out))
}

/// Generic ramfs-style read from `RamBytes`.
pub fn ram_file_read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    let n = match &inode.private {
        InodePrivate::RamBytes(m) => {
            let g = m.lock();
            let logical_len = inode.size.load(Ordering::Acquire) as usize;
            let start = (*pos as usize).min(logical_len);
            let n = (logical_len - start).min(buf.len());
            let materialized = if start < g.len() {
                let bytes = (g.len() - start).min(n);
                buf[..bytes].copy_from_slice(&g[start..start + bytes]);
                bytes
            } else {
                0
            };
            if materialized < n {
                buf[materialized..n].fill(0);
            }
            n
        }
        InodePrivate::StaticBytes(bytes) => {
            let start = (*pos as usize).min(bytes.len());
            let n = (bytes.len() - start).min(buf.len());
            buf[..n].copy_from_slice(&bytes[start..start + n]);
            n
        }
        InodePrivate::StaticCowBytes { base, overlay } => {
            let logical_len = inode.size.load(Ordering::Acquire) as usize;
            let start = (*pos as usize).min(logical_len);
            let n = (logical_len - start).min(buf.len());
            if let Some(bytes) = overlay.lock().as_ref() {
                let materialized = if start < bytes.len() {
                    let bytes_to_copy = (bytes.len() - start).min(n);
                    buf[..bytes_to_copy].copy_from_slice(&bytes[start..start + bytes_to_copy]);
                    bytes_to_copy
                } else {
                    0
                };
                if materialized < n {
                    buf[materialized..n].fill(0);
                }
            } else {
                let base_len = base.len().min(logical_len);
                let copied = if start < base_len {
                    let bytes_to_copy = (base_len - start).min(n);
                    buf[..bytes_to_copy].copy_from_slice(&base[start..start + bytes_to_copy]);
                    bytes_to_copy
                } else {
                    0
                };
                if copied < n {
                    buf[copied..n].fill(0);
                }
            }
            n
        }
        _ => return Err(EINVAL),
    };
    *pos += n as u64;
    Ok(n)
}

/// Generic ramfs-style write into `RamBytes`.
pub fn ram_file_write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let inode = file.inode().ok_or(EINVAL)?;
    match &inode.private {
        InodePrivate::RamBytes(m) => {
            let mut g = m.lock();
            write_into_vec(&mut g, &inode, buf, pos)
        }
        InodePrivate::StaticCowBytes { base, overlay } => {
            let mut maybe_overlay = overlay.lock();
            if maybe_overlay.is_none() {
                let mut materialized = Vec::new();
                materialized.try_reserve(base.len()).map_err(|_| ENOMEM)?;
                materialized.extend_from_slice(base);
                *maybe_overlay = Some(materialized);
            }
            let bytes = maybe_overlay.as_mut().ok_or(EINVAL)?;
            write_into_vec(bytes, &inode, buf, pos)
        }
        InodePrivate::StaticBytes(_) => return Err(EROFS),
        _ => return Err(EINVAL),
    }
}

fn write_into_vec(
    g: &mut Vec<u8>,
    inode: &InodeRef,
    buf: &[u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    let p = *pos as usize;
    let end = p.checked_add(buf.len()).ok_or(EINVAL)?;
    if g.len() < end {
        let additional = end - g.len();
        g.try_reserve(additional).map_err(|_| ENOMEM)?;
        g.resize(end, 0);
    }
    g[p..end].copy_from_slice(buf);
    *pos += buf.len() as u64;
    let logical_len = inode.size.load(Ordering::Acquire).max(end as u64);
    inode.size.store(logical_len, Ordering::Release);
    touch_inode_now(inode);
    Ok(buf.len())
}

/// Update the logical size of a ramfs/tmpfs byte file without eagerly
/// materializing holes. Linux grows files through the page cache a page at a
/// time; a single contiguous `Vec` is only our compact representation for data
/// that has actually been written.
pub fn ram_file_set_size(inode: &InodeRef, size: u64) -> Result<(), i32> {
    if size > usize::MAX as u64 {
        return Err(EFBIG);
    }
    let new_len = size as usize;
    match &inode.private {
        InodePrivate::RamBytes(bytes) => {
            let mut g = bytes.lock();
            if new_len < g.len() {
                g.truncate(new_len);
            }
        }
        InodePrivate::StaticCowBytes { overlay, .. } => {
            if let Some(bytes) = overlay.lock().as_mut() {
                if new_len < bytes.len() {
                    bytes.truncate(new_len);
                }
            }
        }
        _ => {}
    }
    inode.size.store(size, Ordering::Release);
    touch_inode_now(inode);
    Ok(())
}

pub fn ram_file_zero_range(
    inode: &InodeRef,
    offset: u64,
    len: u64,
    keep_size: bool,
) -> Result<(), i32> {
    let end = offset.checked_add(len).ok_or(EINVAL)?;
    if !keep_size && end > inode.size.load(Ordering::Acquire) {
        ram_file_set_size(inode, end)?;
    }
    let start = offset.min(usize::MAX as u64) as usize;
    let end = end.min(usize::MAX as u64) as usize;
    if start >= end {
        return Ok(());
    }
    match &inode.private {
        InodePrivate::RamBytes(bytes) => {
            let mut g = bytes.lock();
            let zero_end = end.min(g.len());
            if start < zero_end {
                g[start..zero_end].fill(0);
            }
        }
        InodePrivate::StaticCowBytes { base, overlay } => {
            let mut maybe_overlay = overlay.lock();
            if maybe_overlay.is_none() {
                let mut materialized = Vec::new();
                materialized.try_reserve(base.len()).map_err(|_| ENOMEM)?;
                materialized.extend_from_slice(base);
                *maybe_overlay = Some(materialized);
            }
            if let Some(bytes) = maybe_overlay.as_mut() {
                let zero_end = end.min(bytes.len());
                if start < zero_end {
                    bytes[start..zero_end].fill(0);
                }
            }
        }
        _ => {}
    }
    touch_inode_now(inode);
    Ok(())
}

/// Build an empty `InodePrivate::RamDir`.
pub fn empty_ram_dir() -> InodePrivate {
    InodePrivate::RamDir(Mutex::new(BTreeMap::new()))
}

/// Build an empty `InodePrivate::RamBytes`.
pub fn empty_ram_bytes() -> InodePrivate {
    InodePrivate::RamBytes(Mutex::new(Vec::new()))
}

/// Build copy-on-write ramfs bytes backed by the installed initramfs image.
pub fn static_cow_bytes(base: &'static [u8]) -> InodePrivate {
    InodePrivate::StaticCowBytes {
        base,
        overlay: Mutex::new(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU64, Ordering};

    unsafe extern "C" fn simple_attr_test_set(data: *mut c_void, value: u64) -> i32 {
        if data.is_null() {
            return -EINVAL;
        }
        unsafe { (*(data as *const AtomicU64)).store(value, Ordering::Release) };
        0
    }

    #[test]
    fn simple_attr_exports_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/libfs.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL_GPL(simple_attr_open);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(simple_attr_release);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(simple_attr_read);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(simple_attr_write);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(simple_attr_write_signed);"));

        register_module_exports();
        assert_eq!(
            find_symbol("simple_attr_open"),
            Some(linux_simple_attr_open as usize)
        );
        assert_eq!(
            find_symbol("simple_attr_release"),
            Some(linux_simple_attr_release as usize)
        );
        assert_eq!(
            find_symbol("simple_attr_read"),
            Some(linux_simple_attr_read as usize)
        );
        assert_eq!(
            find_symbol("simple_attr_write"),
            Some(linux_simple_attr_write as usize)
        );
        assert_eq!(
            find_symbol("simple_attr_write_signed"),
            Some(linux_simple_attr_write_signed as usize)
        );
    }

    #[test]
    fn simple_read_from_buffer_copies_and_advances_position() {
        let source = *b"abcdef";
        let mut out = [0u8; 4];
        let mut pos = 2i64;

        let ret = unsafe {
            linux_simple_read_from_buffer(
                out.as_mut_ptr().cast(),
                out.len(),
                core::ptr::addr_of_mut!(pos),
                source.as_ptr().cast(),
                source.len(),
            )
        };

        assert_eq!(ret, 4);
        assert_eq!(&out, b"cdef");
        assert_eq!(pos, 6);
        assert_eq!(
            unsafe {
                linux_simple_read_from_buffer(
                    out.as_mut_ptr().cast(),
                    out.len(),
                    core::ptr::addr_of_mut!(pos),
                    source.as_ptr().cast(),
                    source.len(),
                )
            },
            0
        );
    }

    #[test]
    fn simple_attr_write_parses_value_and_calls_setter() {
        let stored = AtomicU64::new(0);
        let mut attr = LinuxSimpleAttr {
            get: None,
            set: Some(simple_attr_test_set),
            data: (&stored as *const AtomicU64).cast_mut().cast(),
            fmt: SIMPLE_ATTR_DEFAULT_FMT.as_ptr().cast(),
            get_buf: [0; LINUX_SIMPLE_ATTR_BUF_SIZE],
            set_buf: [0; LINUX_SIMPLE_ATTR_BUF_SIZE],
        };
        let mut file = [0u8; 64];
        unsafe {
            write_usize(
                file.as_mut_ptr() as usize + LINUX_FILE_PRIVATE_DATA_OFFSET,
                (&mut attr as *mut LinuxSimpleAttr) as usize,
            );
        }

        let input = b"0x2a\n";
        assert_eq!(
            unsafe {
                linux_simple_attr_write(
                    file.as_mut_ptr().cast(),
                    input.as_ptr().cast(),
                    input.len(),
                    core::ptr::null_mut(),
                )
            },
            input.len() as isize
        );
        assert_eq!(stored.load(Ordering::Acquire), 42);

        let signed = b"-7";
        assert_eq!(
            unsafe {
                linux_simple_attr_write_signed(
                    file.as_mut_ptr().cast(),
                    signed.as_ptr().cast(),
                    signed.len(),
                    core::ptr::null_mut(),
                )
            },
            signed.len() as isize
        );
        assert_eq!(stored.load(Ordering::Acquire), (-7i64) as u64);
    }

    #[test]
    fn simple_attr_write_rejects_missing_setter_and_bad_input() {
        let mut attr = LinuxSimpleAttr {
            get: None,
            set: None,
            data: core::ptr::null_mut(),
            fmt: SIMPLE_ATTR_DEFAULT_FMT.as_ptr().cast(),
            get_buf: [0; LINUX_SIMPLE_ATTR_BUF_SIZE],
            set_buf: [0; LINUX_SIMPLE_ATTR_BUF_SIZE],
        };
        let mut file = [0u8; 64];
        unsafe {
            write_usize(
                file.as_mut_ptr() as usize + LINUX_FILE_PRIVATE_DATA_OFFSET,
                (&mut attr as *mut LinuxSimpleAttr) as usize,
            );
        }

        assert_eq!(
            unsafe {
                linux_simple_attr_write(
                    file.as_mut_ptr().cast(),
                    b"1".as_ptr().cast(),
                    1,
                    core::ptr::null_mut(),
                )
            },
            -(EACCES as isize)
        );

        attr.set = Some(simple_attr_test_set);
        assert_eq!(
            unsafe {
                linux_simple_attr_write(
                    file.as_mut_ptr().cast(),
                    b"nope".as_ptr().cast(),
                    4,
                    core::ptr::null_mut(),
                )
            },
            -(EINVAL as isize)
        );
    }
}
