//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module
//! test-origin: linux:vendor/linux/kernel/module
//! Module-loader syscall glue.
//!
//! Ref: `vendor/linux/kernel/module/main.c`.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::arch::x86::kernel::uaccess;
use crate::fs::types::{FileRef, InodeKind};
use crate::include::uapi::errno::{
    EBADF, EFAULT, EFBIG, EINVAL, EIO, ENOEXEC, ENOMEM, EOPNOTSUPP, EPERM,
};
use crate::include::uapi::fcntl::{O_ACCMODE, O_PATH, O_RDONLY, O_RDWR};
use crate::kernel::{
    capability::{CAP_SYS_MODULE, capable},
    files, sched,
};

use super::{LoadModuleError, delete_module, load_module};

const MODULE_INIT_IGNORE_MODVERSIONS: i32 = 1;
const MODULE_INIT_IGNORE_VERMAGIC: i32 = 2;
const MODULE_INIT_COMPRESSED_FILE: i32 = 4;
const MODULE_INIT_VALID_FLAGS: i32 =
    MODULE_INIT_IGNORE_MODVERSIONS | MODULE_INIT_IGNORE_VERMAGIC | MODULE_INIT_COMPRESSED_FILE;
const MAX_MODULE_FILE_SIZE: u64 = i32::MAX as u64;
const MAX_RW_COUNT: usize = 0x7fff_f000;

struct CopiedModuleImage {
    ptr: *mut u8,
    len: usize,
}

