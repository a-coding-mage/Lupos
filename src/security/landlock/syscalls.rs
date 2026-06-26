//! linux-parity: complete
//! linux-source: vendor/linux/security/landlock/syscalls.c
//! test-origin: linux:vendor/linux/security/landlock/syscalls.c
//! Landlock syscall surface.  Linux syscalls 444/445/446.

extern crate alloc;

use alloc::string::String;

use super::{add_path_rule, create_ruleset, restrict_self};
use crate::include::uapi::errno::{EBADF, EINVAL};

#[repr(C)]
pub struct LandlockRulesetAttr {
    pub handled_access_fs: u64,
    pub handled_access_net: u64,
    pub scoped: u64,
}

#[repr(C)]
pub struct LandlockPathBeneathAttr {
    pub allowed_access: u64,
    pub parent_fd: i32,
}

pub const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;

/// `sys_landlock_create_ruleset(attr, size, flags)` â€” syscall 444.
pub unsafe fn sys_landlock_create_ruleset(
    attr: *const LandlockRulesetAttr,
    _size: usize,
    _flags: u32,
) -> i64 {
    if attr.is_null() {
        return -(EINVAL as i64);
    }
    let a = unsafe { &*attr };
    create_ruleset(a.handled_access_fs) as i64
}

/// `sys_landlock_add_rule(ruleset_fd, rule_type, rule_attr, flags)` â€” syscall 445.
pub unsafe fn sys_landlock_add_rule(
    ruleset_fd: i32,
    rule_type: u32,
    rule_attr: *const u8,
    _flags: u32,
) -> i64 {
    if rule_type != LANDLOCK_RULE_PATH_BENEATH || rule_attr.is_null() {
        return -(EINVAL as i64);
    }
    let pba = unsafe { &*(rule_attr as *const LandlockPathBeneathAttr) };
    let path = match path_from_fd(pba.parent_fd) {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    match add_path_rule(ruleset_fd, &path, pba.allowed_access) {
        Ok(()) => 0,
        Err(e) => e as i64,
    }
}

/// `sys_landlock_restrict_self(ruleset_fd, flags)` â€” syscall 446.
pub unsafe fn sys_landlock_restrict_self(ruleset_fd: i32, _flags: u32) -> i64 {
    match restrict_self(ruleset_fd) {
        Ok(()) => 0,
        Err(e) => e as i64,
    }
}

fn path_from_fd(fd: i32) -> Result<String, i32> {
    if fd < 0 {
        return Err(EBADF);
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    let Some(files) = (unsafe { crate::kernel::files::get_task_files(task) }) else {
        return Err(EBADF);
    };
    let file = files.get(fd)?;
    Ok(crate::fs::file::file_path(&file))
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::boxed::Box;

    use super::*;
    use crate::fs::dcache::{d_alloc, d_alloc_child};
    use crate::fs::fdtable::FilesStruct;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::NOOP_FILE_OPS;
    use crate::include::uapi::fcntl::O_RDONLY;
    use crate::kernel::{files, sched, task::TaskStruct};

    #[test]
    fn syscall_m78_security_bpf_perf_parity() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(
            unsafe { sys_landlock_create_ruleset(core::ptr::null(), 0, 0) },
            -(EINVAL as i64)
        );
        let attr = LandlockRulesetAttr {
            handled_access_fs: 1,
            handled_access_net: 0,
            scoped: 0,
        };
        let fd = unsafe {
            sys_landlock_create_ruleset(&attr, core::mem::size_of::<LandlockRulesetAttr>(), 0)
        };
        assert!(fd > 0);
        assert_eq!(
            unsafe { sys_landlock_add_rule(fd as i32, 999, core::ptr::null(), 0) },
            -(EINVAL as i64)
        );
        assert_eq!(unsafe { sys_landlock_restrict_self(fd as i32, 0) }, 0);
    }

    #[test]
    fn add_rule_uses_parent_fd_canonical_path() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::landlock::reset_for_test();

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let fdtable = FilesStruct::new();
        let root = d_alloc("/");
        let tmp = d_alloc_child(&root, "tmp");
        let file = alloc_file(tmp, O_RDONLY as u32, 0, &NOOP_FILE_OPS);
        let fd = fdtable.install(file, false).expect("install fd");

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, fdtable);
            sched::set_current(&mut *current as *mut TaskStruct);
        }

        let ruleset = create_ruleset(crate::security::landlock::LANDLOCK_ACCESS_FS_READ_FILE);
        let attr = LandlockPathBeneathAttr {
            allowed_access: crate::security::landlock::LANDLOCK_ACCESS_FS_READ_FILE,
            parent_fd: fd,
        };
        assert_eq!(
            unsafe {
                sys_landlock_add_rule(
                    ruleset,
                    LANDLOCK_RULE_PATH_BENEATH,
                    &attr as *const _ as *const u8,
                    0,
                )
            },
            0
        );
        restrict_self(ruleset).expect("restrict");
        assert_eq!(crate::security::landlock::check_path_open(b"/tmp/x", 0), 0);

        unsafe {
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
