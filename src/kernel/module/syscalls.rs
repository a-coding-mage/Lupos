//! linux-parity: complete
//! linux-source: vendor/linux/kernel/module
//! test-origin: linux:vendor/linux/kernel/module
//! Module-loader syscall glue.
//!
//! Ref: `vendor/linux/kernel/module/main.c`.

extern crate alloc;

use alloc::string::String;

use crate::include::uapi::errno::{EBADF, EFAULT, EINVAL, EPERM};
use crate::kernel::{
    capability::{CAP_SYS_MODULE, capable},
    files, sched,
};

use super::{LoadModuleError, delete_module, load_module};

fn load_errno(err: LoadModuleError) -> i32 {
    match err {
        LoadModuleError::BadElf => crate::include::uapi::errno::ENOEXEC,
        LoadModuleError::AlreadyLoaded => crate::include::uapi::errno::EEXIST,
        LoadModuleError::UndefinedSymbol(_) => crate::include::uapi::errno::ENOENT,
        LoadModuleError::UnsupportedReloc | LoadModuleError::Invalid => EINVAL,
        LoadModuleError::InitFailed(errno) => errno.abs(),
    }
}

unsafe fn user_bytes(ptr: *const u8, len: usize) -> Result<&'static [u8], i32> {
    if ptr.is_null() && len != 0 {
        return Err(EFAULT);
    }
    Ok(unsafe { core::slice::from_raw_parts(ptr, len) })
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

pub unsafe fn sys_init_module(module_image: *const u8, len: usize, _params: *const u8) -> i64 {
    if !may_manage_modules() {
        return -(EPERM as i64);
    }

    let bytes = match unsafe { user_bytes(module_image, len) } {
        Ok(bytes) => bytes,
        Err(errno) => return -(errno as i64),
    };
    match load_module(bytes) {
        Ok(_) => 0,
        Err(err) => -(load_errno(err) as i64),
    }
}

pub unsafe fn sys_finit_module(fd: i32, _params: *const u8, _flags: i32) -> i64 {
    if !may_manage_modules() {
        return -(EPERM as i64);
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
    let Some(inode) = file.inode() else {
        return -(EBADF as i64);
    };
    if let crate::fs::types::InodePrivate::RamBytes(bytes) = &inode.private {
        let bytes = bytes.lock();
        unsafe { sys_init_module(bytes.as_ptr(), bytes.len(), core::ptr::null()) }
    } else {
        -(EINVAL as i64)
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
            -(EFAULT as i64)
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