impl CopiedModuleImage {
    fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for CopiedModuleImage {
    fn drop(&mut self) {
        crate::mm::vmalloc::vfree(self.ptr);
    }
}

fn load_errno(err: LoadModuleError) -> i32 {
    match err {
        LoadModuleError::BadElf => crate::include::uapi::errno::ENOEXEC,
        LoadModuleError::AlreadyLoaded => crate::include::uapi::errno::EEXIST,
        LoadModuleError::UndefinedSymbol(_) => crate::include::uapi::errno::ENOENT,
        LoadModuleError::UnsupportedSection(_) | LoadModuleError::UnsupportedReloc => {
            crate::include::uapi::errno::ENOEXEC
        }
        LoadModuleError::OutOfMemory => crate::include::uapi::errno::ENOMEM,
        LoadModuleError::Invalid => EINVAL,
        LoadModuleError::InitFailed(errno) => errno.saturating_abs(),
    }
}

unsafe fn user_cstr(ptr: *const u8) -> Result<String, i32> {
    if ptr.is_null() {
        return Err(EFAULT);
    }
    const MAX: usize = 256;
    let bytes = unsafe { core::slice::from_raw_parts(ptr, MAX) };
    let end = bytes.iter().position(|b| *b == 0).ok_or(EINVAL)?;
    core::str::from_utf8(&bytes[..end])
        .map(String::from)
        .map_err(|_| EINVAL)
}

fn may_manage_modules() -> bool {
    capable(CAP_SYS_MODULE)
}

/// Selected-config `copy_module_from_user()` data path.
///
/// Vendor Linux rejects a shorter-than-ELF header before allocation, then
/// copies in 16-page chunks. Lupos still lacks the corresponding
/// `security_kernel_load_data` pre/post LSM hooks.
unsafe fn copy_module_from_user(
    module_image: *const u8,
    len: usize,
) -> Result<CopiedModuleImage, i32> {
    const ELF_HEADER_SIZE: usize = 64;
    const COPY_CHUNK_SIZE: usize = 16 * 4096;

    if len < ELF_HEADER_SIZE {
        return Err(ENOEXEC);
    }

    // Lupos vmalloc currently performs this page-rounding expression
    // internally without checked arithmetic; make its allocation failure
    // behavior safe for an arbitrary userspace length.
    len.checked_add(4095).ok_or(ENOMEM)?;
    let ptr = crate::mm::vmalloc::vmalloc(len);
    if ptr.is_null() {
        return Err(ENOMEM);
    }
    let image = CopiedModuleImage { ptr, len };

    let mut copied = 0usize;
    while copied < len {
        let count = (len - copied).min(COPY_CHUNK_SIZE);
        let not_copied = unsafe {
            uaccess::copy_from_user(
                image.ptr.add(copied),
                module_image.wrapping_add(copied),
                count,
            )
        };
        if not_copied != 0 {
            return Err(EFAULT);
        }
        copied += count;
    }
    Ok(image)
}

/// Selected-config `kernel_read_file(..., READING_MODULE)` data path.
///
/// The explicit local position is essential: vendor Linux starts at zero and
/// neither consults nor changes `file->f_pos`. Full parity remains partial
/// until Lupos has inode writer accounting/`deny_write_access` and the
/// `security_kernel_read_file` pre/post LSM hooks.
fn read_module_file(file: &FileRef) -> Result<Vec<u8>, i32> {
    let flags = file.flags.load(Ordering::Acquire);
    let access_mode = flags & O_ACCMODE;
    if flags & O_PATH != 0 || !matches!(access_mode, O_RDONLY | O_RDWR) {
        return Err(EBADF);
    }

    let inode = file.inode().ok_or(EBADF)?;
    if inode.kind != InodeKind::Regular {
        return Err(EINVAL);
    }

    let file_size = inode.size.load(Ordering::Acquire);
    if file_size == 0 {
        return Err(EINVAL);
    }
    if file_size > MAX_MODULE_FILE_SIZE {
        return Err(EFBIG);
    }
    let file_size = usize::try_from(file_size).map_err(|_| EFBIG)?;
    let read = file.fops.read.ok_or(EINVAL)?;

    let mut bytes = Vec::new();
    bytes.try_reserve_exact(file_size).map_err(|_| ENOMEM)?;
    bytes.resize(file_size, 0);

    let mut copied = 0usize;
    let mut pos = 0u64;
    while copied < file_size {
        let wanted = (file_size - copied).min(MAX_RW_COUNT);
        let count = read(file, &mut bytes[copied..copied + wanted], &mut pos)?;
        if count == 0 || count > wanted {
            return Err(EIO);
        }
        copied = copied.checked_add(count).ok_or(EIO)?;
    }
    if copied != file_size || pos != file_size as u64 {
        return Err(EIO);
    }
    Ok(bytes)
}

pub unsafe fn sys_init_module(module_image: *const u8, len: usize, _params: *const u8) -> i64 {
    if !may_manage_modules() {
        return -(EPERM as i64);
    }

    let bytes = match unsafe { copy_module_from_user(module_image, len) } {
        Ok(bytes) => bytes,
        Err(errno) => return -(errno as i64),
    };
    match load_module(bytes.as_slice()) {
        Ok(_) => 0,
        Err(err) => -(load_errno(err) as i64),
    }
}

pub unsafe fn sys_finit_module(fd: i32, _params: *const u8, flags: i32) -> i64 {
    if !may_manage_modules() {
        return -(EPERM as i64);
    }
    if flags & !MODULE_INIT_VALID_FLAGS != 0 {
        return -(EINVAL as i64);
    }

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let Some(ft) = (unsafe { files::get_task_files(task) }) else {
        return -(EBADF as i64);
    };
    let file = match ft.get(fd) {
        Ok(file) => file,
        Err(errno) => return -(errno as i64),
    };
    let bytes = match read_module_file(&file) {
        Ok(bytes) => bytes,
        Err(errno) => return -(errno as i64),
    };

    // The selected vendor configuration has CONFIG_MODULE_DECOMPRESS=n, so
    // the decompressor stub runs only after the complete file read.
    if flags & MODULE_INIT_COMPRESSED_FILE != 0 {
        return -(EOPNOTSUPP as i64);
    }
    // With CONFIG_MODULE_FORCE_LOAD=n, asking to ignore vermagic reaches
    // try_to_force_load() and fails. Keep this after the read, matching Linux
    // error ordering. IGNORE_MODVERSIONS is a no-op with MODVERSIONS=n.
    if flags & MODULE_INIT_IGNORE_VERMAGIC != 0 {
        return -(ENOEXEC as i64);
    }

    match load_module(&bytes) {
        Ok(_) => 0,
        Err(err) => -(load_errno(err) as i64),
    }
}

pub unsafe fn sys_delete_module(name: *const u8, _flags: u32) -> i64 {
    if !may_manage_modules() {
        return -(EPERM as i64);
    }

    let name = match unsafe { user_cstr(name) } {
        Ok(name) => name,
        Err(errno) => return -(errno as i64),
    };
    match delete_module(&name) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use core::sync::atomic::AtomicUsize;

    use super::*;
    use crate::kernel::{
        capability::KernelCapT,
        cred::{Cred, GroupInfo, KGid, KUid, NGROUPS_MAX_INLINE},
        sched,
        task::TaskStruct,
    };

    fn unprivileged_cred() -> Box<Cred> {
        Box::new(Cred {
            usage: AtomicUsize::new(1),
            uid: KUid(1000),
            gid: KGid(1000),
            suid: KUid(1000),
            sgid: KGid(1000),
            euid: KUid(1000),
            egid: KGid(1000),
            fsuid: KUid(1000),
            fsgid: KGid(1000),
            cap_inheritable: KernelCapT::empty(),
            cap_permitted: KernelCapT::empty(),
            cap_effective: KernelCapT::empty(),
            cap_bset: KernelCapT::full(),
            cap_ambient: KernelCapT::empty(),
            securebits: 0,
            group_info: GroupInfo {
                usage: 1,
                ngroups: 0,
                gid: [KGid(0); NGROUPS_MAX_INLINE],
            },
            user_ns: core::ptr::null(),
        })
    }

    #[test]
    fn init_rejects_non_elf_module_image() {
        let image = b"not-a-linux-module";
        assert_eq!(
            unsafe { sys_init_module(image.as_ptr(), image.len(), core::ptr::null()) },
            -(crate::include::uapi::errno::ENOEXEC as i64)
        );
    }

    #[test]
    fn unprivileged_module_syscalls_require_cap_sys_module() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let cred = unprivileged_cred();
        current.pid = 175;
        current.tgid = 175;
        current.cred = &*cred as *const Cred;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            let image = b"not-a-linux-module";
            assert_eq!(
                sys_init_module(image.as_ptr(), image.len(), core::ptr::null()),
                -(EPERM as i64)
            );
            assert_eq!(sys_finit_module(-1, core::ptr::null(), 0), -(EPERM as i64));
            assert_eq!(sys_delete_module(core::ptr::null(), 0), -(EPERM as i64));

            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m78_security_bpf_perf_parity() {
        assert_eq!(
            unsafe { sys_init_module(core::ptr::null(), 1, core::ptr::null()) },
            -(ENOEXEC as i64)
        );
        assert!(unsafe { sys_init_module(b"bad".as_ptr(), 3, core::ptr::null()) } < 0);
        assert_eq!(
            unsafe { sys_finit_module(-1, core::ptr::null(), 0) },
            -(EBADF as i64)
        );
        assert_eq!(
            unsafe { sys_delete_module(core::ptr::null(), 0) },
            -(EFAULT as i64)
        );
    }
}
