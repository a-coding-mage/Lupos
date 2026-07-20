//! linux-parity: partial
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Small kernel syscall helpers shared by late closure milestones.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use spin::Mutex;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{
    E2BIG, EBADF, EFAULT, EINTR, EINVAL, ENODEV, ENOENT, ENOMEM, ENOSYS, ENOTSUP, EPERM, ESRCH,
};
use crate::include::uapi::fcntl::{AT_EMPTY_PATH, AT_FDCWD, AT_SYMLINK_NOFOLLOW};
use crate::kernel::{capability, cred, sched, session};

use super::task::TaskStruct;
use super::utsname::{INIT_UTS_NS, NEW_UTS_LEN_PLUS_NUL, NewUtsname};
use cred::{KGid, KUid};

static GETRANDOM_STATE: AtomicU64 = AtomicU64::new(0x6a09_e667_f3bc_c909);
static PROCESS_NICE: AtomicU32 = AtomicU32::new(20);
const RLIM_INFINITY: u64 = u64::MAX;
const RLIM_NLIMITS: i32 = 16;
const RLIMIT_CPU: i32 = 0;
const RLIMIT_FSIZE: i32 = 1;
const RLIMIT_DATA: i32 = 2;
const RLIMIT_STACK: i32 = 3;
const RLIMIT_CORE: i32 = 4;
const RLIMIT_RSS: i32 = 5;
const RLIMIT_NPROC: i32 = 6;
const RLIMIT_NOFILE: i32 = 7;
pub(crate) const RLIMIT_MEMLOCK: i32 = 8;
const RLIMIT_AS: i32 = 9;
const RLIMIT_LOCKS: i32 = 10;
const RLIMIT_SIGPENDING: i32 = 11;
const RLIMIT_MSGQUEUE: i32 = 12;
const RLIMIT_NICE: i32 = 13;
const RLIMIT_RTPRIO: i32 = 14;
const RLIMIT_RTTIME: i32 = 15;
const SIGSET_WORD_BYTES: usize = 8;
const PRIO_PROCESS: i32 = 0;
const PRIO_PGRP: i32 = 1;
const PRIO_USER: i32 = 2;
const RSEQ_ORIG_SIZE: u32 = 32;
const RSEQ_ORIG_ALIGN: usize = 32;
const RSEQ_FLAG_UNREGISTER: i32 = 1 << 0;
const RSEQ_FLAG_SLICE_EXT_DEFAULT_ON: i32 = 1 << 1;
const RSEQ_FLAGS_SUPPORTED: i32 = RSEQ_FLAG_SLICE_EXT_DEFAULT_ON;
const RSEQ_CPU_ID_UNINITIALIZED: u32 = u32::MAX;
const MAX_RSEQ_REGISTRATIONS: usize = 256;

#[inline]
fn rseq_runtime_supported() -> bool {
    false
}

#[cfg(not(test))]
const MAX_RLIMIT_ENTRIES: usize = crate::kernel::sched::MAX_RUN_QUEUE;

#[cfg(not(test))]
#[derive(Clone, Copy)]
struct RlimitEntry {
    key: i32,
    limits: [RLimit; RLIM_NLIMITS as usize],
}

#[cfg(not(test))]
const EMPTY_RLIMIT_ENTRY: RlimitEntry = RlimitEntry {
    key: 0,
    limits: default_rlimits(),
};

#[cfg(not(test))]
struct ProcessRlimitTable {
    entries: [RlimitEntry; MAX_RLIMIT_ENTRIES],
}

#[cfg(not(test))]
impl ProcessRlimitTable {
    const fn new() -> Self {
        Self {
            entries: [EMPTY_RLIMIT_ENTRY; MAX_RLIMIT_ENTRIES],
        }
    }

    fn get(&self, key: i32) -> Option<[RLimit; RLIM_NLIMITS as usize]> {
        if key <= 0 {
            return None;
        }
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.limits)
    }

    fn get_or_insert_mut(&mut self, key: i32) -> Option<&mut [RLimit; RLIM_NLIMITS as usize]> {
        if key <= 0 {
            return None;
        }
        if let Some(idx) = self.entries.iter().position(|entry| entry.key == key) {
            return Some(&mut self.entries[idx].limits);
        }
        let idx = self.entries.iter().position(|entry| entry.key == 0)?;
        self.entries[idx] = RlimitEntry {
            key,
            limits: default_rlimits(),
        };
        Some(&mut self.entries[idx].limits)
    }

    fn insert(&mut self, key: i32, limits: [RLimit; RLIM_NLIMITS as usize]) {
        if let Some(slot) = self.get_or_insert_mut(key) {
            *slot = limits;
        }
    }

    fn remove(&mut self, key: i32) {
        if key <= 0 {
            return;
        }
        if let Some(idx) = self.entries.iter().position(|entry| entry.key == key) {
            self.entries[idx] = EMPTY_RLIMIT_ENTRY;
        }
    }
}

#[cfg(not(test))]
static PROCESS_RLIMITS: Mutex<ProcessRlimitTable> = Mutex::new(ProcessRlimitTable::new());

#[derive(Clone, Copy)]
struct RseqRegistration {
    active: bool,
    key: i32,
    rseq: usize,
    len: u32,
    sig: u32,
}

const EMPTY_RSEQ_REGISTRATION: RseqRegistration = RseqRegistration {
    active: false,
    key: 0,
    rseq: 0,
    len: 0,
    sig: 0,
};

struct RseqRegistry {
    entries: [RseqRegistration; MAX_RSEQ_REGISTRATIONS],
}

impl RseqRegistry {
    const fn new() -> Self {
        Self {
            entries: [EMPTY_RSEQ_REGISTRATION; MAX_RSEQ_REGISTRATIONS],
        }
    }

    fn get(&self, key: i32) -> Option<RseqRegistration> {
        self.entries
            .iter()
            .copied()
            .find(|entry| entry.active && entry.key == key)
    }

    fn insert(&mut self, entry: RseqRegistration) -> Result<(), i32> {
        if let Some(slot) = self
            .entries
            .iter_mut()
            .find(|candidate| candidate.active && candidate.key == entry.key)
        {
            *slot = entry;
            return Ok(());
        }
        let Some(slot) = self.entries.iter_mut().find(|candidate| !candidate.active) else {
            return Err(ENOMEM);
        };
        *slot = entry;
        Ok(())
    }

    fn remove(&mut self, key: i32) {
        if let Some(slot) = self
            .entries
            .iter_mut()
            .find(|entry| entry.active && entry.key == key)
        {
            *slot = EMPTY_RSEQ_REGISTRATION;
        }
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.entries = [EMPTY_RSEQ_REGISTRATION; MAX_RSEQ_REGISTRATIONS];
    }
}

static RSEQ_REGISTRY: Mutex<RseqRegistry> = Mutex::new(RseqRegistry::new());

struct RealItimerState {
    timer: UnsafeCell<crate::kernel::time::hrtimer::Hrtimer>,
    interval_ns: AtomicU64,
}

unsafe impl Send for RealItimerState {}
unsafe impl Sync for RealItimerState {}

impl RealItimerState {
    const fn new() -> Self {
        Self {
            timer: UnsafeCell::new(crate::kernel::time::hrtimer::Hrtimer::new()),
            interval_ns: AtomicU64::new(0),
        }
    }

    #[inline]
    fn timer_ptr(&self) -> *mut crate::kernel::time::hrtimer::Hrtimer {
        self.timer.get()
    }

    fn cancel_synchronously(&self) {
        loop {
            let ret = crate::kernel::time::hrtimer::hrtimer_try_to_cancel(self.timer_ptr());
            if ret >= 0 {
                return;
            }
            crate::kernel::time::hrtimer::hrtimer_cancel_wait_running(self.timer_ptr());
        }
    }
}

impl Drop for RealItimerState {
    fn drop(&mut self) {
        self.cancel_synchronously();
    }
}

/// Per-process `ITIMER_REAL` state, keyed by tgid.  Linux stores the real
/// interval timer in `signal_struct::real_timer`, which is shared by the thread
/// group.  Each entry is boxed so the contained `Hrtimer` keeps a stable address
/// while it is enqueued in the hrtimer wheel by raw pointer.
static REAL_ITIMERS: Mutex<BTreeMap<i32, Box<RealItimerState>>> = Mutex::new(BTreeMap::new());

/// Cancel and drop a process's real interval timer by tgid.  Called from
/// `release_task`; only the thread-group leader pid matches this key.
pub fn release_task_real_itimer(tgid: i32) {
    if tgid <= 0 {
        return;
    }
    let state = {
        let mut timers = REAL_ITIMERS.lock();
        timers.remove(&tgid)
    };
    if let Some(state) = state {
        state.cancel_synchronously();
    }
}

unsafe fn copy_struct_from_user<T: Copy>(src: *const T) -> Result<T, i32> {
    if src.is_null() {
        return Err(EFAULT);
    }

    let mut value = MaybeUninit::<T>::uninit();
    let left = unsafe {
        uaccess::copy_from_user(
            value.as_mut_ptr() as *mut u8,
            src as *const u8,
            core::mem::size_of::<T>(),
        )
    };
    if left == 0 {
        Ok(unsafe { value.assume_init() })
    } else {
        Err(EFAULT)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxUtsname {
    pub sysname: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub nodename: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub release: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub version: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub machine: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub domainname: [u8; NEW_UTS_LEN_PLUS_NUL],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimeVal {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimeZone {
    pub tz_minuteswest: i32,
    pub tz_dsttime: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ITimerVal {
    pub it_interval: TimeVal,
    pub it_value: TimeVal,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RLimit {
    pub rlim_cur: u64,
    pub rlim_max: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RUsage {
    pub ru_utime: TimeVal,
    pub ru_stime: TimeVal,
    pub ru_maxrss: i64,
    pub ru_ixrss: i64,
    pub ru_idrss: i64,
    pub ru_isrss: i64,
    pub ru_minflt: i64,
    pub ru_majflt: i64,
    pub ru_nswap: i64,
    pub ru_inblock: i64,
    pub ru_oublock: i64,
    pub ru_msgsnd: i64,
    pub ru_msgrcv: i64,
    pub ru_nsignals: i64,
    pub ru_nvcsw: i64,
    pub ru_nivcsw: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Tms {
    pub tms_utime: i64,
    pub tms_stime: i64,
    pub tms_cutime: i64,
    pub tms_cstime: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysInfo {
    pub uptime: i64,
    pub loads: [u64; 3],
    pub totalram: u64,
    pub freeram: u64,
    pub sharedram: u64,
    pub bufferram: u64,
    pub totalswap: u64,
    pub freeswap: u64,
    pub procs: u16,
    pub pad: u16,
    pub totalhigh: u64,
    pub freehigh: u64,
    pub mem_unit: u32,
    pub _f: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timex {
    pub modes: u32,
    pub offset: i64,
    pub freq: i64,
    pub maxerror: i64,
    pub esterror: i64,
    pub status: i32,
    pub constant: i64,
    pub precision: i64,
    pub tolerance: i64,
    pub time: TimeVal,
    pub tick: i64,
    pub ppsfreq: i64,
    pub jitter: i64,
    pub shift: i32,
    pub stabil: i64,
    pub jitcnt: i64,
    pub calcnt: i64,
    pub errcnt: i64,
    pub stbcnt: i64,
    pub tai: i32,
    pub _padding: [i32; 11],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FileHandle {
    pub handle_bytes: u32,
    pub handle_type: i32,
    pub f_handle: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CacheStatRange {
    pub off: u64,
    pub len: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CacheStat {
    pub nr_cache: u64,
    pub nr_dirty: u64,
    pub nr_writeback: u64,
    pub nr_evicted: u64,
    pub nr_recently_evicted: u64,
}

pub const IOPRIO_CLASS_NONE: i32 = 0;
pub const IOPRIO_CLASS_RT: i32 = 1;
pub const IOPRIO_CLASS_BE: i32 = 2;
pub const IOPRIO_CLASS_IDLE: i32 = 3;

fn current_task() -> Result<*mut TaskStruct, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() { Err(ESRCH) } else { Ok(task) }
}

fn task_by_pid(pid: i32) -> *mut TaskStruct {
    if pid == 0 {
        return unsafe { sched::get_current() };
    }
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    sched::find_pool_task_by_pid(pid)
}

fn build_utsname() -> LinuxUtsname {
    let mut name = LinuxUtsname {
        sysname: INIT_UTS_NS.name.sysname,
        nodename: INIT_UTS_NS.name.nodename,
        release: INIT_UTS_NS.name.release,
        version: INIT_UTS_NS.name.version,
        machine: INIT_UTS_NS.name.machine,
        domainname: INIT_UTS_NS.name.domainname,
    };
    name.nodename = super::utsname::current_nodename();
    name.domainname = super::utsname::current_domainname();
    name
}

fn current_cred_ref() -> &'static cred::Cred {
    unsafe { &*cred::current_cred() }
}

fn has_current_cap(cap: u32) -> bool {
    capability::capable(cap)
}

fn uid_matches_any(uid: KUid, candidates: &[KUid]) -> bool {
    candidates.iter().any(|candidate| *candidate == uid)
}

fn gid_matches_any(gid: KGid, candidates: &[KGid]) -> bool {
    candidates.iter().any(|candidate| *candidate == gid)
}

fn uid_change_allowed(uid: KUid, old: &cred::Cred) -> bool {
    uid_matches_any(uid, &[old.uid, old.euid, old.suid])
}

fn gid_change_allowed(gid: KGid, old: &cred::Cred) -> bool {
    gid_matches_any(gid, &[old.gid, old.egid, old.sgid])
}

fn mutate_current_cred_with_old(f: impl FnOnce(&mut cred::Cred, &cred::Cred)) -> i64 {
    if unsafe { sched::get_current() }.is_null() {
        return -(ESRCH as i64);
    }
    let old_cred = cred::current_cred();
    let Some(new_cred) = cred::prepare_creds() else {
        return -(ENOMEM as i64);
    };
    unsafe {
        f(&mut *new_cred, &*old_cred);
    }
    cred::commit_creds(new_cred);
    0
}

fn securebit_mask(bit: u32) -> u32 {
    1u32 << bit
}

fn apply_setxuid_capability_fixup(new: &mut cred::Cred, old: &cred::Cred) {
    if old.securebits & securebit_mask(cred::securebits::SECURE_NO_SETUID_FIXUP) != 0 {
        return;
    }

    let old_had_root_uid = old.uid.0 == 0 || old.euid.0 == 0 || old.suid.0 == 0;
    let new_has_no_root_uid = new.uid.0 != 0 && new.euid.0 != 0 && new.suid.0 != 0;
    if old_had_root_uid && new_has_no_root_uid {
        if old.securebits & securebit_mask(cred::securebits::SECURE_KEEP_CAPS) == 0 {
            new.cap_permitted = crate::kernel::capability::KernelCapT::empty();
            new.cap_effective = crate::kernel::capability::KernelCapT::empty();
        }
        new.cap_ambient = crate::kernel::capability::KernelCapT::empty();
    }

    if old.euid.0 == 0 && new.euid.0 != 0 {
        new.cap_effective = crate::kernel::capability::KernelCapT::empty();
    }
    if old.euid.0 != 0 && new.euid.0 == 0 {
        new.cap_effective = new.cap_permitted;
    }
}

fn mutate_current_cred(f: impl FnOnce(&mut cred::Cred)) -> i64 {
    if unsafe { sched::get_current() }.is_null() {
        return -(ESRCH as i64);
    }
    let Some(new_cred) = cred::prepare_creds() else {
        return -(ENOMEM as i64);
    };
    unsafe {
        f(&mut *new_cred);
    }
    cred::commit_creds(new_cred);
    0
}

/// `getpid(2)` — Linux x86-64 syscall 39.
pub unsafe fn sys_getpid() -> i64 {
    match current_task() {
        Ok(task) => unsafe { (*task).tgid as i64 },
        Err(errno) => -(errno as i64),
    }
}

/// `gettid(2)` — Linux x86-64 syscall 186.
pub unsafe fn sys_gettid() -> i64 {
    match current_task() {
        Ok(task) => unsafe { (*task).pid as i64 },
        Err(errno) => -(errno as i64),
    }
}

/// `getppid(2)` — Linux x86-64 syscall 110.
pub unsafe fn sys_getppid() -> i64 {
    let task = match current_task() {
        Ok(task) => task,
        Err(errno) => return -(errno as i64),
    };
    let parent = unsafe { (*task).m26.real_parent };
    if parent.is_null() {
        0
    } else {
        unsafe { (*parent).tgid as i64 }
    }
}

/// `uname(2)` — Linux x86-64 syscall 63.
pub unsafe fn sys_uname(buf: *mut LinuxUtsname) -> i64 {
    if buf.is_null() {
        return -(EFAULT as i64);
    }
    let name = build_utsname();
    let left = unsafe {
        uaccess::copy_to_user(
            buf as *mut u8,
            &name as *const LinuxUtsname as *const u8,
            core::mem::size_of::<LinuxUtsname>(),
        )
    };
    if left == 0 { 0 } else { -(EFAULT as i64) }
}

pub unsafe fn sys_getuid() -> i64 {
    let c = unsafe { &*cred::current_cred() };
    c.uid.0 as i64
}

pub unsafe fn sys_getgid() -> i64 {
    let c = unsafe { &*cred::current_cred() };
    c.gid.0 as i64
}

pub unsafe fn sys_geteuid() -> i64 {
    let c = unsafe { &*cred::current_cred() };
    c.euid.0 as i64
}

pub unsafe fn sys_getegid() -> i64 {
    let c = unsafe { &*cred::current_cred() };
    c.egid.0 as i64
}

pub unsafe fn sys_setuid(uid: u32) -> i64 {
    let old = current_cred_ref();
    let uid = KUid(uid);
    let privileged = has_current_cap(capability::CAP_SETUID);
    if !privileged && !uid_matches_any(uid, &[old.uid, old.suid]) {
        return -(EPERM as i64);
    }

    mutate_current_cred_with_old(|c, old| {
        c.euid = uid;
        c.fsuid = uid;
        if privileged {
            c.uid = uid;
            c.suid = uid;
        }
        apply_setxuid_capability_fixup(c, old);
    })
}

pub unsafe fn sys_setgid(gid: u32) -> i64 {
    let old = current_cred_ref();
    let gid = KGid(gid);
    let privileged = has_current_cap(capability::CAP_SETGID);
    if !privileged && !gid_matches_any(gid, &[old.gid, old.sgid]) {
        return -(EPERM as i64);
    }

    mutate_current_cred(|c| {
        c.egid = gid;
        c.fsgid = gid;
        if privileged {
            c.gid = gid;
            c.sgid = gid;
        }
    })
}

pub unsafe fn sys_setreuid(ruid: u32, euid: u32) -> i64 {
    let old = current_cred_ref();
    let privileged = has_current_cap(capability::CAP_SETUID);
    if !privileged {
        if ruid != u32::MAX && !uid_matches_any(KUid(ruid), &[old.uid, old.euid]) {
            return -(EPERM as i64);
        }
        if euid != u32::MAX && !uid_change_allowed(KUid(euid), old) {
            return -(EPERM as i64);
        }
    }

    mutate_current_cred_with_old(|c, old| {
        if ruid != u32::MAX {
            c.uid = KUid(ruid);
        }
        if euid != u32::MAX {
            c.euid = KUid(euid);
            c.fsuid = KUid(euid);
        }
        if privileged || ruid != u32::MAX || (euid != u32::MAX && KUid(euid) != old.uid) {
            c.suid = c.euid;
        }
        apply_setxuid_capability_fixup(c, old);
    })
}

pub unsafe fn sys_setregid(rgid: u32, egid: u32) -> i64 {
    let old = current_cred_ref();
    let privileged = has_current_cap(capability::CAP_SETGID);
    if !privileged {
        if rgid != u32::MAX && !gid_matches_any(KGid(rgid), &[old.gid, old.egid]) {
            return -(EPERM as i64);
        }
        if egid != u32::MAX && !gid_change_allowed(KGid(egid), old) {
            return -(EPERM as i64);
        }
    }

    mutate_current_cred(|c| {
        if rgid != u32::MAX {
            c.gid = KGid(rgid);
        }
        if egid != u32::MAX {
            c.egid = KGid(egid);
            c.fsgid = KGid(egid);
        }
        if privileged || rgid != u32::MAX || (egid != u32::MAX && KGid(egid) != old.gid) {
            c.sgid = c.egid;
        }
    })
}

pub unsafe fn sys_setresuid(ruid: u32, euid: u32, suid: u32) -> i64 {
    let old = current_cred_ref();
    let privileged = has_current_cap(capability::CAP_SETUID);
    if !privileged {
        for uid in [ruid, euid, suid] {
            if uid != u32::MAX && !uid_change_allowed(KUid(uid), old) {
                return -(EPERM as i64);
            }
        }
    }

    mutate_current_cred_with_old(|c, old| {
        if ruid != u32::MAX {
            c.uid = KUid(ruid);
        }
        if euid != u32::MAX {
            c.euid = KUid(euid);
            c.fsuid = KUid(euid);
        }
        if suid != u32::MAX {
            c.suid = KUid(suid);
        }
        apply_setxuid_capability_fixup(c, old);
    })
}

pub unsafe fn sys_setresgid(rgid: u32, egid: u32, sgid: u32) -> i64 {
    let old = current_cred_ref();
    let privileged = has_current_cap(capability::CAP_SETGID);
    if !privileged {
        for gid in [rgid, egid, sgid] {
            if gid != u32::MAX && !gid_change_allowed(KGid(gid), old) {
                return -(EPERM as i64);
            }
        }
    }

    mutate_current_cred(|c| {
        if rgid != u32::MAX {
            c.gid = KGid(rgid);
        }
        if egid != u32::MAX {
            c.egid = KGid(egid);
            c.fsgid = KGid(egid);
        }
        if sgid != u32::MAX {
            c.sgid = KGid(sgid);
        }
    })
}

pub unsafe fn sys_getresuid(ruid: *mut u32, euid: *mut u32, suid: *mut u32) -> i64 {
    if ruid.is_null() || euid.is_null() || suid.is_null() {
        return -(EFAULT as i64);
    }
    let c = current_cred_ref();
    for (ptr, value) in [(ruid, c.uid.0), (euid, c.euid.0), (suid, c.suid.0)] {
        if unsafe { uaccess::put_user_u32(ptr, value) }.is_err() {
            return -(EFAULT as i64);
        }
    }
    0
}

pub unsafe fn sys_getresgid(rgid: *mut u32, egid: *mut u32, sgid: *mut u32) -> i64 {
    if rgid.is_null() || egid.is_null() || sgid.is_null() {
        return -(EFAULT as i64);
    }
    let c = current_cred_ref();
    for (ptr, value) in [(rgid, c.gid.0), (egid, c.egid.0), (sgid, c.sgid.0)] {
        if unsafe { uaccess::put_user_u32(ptr, value) }.is_err() {
            return -(EFAULT as i64);
        }
    }
    0
}

pub unsafe fn sys_setfsuid(uid: u32) -> i64 {
    let old = current_cred_ref();
    let old_fsuid = old.fsuid.0 as i64;
    let uid = KUid(uid);
    if has_current_cap(capability::CAP_SETUID)
        || uid_matches_any(uid, &[old.uid, old.euid, old.suid, old.fsuid])
    {
        let _ = mutate_current_cred(|c| {
            c.fsuid = uid;
        });
    }
    old_fsuid
}

pub unsafe fn sys_setfsgid(gid: u32) -> i64 {
    let old = current_cred_ref();
    let old_fsgid = old.fsgid.0 as i64;
    let gid = KGid(gid);
    if has_current_cap(capability::CAP_SETGID)
        || gid_matches_any(gid, &[old.gid, old.egid, old.sgid, old.fsgid])
    {
        let _ = mutate_current_cred(|c| {
            c.fsgid = gid;
        });
    }
    old_fsgid
}

pub unsafe fn sys_getgroups(size: i32, list: *mut u32) -> i64 {
    if size < 0 {
        return -(EINVAL as i64);
    }
    let groups = &current_cred_ref().group_info;
    let ngroups = groups.ngroups as i32;
    if size == 0 {
        return ngroups as i64;
    }
    if size < ngroups {
        return -(EINVAL as i64);
    }
    if list.is_null() && ngroups != 0 {
        return -(EFAULT as i64);
    }
    for i in 0..groups.ngroups as usize {
        if unsafe { uaccess::put_user_u32(list.add(i), groups.gid[i].0) }.is_err() {
            return -(EFAULT as i64);
        }
    }
    ngroups as i64
}

pub unsafe fn sys_setgroups(size: i32, list: *const u32) -> i64 {
    if size < 0 || size as usize > cred::NGROUPS_MAX_INLINE {
        return -(EINVAL as i64);
    }
    if !has_current_cap(capability::CAP_SETGID) {
        return -(EPERM as i64);
    }
    if size != 0 && list.is_null() {
        return -(EFAULT as i64);
    }
    let mut group_info = cred::GroupInfo::default();
    group_info.ngroups = size as u32;
    for i in 0..size as usize {
        let value = match unsafe { uaccess::get_user_u32(list.add(i)) } {
            Ok(value) => value,
            Err(_) => return -(EFAULT as i64),
        };
        group_info.gid[i] = KGid(value);
    }
    // vendor/linux/kernel/groups.c::set_groups() sorts before publishing the
    // credential because in_group_p() performs a binary search. NSS/PAM does
    // not promise that initgroups(3) supplies gids in sorted order.
    crate::kernel::groups::groups_sort(&mut group_info);
    mutate_current_cred(|c| {
        c.group_info = group_info;
    })
}

/// `getpgrp(2)` — Linux returns the caller's process-group ID.
pub unsafe fn sys_getpgrp() -> i64 {
    let task = match current_task() {
        Ok(task) => task,
        Err(errno) => return -(errno as i64),
    };
    let pid = unsafe { (*task).pid };
    session::process_group(pid).unwrap_or(pid) as i64
}

pub unsafe fn sys_getpgid(pid: i32) -> i64 {
    if pid < 0 {
        return -(EINVAL as i64);
    }
    let task = task_by_pid(pid);
    if task.is_null() {
        return -(ESRCH as i64);
    }
    let task_pid = unsafe { (*task).pid };
    session::process_group(task_pid).unwrap_or(task_pid) as i64
}

pub unsafe fn sys_getsid(pid: i32) -> i64 {
    if pid < 0 {
        return -(EINVAL as i64);
    }
    let task = task_by_pid(pid);
    if task.is_null() {
        return -(ESRCH as i64);
    }
    let task_pid = unsafe { (*task).pid };
    session::session_id(task_pid).unwrap_or(task_pid) as i64
}

pub fn sys_pause() -> i64 {
    -(EINTR as i64)
}

fn send_signal_to_all_processes(sig: i32) -> i64 {
    let current = unsafe { sched::get_current() };
    let current_tgid = if current.is_null() {
        0
    } else {
        unsafe { (*current).tgid }
    };
    let mut sent_tgids = alloc::vec::Vec::new();
    crate::kernel::fork::for_each_heap_task(|task| {
        let pid = unsafe { (*task).pid };
        let tgid = unsafe { if (*task).tgid > 0 { (*task).tgid } else { pid } };
        // kill(-1) excludes init's entire thread group, not just the leader's
        // numeric PID. A worker thread has pid > 1 but still has tgid == 1.
        if tgid <= 1 {
            return;
        }
        if (current_tgid > 0 && tgid == current_tgid) || sent_tgids.contains(&tgid) {
            return;
        }
        sent_tgids.push(tgid);
    });
    // Deliver after for_each_heap_task releases HEAP_TASKS. Process-directed
    // signal delivery scans the same tracker to select a thread-group target.
    let mut sent = 0i32;
    for tgid in sent_tgids {
        if sig == 0 || crate::kernel::signal::send_user_signal_to_process(tgid, sig) == 0 {
            sent += 1;
        }
    }
    if sent == 0 { -(ESRCH as i64) } else { 0 }
}

pub unsafe fn sys_kill(pid: i32, sig: i32) -> i64 {
    if !(0..=64).contains(&sig) {
        return -(EINVAL as i64);
    }
    // Linux `kill_something_info()` interprets non-positive PID values as
    // process-group/broadcast selectors; see vendor/linux/kernel/signal.c.
    if pid == 0 {
        let current = match current_task() {
            Ok(task) => task,
            Err(errno) => return -(errno as i64),
        };
        let current_pid = unsafe { (*current).pid };
        let pgrp = session::process_group(current_pid).unwrap_or(current_pid);
        return crate::kernel::signal::send_user_signal_to_process_group(pgrp, sig) as i64;
    }
    if pid == -1 {
        return send_signal_to_all_processes(sig);
    }
    if pid == i32::MIN {
        return -(ESRCH as i64);
    }
    if pid < -1 {
        let pgrp = -pid;
        return crate::kernel::signal::send_user_signal_to_process_group(pgrp, sig) as i64;
    }
    let mut target = task_by_pid(pid);
    if target.is_null() {
        if let Ok(current) = current_task() {
            if unsafe { (*current).pid == pid || (*current).tgid == pid } {
                target = current;
            }
        }
    }
    if target.is_null() {
        return -(ESRCH as i64);
    }
    if sig == 0 {
        return 0;
    }
    let tgid = unsafe {
        if (*target).tgid > 0 {
            (*target).tgid
        } else {
            (*target).pid
        }
    };
    crate::kernel::signal::send_user_signal_to_process(tgid, sig) as i64
}

pub fn sys_rt_sigsuspend(sigsetsize: usize) -> i64 {
    if sigsetsize != SIGSET_WORD_BYTES {
        return -(EINVAL as i64);
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    if !task.is_null() {
        unsafe {
            (*task).__state.store(
                crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                core::sync::atomic::Ordering::Release,
            );
            crate::kernel::sched::schedule_with_irqs_enabled();
            (*task).__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                core::sync::atomic::Ordering::Release,
            );
        }
    }
    -(EINTR as i64)
}

pub fn sys_restart_syscall() -> i64 {
    -(EINTR as i64)
}

pub unsafe fn sys_time(tloc: *mut i64) -> i64 {
    let sec = (crate::kernel::time::ktime_get_real() / 1_000_000_000) as i64;
    if !tloc.is_null() {
        let left = unsafe {
            uaccess::copy_to_user(
                tloc as *mut u8,
                &sec as *const i64 as *const u8,
                core::mem::size_of::<i64>(),
            )
        };
        if left != 0 {
            return -(EFAULT as i64);
        }
    }
    sec
}

pub unsafe fn sys_getitimer(which: i32, curr_value: *mut ITimerVal) -> i64 {
    if !(0..=2).contains(&which) {
        return -(EINVAL as i64);
    }
    if curr_value.is_null() {
        return -(EFAULT as i64);
    }
    let value = if which == crate::kernel::time::itimer::ITIMER_REAL {
        real_itimer_snapshot()
    } else {
        ITimerVal::default()
    };
    let left = unsafe {
        uaccess::copy_to_user(
            curr_value as *mut u8,
            &value as *const ITimerVal as *const u8,
            core::mem::size_of::<ITimerVal>(),
        )
    };
    if left == 0 { 0 } else { -(EFAULT as i64) }
}

fn timeval_to_ns(tv: TimeVal) -> u64 {
    (tv.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add((tv.tv_usec as u64).saturating_mul(1_000))
}

fn ns_to_timeval(ns: u64) -> TimeVal {
    TimeVal {
        tv_sec: (ns / 1_000_000_000) as i64,
        tv_usec: ((ns % 1_000_000_000) / 1_000) as i64,
    }
}

fn real_itimer_snapshot() -> ITimerVal {
    let pid = current_real_itimer_key();
    let timers = REAL_ITIMERS.lock();
    let Some(state) = timers.get(&pid) else {
        return ITimerVal::default();
    };
    ITimerVal {
        it_interval: ns_to_timeval(state.interval_ns.load(Ordering::Acquire)),
        it_value: ns_to_timeval(crate::kernel::time::hrtimer::hrtimer_get_remaining(
            state.timer_ptr(),
        )),
    }
}

fn current_real_itimer_key() -> i32 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        0
    } else {
        unsafe {
            if (*task).tgid > 0 {
                (*task).tgid
            } else {
                (*task).pid.max(0)
            }
        }
    }
}

fn current_real_itimer_target() -> *mut crate::kernel::task::TaskStruct {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return core::ptr::null_mut();
    }
    let leader = unsafe { (*task).m26.group_leader };
    if !leader.is_null() && unsafe { (*leader).tgid == (*task).tgid } {
        leader
    } else {
        task
    }
}

fn real_itimer_fired(
    t: *mut crate::kernel::time::hrtimer::Hrtimer,
) -> crate::kernel::time::hrtimer::HrtimerRestart {
    let tgid = if t.is_null() {
        0
    } else {
        unsafe { (*t).data as i32 }
    };
    if tgid > 0 {
        let _ = crate::kernel::signal::queue_itimer_sigalrm_noalloc(tgid);
    }
    crate::kernel::time::hrtimer::HrtimerRestart::NoRestart
}

/// Linux re-arms periodic ITIMER_REAL timers from `dequeue_signal()` when the
/// generated SIGALRM is delivered, not from the hrtimer callback itself.  This
/// prevents a small-interval timer from continuously interrupting a task that
/// has not had a chance to handle the previous SIGALRM.
pub fn rearm_real_itimer_after_sigalrm(pid: i32) {
    if pid <= 0 {
        return;
    }
    let timers = REAL_ITIMERS.lock();
    let Some(state) = timers.get(&pid) else {
        return;
    };
    let interval_ns = state.interval_ns.load(Ordering::Acquire);
    if interval_ns == 0 {
        return;
    }
    let timer_ptr = state.timer_ptr();
    loop {
        let cancel = crate::kernel::time::hrtimer::hrtimer_try_to_cancel(timer_ptr);
        if cancel < 0 {
            crate::kernel::time::hrtimer::hrtimer_cancel_wait_running(timer_ptr);
            continue;
        }
        if cancel > 0 {
            // A newer setitimer() setting was already queued. Preserve its
            // absolute expiry rather than forwarding the old generation.
            crate::kernel::time::hrtimer::hrtimer_restart(timer_ptr);
            return;
        }
        unsafe {
            let _ = crate::kernel::time::hrtimer::hrtimer_forward_now(&mut *timer_ptr, interval_ns);
        }
        crate::kernel::time::hrtimer::hrtimer_restart(timer_ptr);
        return;
    }
}

fn arm_real_itimer(value_ns: u64, interval_ns: u64, target_tgid: i32) -> ITimerVal {
    // Linux allocates signal/process state before an hrtimer can publish it to
    // the interrupt path. Bind the existing group leader here so expiry never
    // needs the allocating task registry or lazy SignalState construction.
    if value_ns != 0 {
        let target = current_real_itimer_target();
        if !target.is_null() && unsafe { (*target).tgid == target_tgid } {
            let _ = unsafe { crate::kernel::signal::prepare_timer_signal_target(target) };
        }
    }

    let mut timers = REAL_ITIMERS.lock();
    let state = timers
        .entry(target_tgid)
        .or_insert_with(|| Box::new(RealItimerState::new()));
    let state = state.as_ref();
    let timer_ptr = state.timer_ptr();
    let old = ITimerVal {
        it_interval: ns_to_timeval(state.interval_ns.load(Ordering::Acquire)),
        it_value: ns_to_timeval(crate::kernel::time::hrtimer::hrtimer_get_remaining(
            timer_ptr,
        )),
    };

    state.cancel_synchronously();
    let timer = unsafe { &mut *timer_ptr };
    crate::kernel::time::hrtimer::hrtimer_init(
        timer,
        // Linux initializes signal_struct::real_timer with CLOCK_MONOTONIC.
        crate::kernel::time::hrtimer::ClockBase::Monotonic,
        crate::kernel::time::hrtimer::HrtimerMode::Abs,
    );
    state.interval_ns.store(
        if value_ns == 0 { 0 } else { interval_ns },
        Ordering::Release,
    );
    timer.function = Some(real_itimer_fired);
    timer.data = target_tgid.max(0) as usize;
    if value_ns != 0 {
        crate::kernel::time::hrtimer::hrtimer_start(
            timer_ptr,
            value_ns,
            crate::kernel::time::hrtimer::HrtimerMode::Rel,
        );
    }
    old
}

pub unsafe fn sys_setitimer(
    which: i32,
    new_value: *const ITimerVal,
    old_value: *mut ITimerVal,
) -> i64 {
    if !(0..=2).contains(&which) {
        return -(EINVAL as i64);
    }
    let new = match unsafe { copy_struct_from_user(new_value) } {
        Ok(new) => new,
        Err(errno) => return -(errno as i64),
    };
    if new.it_interval.tv_sec < 0
        || new.it_interval.tv_usec < 0
        || new.it_interval.tv_usec >= 1_000_000
        || new.it_value.tv_sec < 0
        || new.it_value.tv_usec < 0
        || new.it_value.tv_usec >= 1_000_000
    {
        return -(EINVAL as i64);
    }
    let old = if which == crate::kernel::time::itimer::ITIMER_REAL {
        arm_real_itimer(
            timeval_to_ns(new.it_value),
            timeval_to_ns(new.it_interval),
            current_real_itimer_key(),
        )
    } else {
        ITimerVal::default()
    };
    if !old_value.is_null() {
        let left = unsafe {
            uaccess::copy_to_user(
                old_value as *mut u8,
                &old as *const ITimerVal as *const u8,
                core::mem::size_of::<ITimerVal>(),
            )
        };
        if left != 0 {
            return -(EFAULT as i64);
        }
    }
    0
}

pub fn sys_alarm(seconds: u32) -> i64 {
    let old = arm_real_itimer(
        (seconds as u64).saturating_mul(1_000_000_000),
        0,
        current_real_itimer_key(),
    );
    let remaining_ns = timeval_to_ns(old.it_value);
    if remaining_ns == 0 {
        0
    } else {
        remaining_ns.div_ceil(1_000_000_000) as i64
    }
}

pub unsafe fn sys_utime(filename: *const u8, _times: *const TimeVal) -> i64 {
    if filename.is_null() {
        return -(EFAULT as i64);
    }
    0
}

pub unsafe fn sys_utimes(filename: *const u8, times: *const TimeVal) -> i64 {
    unsafe { sys_utime(filename, times) }
}

pub unsafe fn sys_futimesat(dfd: i32, filename: *const u8, times: *const TimeVal) -> i64 {
    if filename.is_null() {
        return if dfd == AT_FDCWD { -(EFAULT as i64) } else { 0 };
    }
    unsafe { sys_utime(filename, times) }
}

pub unsafe fn sys_utimensat(
    dfd: i32,
    filename: *const u8,
    _times: *const crate::kernel::time::Timespec64,
    flags: i32,
) -> i64 {
    let allowed = (AT_SYMLINK_NOFOLLOW | AT_EMPTY_PATH) as i32;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }
    if filename.is_null() {
        if dfd != AT_FDCWD && flags == 0 {
            return 0;
        }
        return if dfd != AT_FDCWD {
            -(EINVAL as i64)
        } else {
            -(EFAULT as i64)
        };
    }
    0
}

pub unsafe fn sys_gettimeofday(tv: *mut TimeVal, tz: *mut TimeZone) -> i64 {
    let ns = crate::kernel::time::ktime_get_real();
    if !tv.is_null() {
        let val = TimeVal {
            tv_sec: (ns / 1_000_000_000) as i64,
            tv_usec: ((ns % 1_000_000_000) / 1_000) as i64,
        };
        let left = unsafe {
            uaccess::copy_to_user(
                tv as *mut u8,
                &val as *const TimeVal as *const u8,
                core::mem::size_of::<TimeVal>(),
            )
        };
        if left != 0 {
            return -(EFAULT as i64);
        }
    }
    if !tz.is_null() {
        let zone = TimeZone::default();
        let left = unsafe {
            uaccess::copy_to_user(
                tz as *mut u8,
                &zone as *const TimeZone as *const u8,
                core::mem::size_of::<TimeZone>(),
            )
        };
        if left != 0 {
            return -(EFAULT as i64);
        }
    }
    0
}

pub(crate) fn next_random_u64() -> u64 {
    let mut cur = GETRANDOM_STATE.load(Ordering::Acquire);
    loop {
        let mut next = cur;
        next ^= next << 13;
        next ^= next >> 7;
        next ^= next << 17;
        match GETRANDOM_STATE.compare_exchange(cur, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return next,
            Err(actual) => cur = actual,
        }
    }
}

pub unsafe fn sys_getrandom(buf: *mut u8, buflen: usize, flags: u32) -> i64 {
    const GRND_NONBLOCK: u32 = 0x0001;
    const GRND_RANDOM: u32 = 0x0002;
    const GRND_INSECURE: u32 = 0x0004;
    if flags & !(GRND_NONBLOCK | GRND_RANDOM | GRND_INSECURE) != 0 {
        return -(EINVAL as i64);
    }
    if buflen == 0 {
        return 0;
    }
    if buf.is_null() {
        return -(EFAULT as i64);
    }
    let mut done = 0usize;
    while done < buflen {
        let bytes = next_random_u64().to_ne_bytes();
        let n = (buflen - done).min(bytes.len());
        let not_copied = unsafe { uaccess::copy_to_user(buf.add(done), bytes.as_ptr(), n) };
        let copied = n - not_copied;
        done += copied;
        if copied < n {
            break;
        }
    }
    if done == 0 {
        -(EFAULT as i64)
    } else {
        done as i64
    }
}

pub fn sys_umask(mask: u32) -> i64 {
    crate::fs::fs_struct::set_current_umask(mask) as i64
}

pub unsafe fn sys_getrlimit(resource: i32, rlim: *mut RLimit) -> i64 {
    if !(0..RLIM_NLIMITS).contains(&resource) {
        return -(EINVAL as i64);
    }
    if rlim.is_null() {
        return -(EFAULT as i64);
    }
    let value = current_rlimit(resource);
    let left = unsafe {
        uaccess::copy_to_user(
            rlim as *mut u8,
            &value as *const RLimit as *const u8,
            core::mem::size_of::<RLimit>(),
        )
    };
    if left == 0 { 0 } else { -(EFAULT as i64) }
}

const fn default_rlimits() -> [RLimit; RLIM_NLIMITS as usize] {
    [
        default_rlimit(RLIMIT_CPU),
        default_rlimit(RLIMIT_FSIZE),
        default_rlimit(RLIMIT_DATA),
        default_rlimit(RLIMIT_STACK),
        default_rlimit(RLIMIT_CORE),
        default_rlimit(RLIMIT_RSS),
        default_rlimit(RLIMIT_NPROC),
        default_rlimit(RLIMIT_NOFILE),
        default_rlimit(RLIMIT_MEMLOCK),
        default_rlimit(RLIMIT_AS),
        default_rlimit(RLIMIT_LOCKS),
        default_rlimit(RLIMIT_SIGPENDING),
        default_rlimit(RLIMIT_MSGQUEUE),
        default_rlimit(RLIMIT_NICE),
        default_rlimit(RLIMIT_RTPRIO),
        default_rlimit(RLIMIT_RTTIME),
    ]
}

const fn default_rlimit(resource: i32) -> RLimit {
    match resource {
        RLIMIT_STACK => RLimit {
            rlim_cur: 8 * 1024 * 1024,
            rlim_max: RLIM_INFINITY,
        },
        RLIMIT_CORE => RLimit {
            rlim_cur: 0,
            rlim_max: RLIM_INFINITY,
        },
        RLIMIT_NOFILE => RLimit {
            rlim_cur: 1024,
            rlim_max: 1024,
        },
        RLIMIT_NPROC | RLIMIT_SIGPENDING => RLimit {
            rlim_cur: 4096,
            rlim_max: 4096,
        },
        RLIMIT_MEMLOCK => RLimit {
            rlim_cur: 64 * 1024,
            rlim_max: 64 * 1024,
        },
        RLIMIT_MSGQUEUE => RLimit {
            rlim_cur: 819_200,
            rlim_max: 819_200,
        },
        RLIMIT_NICE | RLIMIT_RTPRIO => RLimit {
            rlim_cur: 0,
            rlim_max: 0,
        },
        RLIMIT_CPU | RLIMIT_FSIZE | RLIMIT_DATA | RLIMIT_RSS | RLIMIT_AS | RLIMIT_LOCKS
        | RLIMIT_RTTIME | _ => RLimit {
            rlim_cur: RLIM_INFINITY,
            rlim_max: RLIM_INFINITY,
        },
    }
}

pub(crate) fn current_rlimit(resource: i32) -> RLimit {
    #[cfg(not(test))]
    {
        let key = current_rlimit_key();
        let table = PROCESS_RLIMITS.lock();
        table
            .get(key)
            .map(|limits| limits[resource as usize])
            .unwrap_or_else(|| default_rlimit(resource))
    }
    #[cfg(test)]
    {
        default_rlimit(resource)
    }
}

fn set_current_rlimit(resource: i32, limit: RLimit) {
    #[cfg(not(test))]
    {
        let key = current_rlimit_key();
        let mut table = PROCESS_RLIMITS.lock();
        if let Some(limits) = table.get_or_insert_mut(key) {
            limits[resource as usize] = limit;
        }
    }
    #[cfg(test)]
    {
        let _ = (resource, limit);
    }
}

#[cfg(not(test))]
fn rlimit_key_for_task(task: *mut TaskStruct) -> i32 {
    if task.is_null() {
        return 0;
    }
    unsafe {
        if (*task).tgid > 0 {
            (*task).tgid
        } else {
            (*task).pid.max(0)
        }
    }
}

#[cfg(not(test))]
fn current_rlimit_key() -> i32 {
    rlimit_key_for_task(unsafe { sched::get_current() })
}

pub(crate) fn inherit_process_rlimits(parent: *mut TaskStruct, child: *mut TaskStruct) {
    #[cfg(not(test))]
    {
        let parent_key = rlimit_key_for_task(parent);
        let child_key = rlimit_key_for_task(child);
        if parent_key == child_key {
            return;
        }
        let mut table = PROCESS_RLIMITS.lock();
        let parent_limits = table.get(parent_key).unwrap_or_else(default_rlimits);
        table.insert(child_key, parent_limits);
    }
    #[cfg(test)]
    {
        let _ = (parent, child);
    }
}

pub(crate) fn release_process_rlimits(task: *mut TaskStruct) {
    #[cfg(not(test))]
    {
        if task.is_null() {
            return;
        }
        let should_remove = unsafe { (*task).pid == (*task).tgid };
        if should_remove {
            PROCESS_RLIMITS.lock().remove(rlimit_key_for_task(task));
        }
    }
    #[cfg(test)]
    {
        let _ = task;
    }
}

fn rseq_key_for_task(task: *mut TaskStruct) -> i32 {
    if task.is_null() {
        return 0;
    }
    unsafe { (*task).pid.max(0) }
}

fn current_rseq_key() -> i32 {
    rseq_key_for_task(unsafe { sched::get_current() })
}

pub(crate) fn release_task_rseq_registration(task: *mut TaskStruct) {
    RSEQ_REGISTRY.lock().remove(rseq_key_for_task(task));
}

pub(crate) fn clear_current_rseq_registration_for_exec() {
    // linux-source: vendor/linux/fs/exec.c
    // Linux calls `rseq_execve(current)` while installing a new image; the
    // TLS address belongs to the old mm and must not survive exec.
    RSEQ_REGISTRY.lock().remove(current_rseq_key());
}

pub unsafe fn sys_getpriority(which: i32, who: i32) -> i64 {
    if !matches!(which, PRIO_PROCESS | PRIO_PGRP | PRIO_USER) {
        return -(EINVAL as i64);
    }
    if who < 0 {
        return -(ESRCH as i64);
    }
    PROCESS_NICE.load(Ordering::Acquire) as i64
}

pub unsafe fn sys_setpriority(which: i32, who: i32, niceval: i32) -> i64 {
    if !matches!(which, PRIO_PROCESS | PRIO_PGRP | PRIO_USER) {
        return -(EINVAL as i64);
    }
    if who < 0 {
        return -(ESRCH as i64);
    }
    let clamped = niceval.clamp(-20, 19);
    PROCESS_NICE.store((20 - clamped) as u32, Ordering::Release);
    0
}

pub unsafe fn sys_sysfs(option: i32, _arg1: u64, _arg2: u64) -> i64 {
    match option {
        1 => 0,
        2 | 3 => -(EINVAL as i64),
        _ => -(EINVAL as i64),
    }
}

pub fn sys_vhangup() -> i64 {
    -(EPERM as i64)
}

pub unsafe fn sys_modify_ldt(func: i32, ptr: *mut u8, bytecount: usize) -> i64 {
    unsafe { crate::arch::x86::kernel::ldt::sys_modify_ldt(func, ptr, bytecount) }
}

pub unsafe fn sys_adjtimex(txc_p: *mut Timex) -> i64 {
    let mut txc = match unsafe { copy_struct_from_user(txc_p as *const Timex) } {
        Ok(txc) => txc,
        Err(errno) => return -(errno as i64),
    };
    txc.time.tv_sec = (crate::kernel::time::ktime_get_real() / 1_000_000_000) as i64;
    txc.time.tv_usec = ((crate::kernel::time::ktime_get_real() % 1_000_000_000) / 1_000) as i64;
    let left = unsafe {
        uaccess::copy_to_user(
            txc_p as *mut u8,
            &txc as *const Timex as *const u8,
            core::mem::size_of::<Timex>(),
        )
    };
    if left == 0 { 0 } else { -(EFAULT as i64) }
}

pub unsafe fn sys_settimeofday(tv: *const TimeVal, _tz: *const TimeZone) -> i64 {
    if !tv.is_null() {
        let val = match unsafe { copy_struct_from_user(tv) } {
            Ok(val) => val,
            Err(errno) => return -(errno as i64),
        };
        if val.tv_sec < 0 || !(0..1_000_000).contains(&val.tv_usec) {
            return -(EINVAL as i64);
        }
    }
    -(EPERM as i64)
}

const SWAP_FLAG_PREFER: u32 = 0x8000;
const SWAP_FLAG_PRIO_MASK: u32 = 0x7fff;
const SWAP_FLAG_DISCARD: u32 = 0x10000;
const SWAP_FLAG_DISCARD_ONCE: u32 = 0x20000;
const SWAP_FLAG_DISCARD_PAGES: u32 = 0x40000;
const SWAP_FLAGS_VALID: u32 = SWAP_FLAG_PRIO_MASK
    | SWAP_FLAG_PREFER
    | SWAP_FLAG_DISCARD
    | SWAP_FLAG_DISCARD_ONCE
    | SWAP_FLAG_DISCARD_PAGES;
const DEF_SWAP_PRIO: i32 = -1;
const SWAP_HEADER_BOOTBITS: usize = 1024;
const SWAP_HEADER_MAGIC_OFFSET: usize = crate::mm::frame::PAGE_SIZE - 10;

unsafe fn copy_syscall_path(ptr: *const u8) -> Result<String, i32> {
    if ptr.is_null() {
        return Err(EFAULT);
    }
    const PATH_MAX: usize = 4096;
    let mut buf = vec![0u8; PATH_MAX];
    let n = unsafe { uaccess::strncpy_from_user(buf.as_mut_ptr(), ptr, buf.len()) };
    if n < 0 {
        return Err((-n) as i32);
    }
    core::str::from_utf8(&buf[..n as usize])
        .map(String::from)
        .map_err(|_| EINVAL)
}

fn decode_swap_priority(flags: i32) -> Result<i32, i32> {
    let flags = flags as u32;
    if flags & !SWAP_FLAGS_VALID != 0 {
        return Err(EINVAL);
    }
    if flags & SWAP_FLAG_PREFER != 0 {
        Ok((flags & SWAP_FLAG_PRIO_MASK) as i32)
    } else {
        Ok(DEF_SWAP_PRIO)
    }
}

fn read_le_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

fn parse_swap_header_pages(header: &[u8], available_pages: u32) -> Result<u32, i32> {
    if header.len() < crate::mm::frame::PAGE_SIZE {
        return Err(EINVAL);
    }
    if &header[SWAP_HEADER_MAGIC_OFFSET..] != b"SWAPSPACE2" {
        return Err(EINVAL);
    }

    let version = read_le_u32(header, SWAP_HEADER_BOOTBITS);
    let last_page = read_le_u32(header, SWAP_HEADER_BOOTBITS + 4);
    let nr_badpages = read_le_u32(header, SWAP_HEADER_BOOTBITS + 8);
    if version != 1 || last_page == 0 || nr_badpages != 0 {
        return Err(EINVAL);
    }

    let header_pages = last_page.checked_add(1).ok_or(EINVAL)?;
    if header_pages > available_pages {
        return Err(EINVAL);
    }
    Ok(header_pages)
}

fn swapfile_pages_for_path(
    path: &str,
) -> Result<(String, u32, crate::mm::swap::SwapBackingKind), i32> {
    use crate::fs::file::{alloc_file, dentry_path, fput};
    use crate::fs::read_write::vfs_read;
    use crate::fs::types::InodeKind;
    use crate::include::uapi::fcntl::O_RDONLY;

    let (_mnt, dentry) = crate::fs::mount::resolve_path_follow(path)?;
    let inode = dentry.inode().ok_or(ENOENT)?;
    let canonical_path = dentry_path(&dentry);

    match inode.kind {
        InodeKind::Regular => {
            let size = inode.size.load(Ordering::Acquire);
            let file_pages =
                (size / crate::mm::frame::PAGE_SIZE as u64).min(u32::MAX as u64) as u32;
            if file_pages <= 1 {
                return Err(EINVAL);
            }

            let mut header = vec![0u8; crate::mm::frame::PAGE_SIZE];
            let file = alloc_file(dentry.clone(), O_RDONLY, 0, inode.fops);
            let read = vfs_read(&file, &mut header)?;
            fput(file);
            if read < crate::mm::frame::PAGE_SIZE {
                return Err(EINVAL);
            }
            let pages = parse_swap_header_pages(&header, file_pages)?;
            Ok((
                canonical_path,
                pages,
                crate::mm::swap::SwapBackingKind::File,
            ))
        }
        InodeKind::Blockdev => {
            let bdev =
                crate::block::block_device::lookup_block_device(&canonical_path).ok_or(ENODEV)?;
            let device_pages = (bdev.capacity_bytes() / crate::mm::frame::PAGE_SIZE as u64)
                .min(u32::MAX as u64) as u32;
            if device_pages <= 1 {
                return Err(EINVAL);
            }

            let sectors = (crate::mm::frame::PAGE_SIZE as u64).div_ceil(512);
            let header = crate::block::partitions::read_sectors(&bdev, 0, sectors)?;
            let pages = parse_swap_header_pages(&header, device_pages)?;
            Ok((
                canonical_path,
                pages,
                crate::mm::swap::SwapBackingKind::Partition,
            ))
        }
        _ => Err(EINVAL),
    }
}

fn do_swapon_path(path: String, flags: i32) -> i64 {
    let priority = match decode_swap_priority(flags) {
        Ok(priority) => priority,
        Err(errno) => return -(errno as i64),
    };
    if !crate::kernel::capability::capable(crate::kernel::capability::CAP_SYS_ADMIN) {
        return -(EPERM as i64);
    }
    let (canonical_path, pages, backing_kind) = match swapfile_pages_for_path(&path) {
        Ok(result) => result,
        Err(errno) => {
            crate::kernel::printk::log_warn!(
                "swap",
                "swapon {} rejected before backend errno={}",
                path,
                errno
            );
            return -(errno as i64);
        }
    };
    let backend = match backing_kind {
        crate::mm::swap::SwapBackingKind::File => {
            crate::mm::swap::swapon_path(canonical_path, pages, priority)
        }
        crate::mm::swap::SwapBackingKind::Partition => {
            crate::mm::swap::swapon_block_path(canonical_path, pages, priority)
        }
    };
    match backend {
        Ok(_) => {
            crate::kernel::printk::log_info!("swap", "swapon {} active pages={}", path, pages);
            0
        }
        Err(errno) => {
            crate::kernel::printk::log_warn!(
                "swap",
                "swapon {} backend failed errno={}",
                path,
                errno
            );
            errno as i64
        }
    }
}

pub fn swapon_kernel_path(path: &str, flags: i32) -> i64 {
    do_swapon_path(String::from(path), flags)
}

pub fn sys_swapon(specialfile: *const u8, flags: i32) -> i64 {
    if specialfile.is_null() {
        return -(EFAULT as i64);
    }
    let path = match unsafe { copy_syscall_path(specialfile) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    do_swapon_path(path, flags)
}

pub fn sys_swapoff(specialfile: *const u8) -> i64 {
    if specialfile.is_null() {
        return -(EFAULT as i64);
    }
    if !crate::kernel::capability::capable(crate::kernel::capability::CAP_SYS_ADMIN) {
        return -(EPERM as i64);
    }
    let path = match unsafe { copy_syscall_path(specialfile) } {
        Ok(path) => path,
        Err(errno) => return -(errno as i64),
    };
    let canonical_path = match crate::fs::mount::resolve_path_follow(&path) {
        Ok((_mnt, dentry)) => crate::fs::file::dentry_path(&dentry),
        Err(errno) => return -(errno as i64),
    };
    match crate::mm::swap::swapoff_path(&canonical_path) {
        Ok(()) => 0,
        Err(errno) => errno as i64,
    }
}

// Linux `reboot(2)` magic and command constants.
// Ref: vendor/linux/include/uapi/linux/reboot.h
pub const LINUX_REBOOT_MAGIC1: i32 = 0xfee1dead_u32 as i32;
pub const LINUX_REBOOT_MAGIC2: i32 = 672274793;
pub const LINUX_REBOOT_MAGIC2A: i32 = 85072278;
pub const LINUX_REBOOT_MAGIC2B: i32 = 369367448;
pub const LINUX_REBOOT_MAGIC2C: i32 = 537993216;
pub const LINUX_REBOOT_CMD_RESTART: u32 = 0x0123_4567;
pub const LINUX_REBOOT_CMD_HALT: u32 = 0xCDEF_0123;
pub const LINUX_REBOOT_CMD_POWER_OFF: u32 = 0x4321_FEDC;
pub const LINUX_REBOOT_CMD_RESTART2: u32 = 0xA1B2_C3D4;
pub const LINUX_REBOOT_CMD_CAD_ON: u32 = 0x89AB_CDEF;
pub const LINUX_REBOOT_CMD_CAD_OFF: u32 = 0x0000_0000;

/// Outcome of a syntactically valid `reboot(2)` invocation, mirroring the
/// dispatch in `vendor/linux/kernel/reboot.c::__do_sys_reboot`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebootAction {
    /// `LINUX_REBOOT_CMD_RESTART` / `LINUX_REBOOT_CMD_RESTART2` —
    /// machine restart via the platform reset path.
    Restart,
    /// `LINUX_REBOOT_CMD_HALT` — halt the CPU (no return to userspace).
    Halt,
    /// `LINUX_REBOOT_CMD_POWER_OFF` — shut the machine off.
    PowerOff,
    /// `LINUX_REBOOT_CMD_CAD_ON` / `_CAD_OFF` — toggle Ctrl-Alt-Del handling.
    /// Returns `0` to userspace without rebooting.
    CadToggle,
}

/// Validate the magic words and decode `cmd` into a [`RebootAction`].
///
/// Returns the positive `errno` on rejection so callers can negate it for
/// the syscall ABI.  This mirrors the validation order in
/// `vendor/linux/kernel/reboot.c::__do_sys_reboot`:
///
/// 1. `magic1` must be `LINUX_REBOOT_MAGIC1`.
/// 2. `magic2` must be one of the four accepted MAGIC2 variants.
/// 3. `cmd` must be a recognised `LINUX_REBOOT_CMD_*` constant.
pub fn decode_reboot(magic1: i32, magic2: i32, cmd: u32) -> Result<RebootAction, i32> {
    if magic1 != LINUX_REBOOT_MAGIC1
        || !matches!(
            magic2,
            LINUX_REBOOT_MAGIC2
                | LINUX_REBOOT_MAGIC2A
                | LINUX_REBOOT_MAGIC2B
                | LINUX_REBOOT_MAGIC2C
        )
    {
        return Err(EINVAL);
    }
    match cmd {
        LINUX_REBOOT_CMD_RESTART | LINUX_REBOOT_CMD_RESTART2 => Ok(RebootAction::Restart),
        LINUX_REBOOT_CMD_HALT => Ok(RebootAction::Halt),
        LINUX_REBOOT_CMD_POWER_OFF => Ok(RebootAction::PowerOff),
        LINUX_REBOOT_CMD_CAD_ON | LINUX_REBOOT_CMD_CAD_OFF => Ok(RebootAction::CadToggle),
        _ => Err(EINVAL),
    }
}

pub fn sys_reboot(magic1: i32, magic2: i32, cmd: u32, _arg: *mut u8) -> i64 {
    let action = match decode_reboot(magic1, magic2, cmd) {
        Ok(action) => action,
        Err(errno) => return -(errno as i64),
    };
    if unsafe { sched::get_current() }.is_null()
        || !crate::kernel::capability::capable(crate::kernel::capability::CAP_SYS_BOOT)
    {
        return -(EPERM as i64);
    }
    // The `isa-debug-exit` device and PS/2 reset port are wired by the test
    // harness QEMU command line (`add_qemu_default_devices`) on every public
    // boot, so the userspace `poweroff` / `reboot` paths always terminate the
    // emulator cleanly rather than spinning the CPU in a halt loop.  On real
    // hardware the I/O writes target unused ports and we fall through to the
    // halt loop inside `crate::linux_driver_abi::platform::qemu::*`.
    match action {
        RebootAction::PowerOff | RebootAction::Halt => {
            crate::linux_driver_abi::platform::qemu::exit_success()
        }
        RebootAction::Restart => crate::linux_driver_abi::platform::qemu::machine_restart(),
        RebootAction::CadToggle => 0,
    }
}

pub unsafe fn sys_getrusage(who: i32, usage: *mut RUsage) -> i64 {
    if !matches!(who, -1..=1) {
        return -(EINVAL as i64);
    }
    if usage.is_null() {
        return -(EFAULT as i64);
    }
    let value = RUsage::default();
    let left = unsafe {
        uaccess::copy_to_user(
            usage as *mut u8,
            &value as *const RUsage as *const u8,
            core::mem::size_of::<RUsage>(),
        )
    };
    if left == 0 { 0 } else { -(EFAULT as i64) }
}

pub unsafe fn sys_sysinfo(info: *mut SysInfo) -> i64 {
    if info.is_null() {
        return -(EFAULT as i64);
    }
    let value = SysInfo {
        uptime: (crate::kernel::time::ktime_get_boottime() / 1_000_000_000) as i64,
        totalram: crate::mm::mm_public::totalram_pages() * crate::mm::frame::PAGE_SIZE as u64,
        freeram: crate::mm::page_alloc::nr_free_buffer_pages() as u64
            * crate::mm::frame::PAGE_SIZE as u64,
        totalswap: (crate::mm::swap::total_swap_pages() as u64)
            * crate::mm::frame::PAGE_SIZE as u64,
        freeswap: (crate::mm::swap::free_swap_pages() as u64) * crate::mm::frame::PAGE_SIZE as u64,
        procs: 1,
        mem_unit: 1,
        ..SysInfo::default()
    };
    let left = unsafe {
        uaccess::copy_to_user(
            info as *mut u8,
            &value as *const SysInfo as *const u8,
            core::mem::size_of::<SysInfo>(),
        )
    };
    if left == 0 { 0 } else { -(EFAULT as i64) }
}

pub unsafe fn sys_times(buf: *mut Tms) -> i64 {
    if !buf.is_null() {
        let value = Tms::default();
        let left = unsafe {
            uaccess::copy_to_user(
                buf as *mut u8,
                &value as *const Tms as *const u8,
                core::mem::size_of::<Tms>(),
            )
        };
        if left != 0 {
            return -(EFAULT as i64);
        }
    }
    (crate::kernel::time::ktime_get_boottime() / 10_000_000) as i64
}

pub fn sys_personality(persona: u32) -> i64 {
    crate::kernel::exec_domain::sys_personality(persona)
}

pub fn sys_remap_file_pages(_start: u64, size: u64, _prot: u64, _pgoff: u64, flags: u64) -> i64 {
    if size == 0 || flags != 0 {
        return -(EINVAL as i64);
    }
    0
}

pub fn sys_semtimedop(_semid: i32, _sops: *mut u8, nsops: usize, _timeout: *const u8) -> i64 {
    if nsops == 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_mbind(
    _start: u64,
    len: u64,
    mode: u64,
    _nmask: *const u64,
    _maxnode: u64,
    flags: u32,
) -> i64 {
    crate::mm::mempolicy::mbind(len, mode, flags)
        .map(|_| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub fn sys_set_mempolicy(mode: i32, _nmask: *const u64, _maxnode: u64) -> i64 {
    crate::mm::mempolicy::set_mempolicy(mode)
        .map(|_| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub unsafe fn sys_get_mempolicy(
    policy: *mut i32,
    nmask: *mut u64,
    _maxnode: u64,
    _addr: u64,
    flags: u64,
) -> i64 {
    let current = match crate::mm::mempolicy::get_mempolicy(flags) {
        Ok(policy) => policy,
        Err(errno) => return -(errno as i64),
    };
    if !policy.is_null()
        && unsafe { uaccess::put_user_u32(policy as *mut u32, current.mode as u32) }.is_err()
    {
        return -(EFAULT as i64);
    }
    if !nmask.is_null() && unsafe { uaccess::put_user_u64(nmask, current.nodemask) }.is_err() {
        return -(EFAULT as i64);
    }
    0
}

pub fn sys_mq_timedsend(
    _mqdes: i32,
    msg_ptr: *const u8,
    msg_len: usize,
    _msg_prio: u32,
    _timeout: *const crate::kernel::time::Timespec64,
) -> i64 {
    if msg_len != 0 && msg_ptr.is_null() {
        return -(EFAULT as i64);
    }
    -(EBADF as i64)
}

pub fn sys_mq_timedreceive(
    _mqdes: i32,
    msg_ptr: *mut u8,
    msg_len: usize,
    msg_prio: *mut u32,
    _timeout: *const crate::kernel::time::Timespec64,
) -> i64 {
    if msg_len != 0 && msg_ptr.is_null() {
        return -(EFAULT as i64);
    }
    if msg_prio as u64 >= crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX {
        return -(EFAULT as i64);
    }
    -(EBADF as i64)
}

pub fn sys_migrate_pages(
    pid: i32,
    _maxnode: u64,
    _old_nodes: *const u64,
    _new_nodes: *const u64,
) -> i64 {
    crate::mm::mempolicy::migrate_pages(pid).unwrap_or_else(|errno| -(errno as i64))
}

pub fn sys_move_pages(
    pid: i32,
    nr_pages: usize,
    pages: *const u64,
    _nodes: *const i32,
    status: *mut i32,
    flags: i32,
) -> i64 {
    crate::mm::migration::move_pages(pid, nr_pages, pages, status, flags)
}

pub unsafe fn sys_set_tid_address(tidptr: *mut i32) -> i64 {
    match current_task() {
        Ok(task) => unsafe {
            (*task).m26.clear_child_tid = tidptr;
            (*task).pid as i64
        },
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_readahead(_fd: i32, _offset: i64, _count: usize) -> i64 {
    0
}

pub fn sys_fadvise64(_fd: i32, _offset: i64, _len: i64, advice: i32) -> i64 {
    crate::mm::backing_dev::apply_fadvise(advice)
        .map(|_| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub unsafe fn sys_getcpu(cpup: *mut u32, nodep: *mut u32) -> i64 {
    if !cpup.is_null() && unsafe { uaccess::put_user_u32(cpup, 0) }.is_err() {
        return -(EFAULT as i64);
    }
    if !nodep.is_null() && unsafe { uaccess::put_user_u32(nodep, 0) }.is_err() {
        return -(EFAULT as i64);
    }
    0
}

pub unsafe fn sys_rt_tgsigqueueinfo(tgid: i32, pid: i32, sig: i32, _uinfo: *const u8) -> i64 {
    crate::kernel::signal::sys_tgkill(tgid, pid, sig)
}

pub unsafe fn sys_name_to_handle_at(
    _dfd: i32,
    pathname: *const u8,
    handle: *mut FileHandle,
    mount_id: *mut i32,
    flags: i32,
) -> i64 {
    const AT_EMPTY_PATH: i32 = 0x1000;
    if flags & !AT_EMPTY_PATH != 0 {
        return -(EINVAL as i64);
    }
    if pathname.is_null() || handle.is_null() || mount_id.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOTSUP as i64)
}

pub fn sys_open_by_handle_at(_mount_fd: i32, handle: *mut FileHandle, flags: i32) -> i64 {
    if handle.is_null() {
        return -(EFAULT as i64);
    }
    if flags < 0 {
        return -(EINVAL as i64);
    }
    -(EPERM as i64)
}

pub unsafe fn sys_clock_adjtime(_clk_id: i32, txc_p: *mut Timex) -> i64 {
    unsafe { sys_adjtimex(txc_p) }
}

pub fn sys_process_vm_readv(
    pid: i32,
    lvec: *const u8,
    liovcnt: usize,
    rvec: *const u8,
    riovcnt: usize,
    flags: u64,
) -> i64 {
    if pid < 0 || flags != 0 {
        return -(EINVAL as i64);
    }
    if liovcnt == 0 || riovcnt == 0 {
        return 0;
    }
    let target = task_by_pid(pid);
    if target.is_null() {
        return -(ESRCH as i64);
    }
    let current = unsafe { sched::get_current() };
    if !current.is_null() && unsafe { (*target).mm != (*current).mm } {
        return -(EFAULT as i64);
    }
    unsafe {
        crate::mm::process_vm_access::process_vm_rw_same_mm(
            lvec as *const crate::mm::process_vm_access::ProcessIoVec,
            liovcnt,
            rvec as *const crate::mm::process_vm_access::ProcessIoVec,
            riovcnt,
            flags,
            false,
        )
    }
}

pub fn sys_process_vm_writev(
    pid: i32,
    lvec: *const u8,
    liovcnt: usize,
    rvec: *const u8,
    riovcnt: usize,
    flags: u64,
) -> i64 {
    if pid < 0 || flags != 0 {
        return -(EINVAL as i64);
    }
    if liovcnt == 0 || riovcnt == 0 {
        return 0;
    }
    let target = task_by_pid(pid);
    if target.is_null() {
        return -(ESRCH as i64);
    }
    let current = unsafe { sched::get_current() };
    if !current.is_null() && unsafe { (*target).mm != (*current).mm } {
        return -(EFAULT as i64);
    }
    unsafe {
        crate::mm::process_vm_access::process_vm_rw_same_mm(
            lvec as *const crate::mm::process_vm_access::ProcessIoVec,
            liovcnt,
            rvec as *const crate::mm::process_vm_access::ProcessIoVec,
            riovcnt,
            flags,
            true,
        )
    }
}

pub unsafe fn sys_kcmp(pid1: i32, pid2: i32, _typ: i32, _idx1: u64, _idx2: u64) -> i64 {
    if pid1 < 0 || pid2 < 0 {
        return -(EINVAL as i64);
    }
    if pid1 == pid2 {
        return 0;
    }
    if task_by_pid(pid1).is_null() || task_by_pid(pid2).is_null() {
        return -(ESRCH as i64);
    }
    0
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RseqUserArea {
    cpu_id_start: u32,
    cpu_id: u32,
    rseq_cs: u64,
    flags: u32,
    node_id: u32,
    mm_cid: u32,
    reserved: u32,
}

fn rseq_length_valid(rseq: *const u8, rseq_len: u32) -> bool {
    // linux-source: vendor/linux/kernel/rseq.c
    //
    // Lupos currently exposes only the original 32-byte ABI.  Extended rseq v2
    // fields need return-to-userspace slowpath support before they can be
    // advertised safely.
    rseq_len == RSEQ_ORIG_SIZE && (rseq as usize).is_multiple_of(RSEQ_ORIG_ALIGN)
}

fn write_rseq_area(rseq: *mut u8, area: &RseqUserArea) -> Result<(), i32> {
    if rseq.is_null() {
        return Err(EFAULT);
    }
    let left = unsafe {
        uaccess::copy_to_user(
            rseq,
            area as *const RseqUserArea as *const u8,
            core::mem::size_of::<RseqUserArea>(),
        )
    };
    if left == 0 { Ok(()) } else { Err(EFAULT) }
}

fn rseq_register(key: i32, rseq: *mut u8, rseq_len: u32, sig: u32) -> i64 {
    if !rseq_length_valid(rseq, rseq_len) {
        return -(EINVAL as i64);
    }
    if sched::production_smp_scheduler_enabled() {
        return -(ENOSYS as i64);
    }

    let cpu = sched::current_cpu();
    let area = RseqUserArea {
        cpu_id_start: cpu,
        cpu_id: cpu,
        rseq_cs: 0,
        flags: 0,
        node_id: 0,
        mm_cid: 0,
        reserved: 0,
    };
    if let Err(errno) = write_rseq_area(rseq, &area) {
        return -(errno as i64);
    }

    let entry = RseqRegistration {
        active: true,
        key,
        rseq: rseq as usize,
        len: rseq_len,
        sig,
    };
    match RSEQ_REGISTRY.lock().insert(entry) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

fn rseq_reregister(entry: RseqRegistration, rseq: *mut u8, rseq_len: u32, sig: u32) -> i64 {
    if entry.rseq != rseq as usize || entry.len != rseq_len || entry.sig != sig {
        return -(EINVAL as i64);
    }
    0
}

fn rseq_unregister(key: i32, rseq: *mut u8, rseq_len: u32, flags: i32, sig: u32) -> i64 {
    if flags & !RSEQ_FLAG_UNREGISTER != 0 {
        return -(EINVAL as i64);
    }

    let entry = match RSEQ_REGISTRY.lock().get(key) {
        Some(entry) => entry,
        None => return -(EINVAL as i64),
    };
    if entry.rseq != rseq as usize || entry.len != rseq_len || entry.sig != sig {
        return -(EINVAL as i64);
    }

    let reset = RseqUserArea {
        cpu_id_start: 0,
        cpu_id: RSEQ_CPU_ID_UNINITIALIZED,
        rseq_cs: 0,
        flags: 0,
        node_id: 0,
        mm_cid: 0,
        reserved: 0,
    };
    if let Err(errno) = write_rseq_area(rseq, &reset) {
        return -(errno as i64);
    }
    RSEQ_REGISTRY.lock().remove(key);
    0
}

pub fn sys_rseq(rseq: *mut u8, rseq_len: u32, flags: i32, sig: u32) -> i64 {
    // linux-source: vendor/linux/kernel/rseq.c
    //
    // Fail closed until the x86 return-to-userspace path has the matching
    // rseq resume/abort hooks. Returning success without those hooks lets
    // libc enter restartable-sequence fast paths that the kernel cannot keep
    // atomic across preemption.
    if !rseq_runtime_supported() {
        let _ = (rseq, rseq_len, flags, sig);
        return -(ENOSYS as i64);
    }

    if flags & RSEQ_FLAG_UNREGISTER != 0 {
        return rseq_unregister(current_rseq_key(), rseq, rseq_len, flags, sig);
    }
    if flags & !RSEQ_FLAGS_SUPPORTED != 0 {
        return -(EINVAL as i64);
    }

    let key = current_rseq_key();
    if let Some(entry) = RSEQ_REGISTRY.lock().get(key) {
        return rseq_reregister(entry, rseq, rseq_len, sig);
    }

    rseq_register(key, rseq, rseq_len, sig)
}

pub fn sys_pidfd_send_signal(pidfd: i32, sig: i32, info: *const u8, flags: u32) -> i64 {
    if !(0..=64).contains(&sig) || flags != 0 {
        return -(EINVAL as i64);
    }
    let target = match crate::fs::pidfd::task_for_fd(pidfd) {
        Ok(target) => target,
        Err(errno) => return -(errno as i64),
    };
    if sig == 0 {
        return 0;
    }

    let info = if info.is_null() {
        let current = unsafe { sched::get_current() };
        let sender_pid = if current.is_null() {
            0
        } else {
            unsafe { (*current).tgid }
        };
        let cred = crate::kernel::cred::current_cred();
        let sender_uid = if cred.is_null() {
            0
        } else {
            unsafe { (*cred).uid.0 }
        };
        crate::kernel::signal::SigInfo::with_sender(
            sig,
            crate::kernel::signal::SI_USER,
            sender_pid,
            sender_uid,
        )
    } else {
        let info =
            match unsafe { copy_struct_from_user(info.cast::<crate::kernel::signal::SigInfo>()) } {
                Ok(info) => info,
                Err(errno) => return -(errno as i64),
            };
        if info.signo != sig {
            return -(EINVAL as i64);
        }
        if info.code >= 0 {
            return -(EPERM as i64);
        }
        info
    };

    // A pidfd opened without PIDFD_THREAD identifies a thread group.  Linux
    // therefore uses PIDTYPE_TGID here, placing the signal on
    // signal_struct::shared_pending and waking an eligible interruptible
    // thread.  A task-private enqueue can leave a service asleep in poll(2)
    // forever even though TIF_SIGPENDING was set on its leader.
    crate::kernel::signal::send_signal_info_to_process_for_target(target, sig, info) as i64
}

pub fn sys_process_madvise(
    _pidfd: i32,
    _iovec: *const u8,
    _vlen: usize,
    _advice: i32,
    flags: u32,
) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    -(EBADF as i64)
}

pub fn sys_process_mrelease(pidfd: i32, flags: u32) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    let target = match crate::fs::pidfd::task_for_fd(pidfd) {
        Ok(target) => target,
        Err(errno) => return -(errno as i64),
    };
    if target.is_null() {
        return -(ESRCH as i64);
    }

    let (pid, mm, exited) = unsafe {
        let state = (*target).m26.exit_state
            | (*target)
                .__state
                .load(core::sync::atomic::Ordering::Acquire);
        (
            (*target).pid,
            (*target).mm,
            state
                & (crate::kernel::task::task_state::EXIT_ZOMBIE
                    | crate::kernel::task::task_state::EXIT_DEAD)
                != 0,
        )
    };
    if exited || mm.is_null() {
        return -(ESRCH as i64);
    }
    if !crate::kernel::signal::has_pending_signal_for_pid(pid, crate::kernel::signal::SIGKILL) {
        return -(EINVAL as i64);
    }

    unsafe {
        reap_process_mrelease_mm(mm);
    }
    0
}

unsafe fn reap_process_mrelease_mm(mm: *mut crate::mm::mm_types::MmStruct) {
    if mm.is_null() {
        return;
    }
    let entries = unsafe { (*mm).mm_mt.collect_entries() };
    for (start, end_inclusive, _) in entries {
        let end = end_inclusive.saturating_add(1);
        if end > start {
            unsafe {
                crate::mm::mmap::unmap_page_range(&mut *mm, start, end);
            }
        }
    }
}

pub fn sys_set_mempolicy_home_node(_start: u64, len: u64, _home_node: u64, flags: u64) -> i64 {
    crate::mm::mempolicy::set_mempolicy_home_node(len, flags)
        .map(|_| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub unsafe fn sys_cachestat(
    _fd: u32,
    cstat_range: *const CacheStatRange,
    cstat: *mut CacheStat,
    flags: u32,
) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    if cstat_range.is_null() || cstat.is_null() {
        return -(EFAULT as i64);
    }
    let value = CacheStat::default();
    let left = unsafe {
        uaccess::copy_to_user(
            cstat as *mut u8,
            &value as *const CacheStat as *const u8,
            core::mem::size_of::<CacheStat>(),
        )
    };
    if left == 0 { 0 } else { -(EFAULT as i64) }
}

pub fn sys_map_shadow_stack(addr: u64, size: u64, flags: u32) -> i64 {
    crate::arch::x86::kernel::shstk::sys_map_shadow_stack(addr, size, flags)
}

pub fn sys_memfd_secret(flags: u32) -> i64 {
    crate::fs::syscalls::sys_memfd_secret(flags)
}

pub fn sys_mseal(start: u64, len: u64, flags: u64) -> i64 {
    crate::mm::mlock::seal_range(start, len, flags)
        .map(|_| 0)
        .unwrap_or_else(|errno| -(errno as i64))
}

pub fn sys_listns(_pidfd: i32, _nstype: u64, info: *mut u8, _size: *mut u32, flags: u32) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    if info.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_uretprobe() -> i64 {
    crate::arch::x86::kernel::uprobes::sys_uretprobe()
}

pub fn sys_uprobe() -> i64 {
    crate::arch::x86::kernel::uprobes::sys_uprobe()
}

pub fn sys_rseq_slice_yield(_cpu: i32, _node: i32, flags: u64) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    0
}

pub fn sys_shmget(_key: i32, size: usize, _shmflg: i32) -> i64 {
    if size == 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_shmat(_shmid: i32, _shmaddr: u64, shmflg: i32) -> i64 {
    if shmflg & !0x3fff != 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_shmctl(_shmid: i32, cmd: i32, _buf: *mut u8) -> i64 {
    if cmd < 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_shmdt(shmaddr: u64) -> i64 {
    if shmaddr == 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_semget(_key: i32, nsems: i32, _semflg: i32) -> i64 {
    if nsems < 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_semop(_semid: i32, _sops: *mut u8, nsops: usize) -> i64 {
    sys_semtimedop(0, core::ptr::null_mut(), nsops, core::ptr::null())
}

pub fn sys_semctl(_semid: i32, _semnum: i32, cmd: i32, _arg: u64) -> i64 {
    if cmd < 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_msgget(_key: i32, _msgflg: i32) -> i64 {
    -(ENOSYS as i64)
}

pub fn sys_msgsnd(_msqid: i32, msgp: *const u8, msgsz: usize, _msgflg: i32) -> i64 {
    if msgsz != 0 && msgp.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_msgrcv(_msqid: i32, msgp: *mut u8, msgsz: usize, _msgtyp: i64, _msgflg: i32) -> i64 {
    if msgsz != 0 && msgp.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_msgctl(_msqid: i32, cmd: i32, _buf: *mut u8) -> i64 {
    if cmd < 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_syslog(typ: i32, buf: *mut u8, len: i32) -> i64 {
    if !(0..=10).contains(&typ) || len < 0 {
        return -(EINVAL as i64);
    }
    if len != 0 && buf.is_null() {
        return -(EFAULT as i64);
    }
    0
}

pub fn sys_acct(name: *const u8) -> i64 {
    if name.is_null() {
        return 0;
    }
    -(EPERM as i64)
}

pub fn sys_io_setup(nr_events: u32, ctxp: *mut u64) -> i64 {
    if nr_events == 0 {
        return -(EINVAL as i64);
    }
    if ctxp.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_io_destroy(_ctx: u64) -> i64 {
    -(EINVAL as i64)
}

pub fn sys_io_getevents(
    _ctx: u64,
    min_nr: i64,
    nr: i64,
    _events: *mut u8,
    _timeout: *const crate::kernel::time::Timespec64,
) -> i64 {
    if min_nr < 0 || nr < 0 || min_nr > nr {
        return -(EINVAL as i64);
    }
    0
}

pub fn sys_io_submit(_ctx: u64, nr: i64, iocbpp: *mut u8) -> i64 {
    if nr < 0 {
        return -(EINVAL as i64);
    }
    if nr != 0 && iocbpp.is_null() {
        return -(EFAULT as i64);
    }
    -(EINVAL as i64)
}

pub fn sys_io_cancel(_ctx: u64, iocb: *mut u8, _result: *mut u8) -> i64 {
    if iocb.is_null() {
        return -(EFAULT as i64);
    }
    -(EINVAL as i64)
}

pub fn sys_io_pgetevents(
    ctx: u64,
    min_nr: i64,
    nr: i64,
    events: *mut u8,
    timeout: *const crate::kernel::time::Timespec64,
    _sig: *const u8,
) -> i64 {
    sys_io_getevents(ctx, min_nr, nr, events, timeout)
}

pub fn sys_mq_open(name: *const u8, _oflag: i32, _mode: u32, _attr: *const u8) -> i64 {
    if name.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_mq_unlink(name: *const u8) -> i64 {
    if name.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_mq_notify(_mqdes: i32, _sevp: *const u8) -> i64 {
    -(EBADF as i64)
}

pub fn sys_mq_getsetattr(_mqdes: i32, _new: *const u8, old: *mut u8) -> i64 {
    if old as u64 >= crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX {
        return -(EFAULT as i64);
    }
    -(EBADF as i64)
}

pub fn sys_kexec_load(_entry: u64, nr_segments: u64, _segments: *const u8, flags: u64) -> i64 {
    if nr_segments > 16 || flags & !0xffff != 0 {
        return -(EINVAL as i64);
    }
    -(EPERM as i64)
}

pub fn sys_kexec_file_load(
    _kernel_fd: i32,
    _initrd_fd: i32,
    _cmdline_len: u64,
    _cmdline: *const u8,
    flags: u64,
) -> i64 {
    if flags & !0x7 != 0 {
        return -(EINVAL as i64);
    }
    -(EPERM as i64)
}

pub fn sys_pidfd_open(pid: i32, flags: u32) -> i64 {
    if pid <= 0 || flags != 0 {
        return -(EINVAL as i64);
    }
    let task = task_by_pid(pid);
    if task.is_null() {
        return -(ESRCH as i64);
    }
    match crate::fs::pidfd::install_pidfd(task, false) {
        Ok(fd) => fd as i64,
        Err(errno) => -(errno as i64),
    }
}

pub fn sys_pidfd_getfd(_pidfd: i32, _targetfd: i32, flags: u32) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    -(EBADF as i64)
}

pub fn sys_lsm_get_self_attr(attr: u32, ctx: *mut u8, size: *mut u32, flags: u32) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    if attr == 0 || ctx.is_null() || size.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_lsm_set_self_attr(attr: u32, ctx: *const u8, size: u32, flags: u32) -> i64 {
    if flags != 0 || size == 0 {
        return -(EINVAL as i64);
    }
    if attr == 0 || ctx.is_null() {
        return -(EFAULT as i64);
    }
    -(ENOSYS as i64)
}

pub fn sys_lsm_list_modules(ids: *mut u64, size: *mut u32, flags: u32) -> i64 {
    if flags != 0 {
        return -(EINVAL as i64);
    }
    if size.is_null() {
        return -(EFAULT as i64);
    }
    let user_size = match unsafe { uaccess::get_user_u32(size as *const u32) } {
        Ok(value) => value,
        Err(_) => return -(EFAULT as i64),
    };
    let mut active = [0u64; 16];
    let count = crate::security::lsm_active_ids(&mut active);
    let total_size = (count * core::mem::size_of::<u64>()) as u32;
    if unsafe { uaccess::put_user_u32(size, total_size) }.is_err() {
        return -(EFAULT as i64);
    }
    if user_size < total_size {
        return -(E2BIG as i64);
    }
    if total_size != 0 && ids.is_null() {
        return -(EFAULT as i64);
    }
    for (i, id) in active.iter().copied().take(count).enumerate() {
        if unsafe { uaccess::put_user_u64(ids.wrapping_add(i), id) }.is_err() {
            return -(EFAULT as i64);
        }
    }
    count as i64
}

pub unsafe fn sys_prlimit64(
    pid: i32,
    resource: i32,
    new_rlim: *const RLimit,
    old_rlim: *mut RLimit,
) -> i64 {
    if pid != 0 {
        return -(ESRCH as i64);
    }
    if !(0..RLIM_NLIMITS).contains(&resource) {
        return -(EINVAL as i64);
    }
    let new_limit = if !new_rlim.is_null() {
        let new = unsafe { *new_rlim };
        if new.rlim_cur > new.rlim_max {
            return -(EINVAL as i64);
        }
        Some(new)
    } else {
        None
    };
    if !old_rlim.is_null() {
        let old = current_rlimit(resource);
        let left = unsafe {
            uaccess::copy_to_user(
                old_rlim as *mut u8,
                &old as *const RLimit as *const u8,
                core::mem::size_of::<RLimit>(),
            )
        };
        if left != 0 {
            return -(EFAULT as i64);
        }
    }
    if let Some(new) = new_limit {
        set_current_rlimit(resource, new);
    }
    0
}

fn valid_ioprio(ioprio: i32) -> bool {
    let class = (ioprio >> 13) & 0x7;
    let data = ioprio & 0x1fff;
    matches!(
        class,
        IOPRIO_CLASS_NONE | IOPRIO_CLASS_RT | IOPRIO_CLASS_BE | IOPRIO_CLASS_IDLE
    ) && data <= 7
}

pub unsafe fn sys_ioprio_set(_which: i32, _who: i32, ioprio: i32) -> i64 {
    if !valid_ioprio(ioprio) {
        return -(EINVAL as i64);
    }
    0
}

pub unsafe fn sys_ioprio_get(_which: i32, _who: i32) -> i64 {
    4
}

pub unsafe fn sys_iopl(level: u32) -> i64 {
    if level > 3 {
        return -(EINVAL as i64);
    }
    0
}

pub unsafe fn sys_ioperm(_from: u64, _num: u64, turn_on: i32) -> i64 {
    if turn_on != 0 && turn_on != 1 {
        return -(EINVAL as i64);
    }
    0
}

pub unsafe fn sys_setrlimit(resource: i32, rlim: *const RLimit) -> i64 {
    if !(0..RLIM_NLIMITS).contains(&resource) {
        return -(EINVAL as i64);
    }
    if rlim.is_null() {
        return -(crate::include::uapi::errno::EFAULT as i64);
    }
    let limit = unsafe { *rlim };
    if limit.rlim_cur > limit.rlim_max {
        return -(EINVAL as i64);
    }
    set_current_rlimit(resource, limit);
    0
}

pub unsafe fn sys_quotactl(_cmd: u32, _special: *const u8, _id: i32, _addr: *mut u8) -> i64 {
    0
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::fs::fdtable::FilesStruct;
    use crate::kernel::capability::{
        CAP_CHOWN, CAP_SETGID, CAP_SETUID, KERNEL_CAPABILITY_U32S, LINUX_CAPABILITY_VERSION_3,
        UserCapData, UserCapHeader, sys_capget, sys_capset,
    };
    use crate::kernel::pid::{INIT_PID_NS, alloc_pid};
    use crate::kernel::seccomp::{
        PR_CAP_AMBIENT, PR_CAP_AMBIENT_IS_SET, PR_CAP_AMBIENT_RAISE, PR_GET_KEEPCAPS,
        PR_SET_KEEPCAPS, sys_prctl,
    };
    use crate::kernel::signal::{SIGCONT, SIGKILL, SIGTSTP, has_pending_signal_for_pid};
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};

    static RSEQ_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());
    static ITIMER_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[repr(C, align(32))]
    struct AlignedRseqArea(RseqUserArea);

    fn zeroed_task(pid: i32) -> Box<TaskStruct> {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.pid = pid;
        task.tgid = pid;
        task
    }

    fn invalid_user_ptr<T>() -> *const T {
        (crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX as usize + 1) as *const T
    }

    fn invalid_user_mut_ptr<T>() -> *mut T {
        invalid_user_ptr::<T>() as *mut T
    }

    #[test]
    fn real_itimer_callback_does_not_restart_periodic_timer() {
        let _guard = ITIMER_TEST_LOCK.lock();
        let mut timer = crate::kernel::time::hrtimer::Hrtimer::new();
        timer.data = 0;

        assert_eq!(
            real_itimer_fired(&mut timer as *mut _),
            crate::kernel::time::hrtimer::HrtimerRestart::NoRestart
        );
    }

    #[test]
    fn real_itimer_disarm_clears_periodic_interval() {
        let _guard = ITIMER_TEST_LOCK.lock();
        let pid = 88_001;
        release_task_real_itimer(pid);

        let _ = arm_real_itimer(0, 1_000_000_000, pid);

        let timers = REAL_ITIMERS.lock();
        let state = timers.get(&pid).expect("real itimer state");
        assert_eq!(state.interval_ns.load(Ordering::Acquire), 0);
        assert_eq!(
            crate::kernel::time::hrtimer::hrtimer_state_snapshot(state.timer_ptr()),
            crate::kernel::time::hrtimer::HRTIMER_STATE_INACTIVE
        );
        drop(timers);
        release_task_real_itimer(pid);
    }

    #[test]
    fn sigalrm_delivery_rearms_periodic_real_itimer() {
        let _guard = ITIMER_TEST_LOCK.lock();
        let pid = 88_002;
        release_task_real_itimer(pid);

        let _ = arm_real_itimer(1_000_000_000, 1_000_000_000, pid);
        {
            let timers = REAL_ITIMERS.lock();
            let state = timers.get(&pid).expect("real itimer state");
            state.cancel_synchronously();
            let timer = unsafe { &mut *state.timer_ptr() };
            timer.expires_ns = timer.base_now().saturating_sub(1);
            state.interval_ns.store(1_000_000_000, Ordering::Release);
            timer.state = crate::kernel::time::hrtimer::HRTIMER_STATE_INACTIVE;
        }

        rearm_real_itimer_after_sigalrm(pid);

        let timers = REAL_ITIMERS.lock();
        let state = timers.get(&pid).expect("real itimer state");
        assert_eq!(
            crate::kernel::time::hrtimer::hrtimer_state_snapshot(state.timer_ptr()),
            crate::kernel::time::hrtimer::HRTIMER_STATE_ENQUEUED
        );
        assert!(crate::kernel::time::hrtimer::hrtimer_get_remaining(state.timer_ptr()) > 0);
        state.cancel_synchronously();
        drop(timers);
        release_task_real_itimer(pid);
    }

    #[test]
    fn linux_utsname_matches_new_utsname_layout() {
        assert_eq!(
            core::mem::size_of::<LinuxUtsname>(),
            core::mem::size_of::<NewUtsname>()
        );
        let name = build_utsname();
        assert_eq!(&name.sysname[..5], b"Lupos");
        assert_eq!(&name.machine[..6], b"x86_64");
    }

    #[test]
    fn old_time_struct_layouts_match_x86_64_linux() {
        assert_eq!(core::mem::size_of::<TimeVal>(), 16);
        assert_eq!(core::mem::size_of::<TimeZone>(), 8);
    }

    /// Source-backed parity check for `reboot(2)` magic/command dispatch.
    /// Ref: vendor/linux/include/uapi/linux/reboot.h and
    /// vendor/linux/kernel/reboot.c::__do_sys_reboot.
    #[test]
    fn reboot_decoder_matches_linux_uapi_constants() {
        // Linux uapi constant values must round-trip through the dispatcher.
        assert_eq!(LINUX_REBOOT_MAGIC1, 0xfee1dead_u32 as i32);
        assert_eq!(LINUX_REBOOT_MAGIC2, 672274793);
        assert_eq!(LINUX_REBOOT_MAGIC2A, 85072278);
        assert_eq!(LINUX_REBOOT_MAGIC2B, 369367448);
        assert_eq!(LINUX_REBOOT_MAGIC2C, 537993216);
        assert_eq!(LINUX_REBOOT_CMD_RESTART, 0x0123_4567);
        assert_eq!(LINUX_REBOOT_CMD_HALT, 0xCDEF_0123);
        assert_eq!(LINUX_REBOOT_CMD_POWER_OFF, 0x4321_FEDC);
        assert_eq!(LINUX_REBOOT_CMD_RESTART2, 0xA1B2_C3D4);
        assert_eq!(LINUX_REBOOT_CMD_CAD_ON, 0x89AB_CDEF);
        assert_eq!(LINUX_REBOOT_CMD_CAD_OFF, 0x0000_0000);

        // Bad magic1 is rejected before the cmd switch.
        assert_eq!(
            decode_reboot(0, LINUX_REBOOT_MAGIC2, LINUX_REBOOT_CMD_RESTART),
            Err(EINVAL)
        );
        // Bad magic2 is rejected even if magic1 is correct.
        assert_eq!(
            decode_reboot(LINUX_REBOOT_MAGIC1, 0, LINUX_REBOOT_CMD_RESTART),
            Err(EINVAL)
        );
        // Each accepted magic2 variant unlocks the cmd dispatch.
        for magic2 in [
            LINUX_REBOOT_MAGIC2,
            LINUX_REBOOT_MAGIC2A,
            LINUX_REBOOT_MAGIC2B,
            LINUX_REBOOT_MAGIC2C,
        ] {
            assert_eq!(
                decode_reboot(LINUX_REBOOT_MAGIC1, magic2, LINUX_REBOOT_CMD_RESTART),
                Ok(RebootAction::Restart)
            );
        }
        // Cmd dispatch matches Linux's `__do_sys_reboot` switch.
        assert_eq!(
            decode_reboot(
                LINUX_REBOOT_MAGIC1,
                LINUX_REBOOT_MAGIC2,
                LINUX_REBOOT_CMD_RESTART2
            ),
            Ok(RebootAction::Restart)
        );
        assert_eq!(
            decode_reboot(
                LINUX_REBOOT_MAGIC1,
                LINUX_REBOOT_MAGIC2,
                LINUX_REBOOT_CMD_HALT
            ),
            Ok(RebootAction::Halt)
        );
        assert_eq!(
            decode_reboot(
                LINUX_REBOOT_MAGIC1,
                LINUX_REBOOT_MAGIC2,
                LINUX_REBOOT_CMD_POWER_OFF
            ),
            Ok(RebootAction::PowerOff)
        );
        assert_eq!(
            decode_reboot(
                LINUX_REBOOT_MAGIC1,
                LINUX_REBOOT_MAGIC2,
                LINUX_REBOOT_CMD_CAD_ON
            ),
            Ok(RebootAction::CadToggle)
        );
        assert_eq!(
            decode_reboot(
                LINUX_REBOOT_MAGIC1,
                LINUX_REBOOT_MAGIC2,
                LINUX_REBOOT_CMD_CAD_OFF
            ),
            Ok(RebootAction::CadToggle)
        );
        // Unknown cmd words are rejected with EINVAL, mirroring Linux.
        assert_eq!(
            decode_reboot(LINUX_REBOOT_MAGIC1, LINUX_REBOOT_MAGIC2, 0xDEAD_BEEF),
            Err(EINVAL)
        );
    }

    #[test]
    fn ioprio_rejects_bad_class_and_data() {
        assert_eq!(unsafe { sys_ioprio_set(1, 0, 2 << 13) }, 0);
        assert_eq!(unsafe { sys_ioprio_set(1, 0, 4 << 13) }, -(EINVAL as i64));
        assert_eq!(
            unsafe { sys_ioprio_set(1, 0, (2 << 13) | 8) },
            -(EINVAL as i64)
        );
    }

    #[test]
    fn iopl_level_is_bounded() {
        assert_eq!(unsafe { sys_iopl(3) }, 0);
        assert_eq!(unsafe { sys_iopl(4) }, -(EINVAL as i64));
    }

    #[test]
    fn setrlimit_rejects_soft_above_hard() {
        let ok = RLimit {
            rlim_cur: 10,
            rlim_max: 10,
        };
        let bad = RLimit {
            rlim_cur: 11,
            rlim_max: 10,
        };
        assert_eq!(unsafe { sys_setrlimit(0, &ok) }, 0);
        assert_eq!(unsafe { sys_setrlimit(0, &bad) }, -(EINVAL as i64));
    }

    #[test]
    fn syscall_m76_resource_control_parity() {
        let ok = RLimit {
            rlim_cur: 10,
            rlim_max: 10,
        };
        let bad = RLimit {
            rlim_cur: 11,
            rlim_max: 10,
        };
        assert_eq!(unsafe { sys_setrlimit(0, &ok) }, 0);
        assert_eq!(unsafe { sys_setrlimit(0, &bad) }, -(EINVAL as i64));
        assert_eq!(
            unsafe { sys_setrlimit(RLIM_NLIMITS, &ok) },
            -(EINVAL as i64)
        );
        assert_eq!(
            unsafe { sys_setrlimit(0, core::ptr::null()) },
            -(EFAULT as i64)
        );

        let mut old = RLimit::default();
        assert_eq!(unsafe { sys_prlimit64(0, 0, &ok, &mut old) }, 0);
        assert_eq!(old.rlim_cur, u64::MAX);
        assert_eq!(
            unsafe { sys_prlimit64(9999, 0, core::ptr::null(), core::ptr::null_mut()) },
            -(ESRCH as i64)
        );
        assert_eq!(
            unsafe { sys_prlimit64(0, RLIM_NLIMITS, core::ptr::null(), core::ptr::null_mut()) },
            -(EINVAL as i64)
        );

        assert_eq!(unsafe { sys_iopl(3) }, 0);
        assert_eq!(unsafe { sys_iopl(4) }, -(EINVAL as i64));
        assert_eq!(unsafe { sys_ioperm(0, 8, 1) }, 0);
        assert_eq!(unsafe { sys_ioperm(0, 8, 2) }, -(EINVAL as i64));

        assert_eq!(unsafe { sys_ioprio_set(1, 0, 2 << 13) }, 0);
        assert_eq!(unsafe { sys_ioprio_set(1, 0, 4 << 13) }, -(EINVAL as i64));
        assert_eq!(
            unsafe { sys_ioprio_set(1, 0, (2 << 13) | 8) },
            -(EINVAL as i64)
        );
        assert_eq!(unsafe { sys_ioprio_get(1, 0) }, 4);
    }

    #[test]
    fn syscall_m76_admin_memory_misc_parity() {
        assert_eq!(unsafe { sys_sysfs(1, 0, 0) }, 0);
        assert_eq!(unsafe { sys_sysfs(2, 0, 0) }, -(EINVAL as i64));
        assert_eq!(sys_vhangup(), -(EPERM as i64));

        let mut ldt = [0u8; core::mem::size_of::<crate::arch::x86::kernel::ldt::UserDesc>()];
        assert_eq!(unsafe { sys_modify_ldt(0, core::ptr::null_mut(), 0) }, 0);
        assert_eq!(unsafe { sys_modify_ldt(1, ldt.as_mut_ptr(), ldt.len()) }, 0);
        assert_eq!(
            unsafe { sys_modify_ldt(1, core::ptr::null_mut(), ldt.len()) },
            -(EFAULT as i64)
        );
        assert_eq!(
            unsafe { sys_modify_ldt(2, ldt.as_mut_ptr(), ldt.len()) },
            ldt.len() as i64
        );

        let mut tx = Timex::default();
        assert_eq!(unsafe { sys_adjtimex(&mut tx) }, 0);
        assert!(tx.time.tv_sec >= 0);
        assert_eq!(
            unsafe { sys_adjtimex(core::ptr::null_mut()) },
            -(EFAULT as i64)
        );
        assert_eq!(
            unsafe { sys_adjtimex(invalid_user_mut_ptr()) },
            -(EFAULT as i64)
        );

        let tv = TimeVal {
            tv_sec: 1,
            tv_usec: 999_999,
        };
        assert_eq!(
            unsafe { sys_settimeofday(&tv, core::ptr::null()) },
            -(EPERM as i64)
        );
        let bad_tv = TimeVal {
            tv_sec: -1,
            tv_usec: 0,
        };
        assert_eq!(
            unsafe { sys_settimeofday(&bad_tv, core::ptr::null()) },
            -(EINVAL as i64)
        );
        assert_eq!(
            unsafe { sys_settimeofday(invalid_user_ptr(), core::ptr::null()) },
            -(EFAULT as i64)
        );

        assert_eq!(sys_swapon(core::ptr::null(), 0), -(EFAULT as i64));
        let path = b"/swapfile\0";
        assert_eq!(
            sys_swapon(path.as_ptr(), (SWAP_FLAGS_VALID | 0x8000_0000) as i32),
            -(EINVAL as i64)
        );
        assert_eq!(sys_swapoff(core::ptr::null()), -(EFAULT as i64));
        assert_eq!(sys_reboot(0, 0, 0, core::ptr::null_mut()), -(EINVAL as i64));
        assert_eq!(
            sys_reboot(0xfee1dead_u32 as i32, 672274793, 0, core::ptr::null_mut()),
            -(EPERM as i64)
        );
        // Bogus cmd value with otherwise valid magic must surface EINVAL via
        // decode_reboot before the capability check fires.
        assert_eq!(
            sys_reboot(
                LINUX_REBOOT_MAGIC1,
                LINUX_REBOOT_MAGIC2,
                0xDEAD_BEEF,
                core::ptr::null_mut(),
            ),
            -(EINVAL as i64)
        );

        let host = b"lupos-test";
        assert_eq!(
            unsafe { crate::kernel::utsname::sys_sethostname(host.as_ptr(), host.len()) },
            0
        );
        assert_eq!(
            unsafe { crate::kernel::utsname::sys_sethostname(core::ptr::null(), host.len(),) },
            -(EFAULT as i64)
        );

        assert_eq!(unsafe { crate::mm::syscalls::sys_mlock(0x1000, 4096) }, 0);
        assert_eq!(
            unsafe { crate::mm::syscalls::sys_mlock(0, 4096) },
            -(EINVAL as i64)
        );
        assert_eq!(unsafe { crate::mm::syscalls::sys_munlock(0x1000, 4096) }, 0);
        assert_eq!(crate::mm::syscalls::sys_mlockall(1 | 2), 0);
        assert_eq!(crate::mm::syscalls::sys_mlockall(8), -(EINVAL as i64));
        assert_eq!(crate::mm::syscalls::sys_munlockall(), 0);
        assert_eq!(
            unsafe { crate::mm::syscalls::sys_mlock2(0x1000, 4096, 1) },
            0
        );
        assert_eq!(
            unsafe { crate::mm::syscalls::sys_mlock2(0x1000, 4096, 2) },
            -(EINVAL as i64)
        );
        assert!(
            crate::mm::syscalls::sys_membarrier(0, 0, 0)
                & crate::kernel::sched::membarrier::MEMBARRIER_CMD_GLOBAL as i64
                != 0
        );
        assert_eq!(crate::mm::syscalls::sys_pkey_alloc(0, 0), 1);
        assert_eq!(crate::mm::syscalls::sys_pkey_alloc(1, 0), -(EINVAL as i64));
        assert_eq!(crate::mm::syscalls::sys_pkey_free(1), 0);
        assert_eq!(crate::mm::syscalls::sys_pkey_free(-1), -(EINVAL as i64));
    }

    #[test]
    fn syscall_m76_identity_time_parity() {
        let previous = unsafe { sched::get_current() };
        let mut parent = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        parent.pid = 41;
        parent.tgid = 41;
        parent.cred = &raw const INIT_CRED;

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 42;
        current.tgid = 40;
        current.cred = &raw const INIT_CRED;
        current.m26.real_parent = &mut *parent as *mut TaskStruct;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(sys_getpid(), 40);
            assert_eq!(sys_gettid(), 42);
            assert_eq!(sys_getppid(), 41);
            assert_eq!(sys_getpgrp(), 42);
            assert_eq!(sys_getsid(0), 42);
            assert_eq!(sys_getuid(), 0);
            assert_eq!(sys_getgid(), 0);
            assert_eq!(sys_geteuid(), 0);
            assert_eq!(sys_getegid(), 0);

            assert_eq!(sys_uname(core::ptr::null_mut()), -(EFAULT as i64));
            let mut uts = LinuxUtsname {
                sysname: [0; NEW_UTS_LEN_PLUS_NUL],
                nodename: [0; NEW_UTS_LEN_PLUS_NUL],
                release: [0; NEW_UTS_LEN_PLUS_NUL],
                version: [0; NEW_UTS_LEN_PLUS_NUL],
                machine: [0; NEW_UTS_LEN_PLUS_NUL],
                domainname: [0; NEW_UTS_LEN_PLUS_NUL],
            };
            assert_eq!(sys_uname(&mut uts), 0);
            assert_eq!(&uts.sysname[..5], b"Lupos");
            assert_eq!(&uts.machine[..6], b"x86_64");

            let mut sec = -1;
            let time_ret = sys_time(&mut sec);
            assert!(time_ret >= 0);
            assert_eq!(sec, time_ret);
            assert_eq!(sys_time(core::ptr::null_mut()), time_ret);

            let mut tv = TimeVal::default();
            let mut tz = TimeZone {
                tz_minuteswest: -1,
                tz_dsttime: -1,
            };
            assert_eq!(sys_gettimeofday(&mut tv, &mut tz), 0);
            assert!(tv.tv_sec >= 0);
            assert!((0..1_000_000).contains(&tv.tv_usec));
            assert_eq!(tz, TimeZone::default());

            let old_umask = sys_umask(0o1777);
            assert_eq!(sys_umask(0o022), 0o777);
            let _ = sys_umask(old_umask as u32);

            let mut limit = RLimit::default();
            assert_eq!(sys_getrlimit(0, &mut limit), 0);
            assert_eq!(limit.rlim_cur, u64::MAX);
            assert_eq!(limit.rlim_max, u64::MAX);
            assert_eq!(sys_getrlimit(RLIM_NLIMITS, &mut limit), -(EINVAL as i64));
            assert_eq!(sys_getrlimit(0, core::ptr::null_mut()), -(EFAULT as i64));

            let mut usage = RUsage::default();
            assert_eq!(sys_getrusage(0, &mut usage), 0);
            assert_eq!(usage.ru_utime, TimeVal::default());
            assert_eq!(sys_getrusage(2, &mut usage), -(EINVAL as i64));
            assert_eq!(sys_getrusage(0, core::ptr::null_mut()), -(EFAULT as i64));

            let mut info = SysInfo::default();
            assert_eq!(sys_sysinfo(&mut info), 0);
            assert_eq!(info.mem_unit, 1);
            assert!(info.totalram >= info.freeram);
            assert_eq!(sys_sysinfo(core::ptr::null_mut()), -(EFAULT as i64));

            let mut tms = Tms {
                tms_utime: -1,
                tms_stime: -1,
                tms_cutime: -1,
                tms_cstime: -1,
            };
            assert!(sys_times(&mut tms) >= 0);
            assert_eq!(tms.tms_utime, 0);
            assert!(sys_times(core::ptr::null_mut()) >= 0);

            let mut bytes = [0u8; 24];
            assert_eq!(
                sys_getrandom(bytes.as_mut_ptr(), 0, 0x8000),
                -(EINVAL as i64)
            );
            assert_eq!(sys_getrandom(core::ptr::null_mut(), 1, 0), -(EFAULT as i64));
            assert_eq!(sys_getrandom(bytes.as_mut_ptr(), 0, 0), 0);
            assert_eq!(
                sys_getrandom(bytes.as_mut_ptr(), bytes.len(), 0),
                bytes.len() as i64
            );
            assert!(bytes.iter().any(|&byte| byte != 0));

            crate::fs::fs_struct::exit_fs(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m76_credentials_session_sched_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 53;
        current.tgid = 53;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(sys_setuid(1000), 0);
            assert_eq!(sys_getuid(), 1000);
            assert_eq!(sys_geteuid(), 1000);

            current.cred = &raw const INIT_CRED;
            assert_eq!(sys_setresuid(1001, 1002, 1003), 0);
            let (mut ruid, mut euid, mut suid) = (0, 0, 0);
            assert_eq!(sys_getresuid(&mut ruid, &mut euid, &mut suid), 0);
            assert_eq!((ruid, euid, suid), (1001, 1002, 1003));
            assert_eq!(
                sys_getresuid(core::ptr::null_mut(), &mut euid, &mut suid),
                -(EFAULT as i64)
            );
            assert_eq!(sys_setfsuid(1001), 1002);
            assert_eq!(sys_setreuid(u32::MAX, 1003), 0);
            assert_eq!(sys_geteuid(), 1003);

            current.cred = &raw const INIT_CRED;
            assert_eq!(sys_setgid(3000), 0);
            assert_eq!(sys_getgid(), 3000);
            assert_eq!(sys_getegid(), 3000);

            current.cred = &raw const INIT_CRED;
            assert_eq!(sys_setresgid(3001, 3002, 3003), 0);
            let (mut rgid, mut egid, mut sgid) = (0, 0, 0);
            assert_eq!(sys_getresgid(&mut rgid, &mut egid, &mut sgid), 0);
            assert_eq!((rgid, egid, sgid), (3001, 3002, 3003));
            assert_eq!(
                sys_getresgid(core::ptr::null_mut(), &mut egid, &mut sgid),
                -(EFAULT as i64)
            );
            assert_eq!(sys_setfsgid(3001), 3002);
            assert_eq!(sys_setregid(u32::MAX, 3003), 0);
            assert_eq!(sys_getegid(), 3003);

            current.cred = &raw const INIT_CRED;
            let groups = [55u32, 44u32];
            assert_eq!(sys_getgroups(0, core::ptr::null_mut()), 0);
            assert_eq!(sys_setgroups(groups.len() as i32, groups.as_ptr()), 0);
            assert_eq!(sys_getgroups(0, core::ptr::null_mut()), 2);
            let mut out = [0u32; 2];
            assert_eq!(sys_getgroups(out.len() as i32, out.as_mut_ptr()), 2);
            assert_eq!(out, [44, 55]);
            assert!(
                crate::kernel::groups::in_group_p(KGid(44)),
                "vendor set_groups sorts initgroups input for binary lookup"
            );
            assert_eq!(sys_getgroups(1, out.as_mut_ptr()), -(EINVAL as i64));
            assert_eq!(sys_setgroups(-1, core::ptr::null()), -(EINVAL as i64));
            assert_eq!(sys_setgroups(1, core::ptr::null()), -(EFAULT as i64));

            assert_eq!(crate::kernel::session::sys_setpgid(0, 0), 0);
            assert_eq!(sys_getpgid(0), 53);
            assert_eq!(crate::kernel::session::sys_setsid(), 53);
            assert_eq!(sys_getsid(0), 53);

            assert_eq!(sys_personality(u32::MAX), 0);
            assert_eq!(sys_personality(0x08), 0);
            assert_eq!(sys_personality(u32::MAX), 0x08);
            let _ = sys_personality(0);

            assert_eq!(sys_setpriority(0, 0, -40), 0);
            assert_eq!(sys_getpriority(0, 0), 40);
            assert_eq!(sys_setpriority(3, 0, 0), -(EINVAL as i64));
            assert_eq!(sys_getpriority(0, -1), -(ESRCH as i64));
            let _ = sys_setpriority(0, 0, 0);

            current.cred = &raw const INIT_CRED;
            sched::set_current(previous);
        }
    }

    #[test]
    fn unprivileged_credential_syscalls_cannot_escalate() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 55;
        current.tgid = 55;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            let new = crate::kernel::cred::prepare_creds().expect("prepare creds");
            (*new).uid = KUid(1000);
            (*new).euid = KUid(1000);
            (*new).suid = KUid(1000);
            (*new).fsuid = KUid(1000);
            (*new).gid = KGid(1000);
            (*new).egid = KGid(1000);
            (*new).sgid = KGid(1000);
            (*new).fsgid = KGid(1000);
            (*new).cap_permitted = capability::KernelCapT::empty();
            (*new).cap_effective = capability::KernelCapT::empty();
            crate::kernel::cred::commit_creds(new);

            assert!(!capability::capable(CAP_SETUID));
            assert!(!capability::capable(CAP_SETGID));

            assert_eq!(sys_setuid(0), -(EPERM as i64));
            assert_eq!(sys_setresuid(0, 0, 0), -(EPERM as i64));
            assert_eq!(sys_setfsuid(0), 1000);
            assert_eq!(current_cred_ref().uid.0, 1000);
            assert_eq!(current_cred_ref().euid.0, 1000);
            assert_eq!(current_cred_ref().fsuid.0, 1000);

            assert_eq!(sys_setgid(0), -(EPERM as i64));
            assert_eq!(sys_setresgid(0, 0, 0), -(EPERM as i64));
            assert_eq!(sys_setfsgid(0), 1000);
            assert_eq!(current_cred_ref().gid.0, 1000);
            assert_eq!(current_cred_ref().egid.0, 1000);
            assert_eq!(current_cred_ref().fsgid.0, 1000);

            let groups = [0u32];
            assert_eq!(
                sys_setgroups(groups.len() as i32, groups.as_ptr()),
                -(EPERM as i64)
            );
            assert_eq!(current_cred_ref().group_info.ngroups, 0);

            current.cred = &raw const INIT_CRED;
            sched::set_current(previous);
        }
    }

    #[test]
    fn setresuid_keepcaps_preserves_permitted_for_ambient_rebuild() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 54;
        current.tgid = 54;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            let new = crate::kernel::cred::prepare_creds().expect("prepare creds");
            (*new).cap_inheritable.raise(CAP_CHOWN);
            crate::kernel::cred::commit_creds(new);
            assert_eq!(sys_prctl(PR_SET_KEEPCAPS, 1, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_KEEPCAPS, 0, 0, 0, 0), 1);
            assert_eq!(
                sys_prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_CHOWN as u64, 0, 0),
                0
            );
            assert_eq!(
                sys_prctl(
                    PR_CAP_AMBIENT,
                    PR_CAP_AMBIENT_IS_SET,
                    CAP_CHOWN as u64,
                    0,
                    0
                ),
                1
            );

            assert_eq!(sys_setresuid(1000, 1000, 1000), 0);
            let cred = current_cred_ref();
            assert!(cred.cap_permitted.raised(CAP_CHOWN));
            assert!(!cred.cap_effective.raised(CAP_CHOWN));
            assert!(cred.cap_inheritable.raised(CAP_CHOWN));
            assert!(cred.cap_ambient.is_empty());

            let mut header = UserCapHeader {
                version: LINUX_CAPABILITY_VERSION_3,
                pid: 54,
            };
            let mut data = [UserCapData::default(); KERNEL_CAPABILITY_U32S];
            assert_eq!(sys_capget(&mut header, data.as_mut_ptr()), 0);
            for slot in &mut data {
                slot.effective = slot.permitted;
            }
            assert_eq!(sys_capset(&header, data.as_ptr()), 0);
            assert!(current_cred_ref().cap_effective.raised(CAP_CHOWN));
            assert_eq!(
                sys_prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_CHOWN as u64, 0, 0),
                0
            );

            current.cred = &raw const INIT_CRED;
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m76_planned_partial_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 51;
        current.tgid = 51;
        current.cred = &raw const INIT_CRED;

        unsafe {
            crate::kernel::signal::reset_for_tests();
            crate::kernel::session::reset_for_tests();
            sched::set_current(&mut *current as *mut TaskStruct);
            let _ = sys_alarm(0);

            assert_eq!(sys_pause(), -(EINTR as i64));
            assert_eq!(sys_restart_syscall(), -(EINTR as i64));
            assert_eq!(sys_rt_sigsuspend(4), -(EINVAL as i64));
            assert_eq!(sys_rt_sigsuspend(8), -(EINTR as i64));

            assert_eq!(sys_kill(51, 0), 0);
            assert_eq!(sys_kill(0, 0), 0);
            assert_eq!(sys_kill(-51, 0), 0);
            assert_eq!(sys_kill(-9999, 0), -(ESRCH as i64));
            assert_eq!(sys_kill(i32::MIN, 0), -(ESRCH as i64));
            assert_eq!(sys_kill(9999, 0), -(ESRCH as i64));
            assert_eq!(sys_kill(51, 65), -(EINVAL as i64));

            let mut it = ITimerVal::default();
            assert_eq!(sys_getitimer(0, &mut it), 0);
            assert_eq!(sys_getitimer(3, &mut it), -(EINVAL as i64));
            assert_eq!(sys_getitimer(0, core::ptr::null_mut()), -(EFAULT as i64));

            let new_it = ITimerVal {
                it_interval: TimeVal::default(),
                it_value: TimeVal {
                    tv_sec: 1,
                    tv_usec: 999_999,
                },
            };
            let mut old_it = ITimerVal {
                it_interval: TimeVal {
                    tv_sec: -1,
                    tv_usec: -1,
                },
                it_value: TimeVal {
                    tv_sec: -1,
                    tv_usec: -1,
                },
            };
            assert_eq!(sys_setitimer(0, &new_it, &mut old_it), 0);
            assert_eq!(old_it, ITimerVal::default());
            let bad_it = ITimerVal {
                it_interval: TimeVal::default(),
                it_value: TimeVal {
                    tv_sec: 0,
                    tv_usec: 1_000_000,
                },
            };
            assert_eq!(
                sys_setitimer(0, &bad_it, core::ptr::null_mut()),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_setitimer(0, core::ptr::null(), core::ptr::null_mut()),
                -(EFAULT as i64)
            );
            assert_eq!(
                sys_setitimer(0, invalid_user_ptr(), core::ptr::null_mut()),
                -(EFAULT as i64)
            );

            let old_alarm = sys_alarm(7);
            assert_eq!(sys_alarm(0), 7);
            assert!(old_alarm >= 0);
            let _ = sys_alarm(0);

            assert_eq!(sys_setpriority(0, 0, -40), 0);
            assert_eq!(sys_getpriority(0, 0), 40);
            assert_eq!(sys_setpriority(3, 0, 0), -(EINVAL as i64));
            assert_eq!(sys_getpriority(0, -1), -(ESRCH as i64));
            let _ = sys_setpriority(0, 0, 0);

            let domain = b"localdomain";
            assert_eq!(
                crate::kernel::utsname::sys_setdomainname(domain.as_ptr(), domain.len()),
                0
            );
            let uname = build_utsname();
            assert_eq!(&uname.domainname[..domain.len()], domain);
            assert_eq!(
                crate::kernel::utsname::sys_setdomainname(core::ptr::null(), domain.len()),
                -(EFAULT as i64)
            );

            assert_eq!(sys_set_tid_address(core::ptr::null_mut()), 51);
            assert_eq!(sys_readahead(0, 0, 4096), 0);
            assert_eq!(sys_fadvise64(0, 0, 4096, 0), 0);
            assert_eq!(sys_fadvise64(0, 0, 4096, 6), -(EINVAL as i64));

            let mut cpu = u32::MAX;
            let mut node = u32::MAX;
            assert_eq!(sys_getcpu(&mut cpu, &mut node), 0);
            assert_eq!(cpu, 0);
            assert_eq!(node, 0);

            sched::set_current(previous);
        }
    }

    #[test]
    fn kill_negative_pid_targets_process_group_child() {
        let previous = unsafe { sched::get_current() };
        let mut parent = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        parent.pid = 3100;
        parent.tgid = 3100;
        parent.cred = &raw const INIT_CRED;

        unsafe {
            crate::kernel::signal::reset_for_tests();
            crate::kernel::session::reset_for_tests();
            sched::set_current(&mut *parent as *mut TaskStruct);
            assert_eq!(crate::kernel::session::sys_setpgid(0, 0), 0);

            let child = crate::kernel::fork::copy_process(
                &mut *parent as *mut TaskStruct,
                &crate::kernel::fork::KernelCloneArgs::default(),
            )
            .expect("copy_process");
            let child_pid = (*child).pid;
            assert_eq!(
                crate::kernel::session::process_group(child_pid),
                Some(parent.pid)
            );
            assert_eq!(crate::kernel::session::sys_setpgid(child_pid, child_pid), 0);

            assert_eq!(sys_kill(-child_pid, SIGTSTP), 0);
            assert_eq!(
                (*child).__state.load(core::sync::atomic::Ordering::Acquire),
                crate::kernel::task::task_state::__TASK_STOPPED
            );
            assert_eq!((*child).m26.ptrace_stop_signal, SIGTSTP);
            assert!(!has_pending_signal_for_pid(child_pid, SIGTSTP));

            assert_eq!(sys_kill(-child_pid, SIGCONT), 0);
            assert_eq!(
                (*child).__state.load(core::sync::atomic::Ordering::Acquire),
                crate::kernel::task::task_state::TASK_RUNNING
            );
            assert_eq!((*child).m26.ptrace_stop_signal, 0);

            crate::kernel::exit::release_task(child);
            crate::kernel::session::reset_for_tests();
            crate::kernel::signal::reset_for_tests();
            sched::set_current(previous);
        }
    }

    #[test]
    fn kill_broadcast_excludes_init_worker_threads() {
        let previous = unsafe { sched::get_current() };
        let mut init = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        init.pid = 1;
        init.tgid = 1;
        init.cred = &raw const INIT_CRED;
        let mut caller = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        caller.pid = 3200;
        caller.tgid = 3200;
        caller.cred = &raw const INIT_CRED;

        unsafe {
            crate::kernel::signal::reset_for_tests();
            sched::set_current(&mut *init as *mut TaskStruct);
            let worker = crate::kernel::fork::copy_process(
                &mut *init as *mut TaskStruct,
                &crate::kernel::fork::KernelCloneArgs::default(),
            )
            .expect("copy init worker");
            (*worker).tgid = 1;

            sched::set_current(&mut *caller as *mut TaskStruct);
            assert_eq!(sys_kill(-1, 0), -(ESRCH as i64));

            crate::kernel::exit::release_task(worker);
            crate::kernel::signal::reset_for_tests();
            sched::set_current(previous);
        }
    }

    #[test]
    fn set_tid_address_tracks_clear_child_tid_pointer() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 61;
        current.tgid = 61;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut tid_slot = 0i32;
            assert_eq!(sys_set_tid_address(&mut tid_slot), 61);
            assert_eq!(current.m26.clear_child_tid, &mut tid_slot as *mut i32);

            assert_eq!(sys_set_tid_address(core::ptr::null_mut()), 61);
            assert!(current.m26.clear_child_tid.is_null());

            current.cred = &raw const INIT_CRED;
            sched::set_current(previous);
        }
    }

    #[test]
    fn syscall_m76_memory_policy_process_ipc_parity() {
        let mut policy = -1i32;
        let mut node_mask = u64::MAX;

        assert_eq!(
            sys_mbind(0x1000, 0, 0, core::ptr::null(), 0, 0),
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_mbind(0x1000, 4096, 6, core::ptr::null(), 0, 0),
            -(EINVAL as i64)
        );
        assert_eq!(sys_mbind(0x1000, 4096, 0, core::ptr::null(), 0, 0), 0);
        assert_eq!(sys_set_mempolicy(7, core::ptr::null(), 0), -(EINVAL as i64));
        assert_eq!(sys_set_mempolicy(0, core::ptr::null(), 0), 0);
        assert_eq!(
            unsafe { sys_get_mempolicy(&mut policy, &mut node_mask, 0, 0, 0) },
            0
        );
        assert_eq!((policy, node_mask), (0, 0));
        assert_eq!(
            unsafe { sys_get_mempolicy(&mut policy, &mut node_mask, 0, 0, 0x4) },
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_migrate_pages(-1, 0, core::ptr::null(), core::ptr::null()),
            -(ESRCH as i64)
        );
        assert_eq!(
            sys_migrate_pages(0, 0, core::ptr::null(), core::ptr::null()),
            0
        );
        assert_eq!(
            sys_move_pages(
                0,
                1,
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null_mut(),
                0
            ),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_move_pages(
                0,
                0,
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null_mut(),
                0
            ),
            0
        );
        assert_eq!(
            sys_set_mempolicy_home_node(0x1000, 0, 0, 0),
            -(EINVAL as i64)
        );
        assert_eq!(sys_set_mempolicy_home_node(0x1000, 4096, 0, 0), 0);

        let range = CacheStatRange { off: 0, len: 4096 };
        let mut stat = CacheStat::default();
        assert_eq!(unsafe { sys_cachestat(0, &range, &mut stat, 0) }, 0);
        assert_eq!(
            unsafe { sys_cachestat(0, core::ptr::null(), &mut stat, 0) },
            -(EFAULT as i64)
        );
        assert_eq!(
            unsafe { sys_cachestat(0, &range, &mut stat, 1) },
            -(EINVAL as i64)
        );

        assert_eq!(
            sys_semtimedop(0, core::ptr::null_mut(), 0, core::ptr::null()),
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_semtimedop(0, core::ptr::null_mut(), 1, core::ptr::null()),
            -(ENOSYS as i64)
        );
        assert_eq!(
            sys_mq_timedsend(-1, core::ptr::null(), 1, 0, core::ptr::null()),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_mq_timedsend(-1, core::ptr::null(), 0, 0, core::ptr::null()),
            -(EBADF as i64)
        );
        assert_eq!(
            sys_mq_timedreceive(
                -1,
                core::ptr::null_mut(),
                1,
                core::ptr::null_mut(),
                core::ptr::null()
            ),
            -(EFAULT as i64)
        );
        let mut prio = 0u32;
        assert_eq!(
            sys_mq_timedreceive(-1, core::ptr::null_mut(), 0, &mut prio, core::ptr::null()),
            -(EBADF as i64)
        );

        assert_eq!(
            sys_process_vm_readv(-1, core::ptr::null(), 0, core::ptr::null(), 0, 0),
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_process_vm_readv(0, core::ptr::null(), 0, core::ptr::null(), 0, 0),
            0
        );
        assert_eq!(
            sys_process_vm_writev(0, core::ptr::null(), 0, core::ptr::null(), 0, 1),
            -(EINVAL as i64)
        );
        assert_eq!(unsafe { sys_kcmp(-1, 0, 0, 0, 0) }, -(EINVAL as i64));
        assert_eq!(unsafe { sys_kcmp(0, 0, 0, 0, 0) }, 0);
        assert_eq!(sys_rseq(core::ptr::null_mut(), 1, 0, 0), -(ENOSYS as i64));
        assert_eq!(sys_rseq(core::ptr::null_mut(), 32, 0, 0), -(ENOSYS as i64));
        assert_eq!(
            sys_pidfd_send_signal(-1, 65, core::ptr::null(), 0),
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_pidfd_send_signal(-1, 0, core::ptr::null(), 0),
            -(EBADF as i64)
        );
        assert_eq!(
            sys_process_madvise(-1, core::ptr::null(), 0, 0, 1),
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_process_madvise(-1, core::ptr::null(), 0, 0, 0),
            -(EBADF as i64)
        );
        assert_eq!(sys_process_mrelease(-1, 1), -(EINVAL as i64));
        assert_eq!(sys_process_mrelease(-1, 0), -(EBADF as i64));
        assert_eq!(
            sys_listns(-1, 0, core::ptr::null_mut(), core::ptr::null_mut(), 0),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_listns(-1, 0, &mut 0u8, core::ptr::null_mut(), 0),
            -(ENOSYS as i64)
        );
        assert_eq!(sys_uretprobe(), -(ENOSYS as i64));
        assert_eq!(sys_uprobe(), -(ENOSYS as i64));
        assert_eq!(sys_rseq_slice_yield(0, 0, 1), -(EINVAL as i64));
        assert_eq!(sys_rseq_slice_yield(0, 0, 0), 0);
    }

    #[test]
    fn rseq_fails_closed_until_return_to_user_hooks_exist() {
        let _guard = RSEQ_TEST_LOCK.lock();
        RSEQ_REGISTRY.lock().clear();
        let previous = unsafe { sched::get_current() };
        let mut task = zeroed_task(334);
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        let mut area = AlignedRseqArea(RseqUserArea {
            cpu_id_start: RSEQ_CPU_ID_UNINITIALIZED,
            cpu_id: RSEQ_CPU_ID_UNINITIALIZED,
            rseq_cs: 0xfeed,
            flags: 0xffff,
            node_id: 9,
            mm_cid: 7,
            reserved: 0xeeee,
        });
        let ptr = &mut area.0 as *mut RseqUserArea as *mut u8;
        assert_eq!(
            sys_rseq(ptr, RSEQ_ORIG_SIZE, 0, 0x5305_3053),
            -(ENOSYS as i64)
        );
        assert_eq!(
            area.0,
            RseqUserArea {
                cpu_id_start: RSEQ_CPU_ID_UNINITIALIZED,
                cpu_id: RSEQ_CPU_ID_UNINITIALIZED,
                rseq_cs: 0xfeed,
                flags: 0xffff,
                node_id: 9,
                mm_cid: 7,
                reserved: 0xeeee,
            }
        );
        assert!(RSEQ_REGISTRY.lock().get(334).is_none());
        assert_eq!(
            sys_rseq(ptr, RSEQ_ORIG_SIZE, RSEQ_FLAG_UNREGISTER, 0x5305_3053),
            -(ENOSYS as i64)
        );

        RSEQ_REGISTRY.lock().clear();
        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn rseq_unavailable_takes_precedence_over_validation() {
        let _guard = RSEQ_TEST_LOCK.lock();
        RSEQ_REGISTRY.lock().clear();
        let previous = unsafe { sched::get_current() };
        let mut task = zeroed_task(335);
        unsafe { sched::set_current(&mut *task as *mut TaskStruct) };

        let mut area = AlignedRseqArea(RseqUserArea::default());
        let ptr = &mut area.0 as *mut RseqUserArea as *mut u8;
        assert_eq!(
            sys_rseq(unsafe { ptr.add(4) }, RSEQ_ORIG_SIZE, 0, 1),
            -(ENOSYS as i64)
        );
        assert_eq!(sys_rseq(ptr, RSEQ_ORIG_SIZE + 8, 0, 1), -(ENOSYS as i64));
        assert_eq!(
            sys_rseq(ptr, RSEQ_ORIG_SIZE, RSEQ_FLAG_UNREGISTER, 1),
            -(ENOSYS as i64)
        );
        clear_current_rseq_registration_for_exec();
        assert_eq!(sys_rseq(ptr, RSEQ_ORIG_SIZE, 0, 2), -(ENOSYS as i64));

        RSEQ_REGISTRY.lock().clear();
        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn syscall_m78_ipc_mqueue_parity() {
        assert_eq!(sys_shmget(0, 0, 0), -(EINVAL as i64));
        assert_eq!(sys_shmget(0, 4096, 0), -(ENOSYS as i64));
        assert_eq!(sys_shmat(0, 0, 0x4000), -(EINVAL as i64));
        assert_eq!(sys_shmat(0, 0, 0), -(ENOSYS as i64));
        assert_eq!(sys_shmctl(0, -1, core::ptr::null_mut()), -(EINVAL as i64));
        assert_eq!(sys_shmctl(0, 0, core::ptr::null_mut()), -(ENOSYS as i64));
        assert_eq!(sys_shmdt(0), -(EINVAL as i64));
        assert_eq!(sys_shmdt(0x1000), -(ENOSYS as i64));

        assert_eq!(sys_semget(0, -1, 0), -(EINVAL as i64));
        assert_eq!(sys_semget(0, 1, 0), -(ENOSYS as i64));
        assert_eq!(sys_semop(0, core::ptr::null_mut(), 0), -(EINVAL as i64));
        assert_eq!(sys_semop(0, core::ptr::null_mut(), 1), -(ENOSYS as i64));
        assert_eq!(sys_semctl(0, 0, -1, 0), -(EINVAL as i64));
        assert_eq!(sys_semctl(0, 0, 0, 0), -(ENOSYS as i64));

        assert_eq!(sys_msgget(0, 0), -(ENOSYS as i64));
        assert_eq!(sys_msgsnd(0, core::ptr::null(), 1, 0), -(EFAULT as i64));
        assert_eq!(sys_msgsnd(0, core::ptr::null(), 0, 0), -(ENOSYS as i64));
        assert_eq!(
            sys_msgrcv(0, core::ptr::null_mut(), 1, 0, 0),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_msgrcv(0, core::ptr::null_mut(), 0, 0, 0),
            -(ENOSYS as i64)
        );
        assert_eq!(sys_msgctl(0, -1, core::ptr::null_mut()), -(EINVAL as i64));
        assert_eq!(sys_msgctl(0, 0, core::ptr::null_mut()), -(ENOSYS as i64));

        assert_eq!(
            sys_mq_open(core::ptr::null(), 0, 0, core::ptr::null()),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_mq_open(b"/mq\0".as_ptr(), 0, 0, core::ptr::null()),
            -(ENOSYS as i64)
        );
        assert_eq!(sys_mq_unlink(core::ptr::null()), -(EFAULT as i64));
        assert_eq!(sys_mq_unlink(b"/mq\0".as_ptr()), -(ENOSYS as i64));
        assert_eq!(sys_mq_notify(-1, core::ptr::null()), -(EBADF as i64));
        assert_eq!(
            sys_mq_getsetattr(-1, core::ptr::null(), core::ptr::null_mut()),
            -(EBADF as i64)
        );
    }

    #[test]
    fn syscall_m78_futex_pidfd_parity() {
        assert_eq!(sys_pidfd_open(0, 0), -(EINVAL as i64));
        assert_eq!(sys_pidfd_open(1, 1), -(EINVAL as i64));
        assert_eq!(sys_pidfd_open(9999, 0), -(ESRCH as i64));
        assert_eq!(sys_pidfd_getfd(-1, -1, 1), -(EINVAL as i64));
        assert_eq!(sys_pidfd_getfd(-1, -1, 0), -(EBADF as i64));
    }

    #[test]
    fn pidfd_send_signal_validates_and_targets_pidfd() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 64;
        current.tgid = 64;
        current.cred = &raw const INIT_CRED;

        let mut target = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        target.pid = 65;
        target.tgid = 65;
        target.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(target.pid)).expect("pid alloc");
        target.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            crate::kernel::files::set_task_files(
                &mut *current as *mut TaskStruct,
                FilesStruct::new(),
            );
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                sys_pidfd_send_signal(-1, 65, core::ptr::null(), 0),
                -(EINVAL as i64)
            );
            assert_eq!(
                sys_pidfd_send_signal(-1, 0, core::ptr::null(), 0),
                -(EBADF as i64)
            );

            let fd = crate::fs::pidfd::install_pidfd(&mut *target as *mut TaskStruct, false)
                .expect("pidfd");
            assert_eq!(sys_pidfd_send_signal(fd, 0, core::ptr::null(), 0), 0);

            let wrong_signo = crate::kernel::signal::SigInfo::with_sender_value(
                10,
                crate::kernel::signal::SI_QUEUE,
                current.tgid,
                0,
                0,
            );
            assert_eq!(
                sys_pidfd_send_signal(fd, 18, (&raw const wrong_signo).cast::<u8>(), 0),
                -(EINVAL as i64)
            );
            let forbidden_code = crate::kernel::signal::SigInfo::with_sender(18, 0, 64, 0);
            assert_eq!(
                sys_pidfd_send_signal(fd, 18, (&raw const forbidden_code).cast::<u8>(), 0),
                -(EPERM as i64)
            );
            let queued = crate::kernel::signal::SigInfo::with_sender_value(
                18,
                crate::kernel::signal::SI_QUEUE,
                current.tgid,
                0,
                0x55aa,
            );
            assert_eq!(
                sys_pidfd_send_signal(fd, 18, (&raw const queued).cast::<u8>(), 0),
                0
            );
            assert_eq!(
                crate::kernel::signal::pending_signal_scopes_for_pid(target.pid, 18),
                (false, true)
            );
            assert_eq!(sys_pidfd_send_signal(fd, 18, core::ptr::null(), 0), 0);

            crate::fs::pidfd::notify_task_exit(&mut *target as *mut TaskStruct);
            assert_eq!(
                sys_pidfd_send_signal(fd, 0, core::ptr::null(), 0),
                -(ESRCH as i64)
            );
            assert_eq!(
                sys_pidfd_send_signal(fd, 18, core::ptr::null(), 0),
                -(ESRCH as i64)
            );

            let files = crate::kernel::files::get_task_files(&mut *current as *mut TaskStruct)
                .expect("files");
            files.close(fd).expect("close pidfd");
            crate::kernel::files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            target.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn process_mrelease_requires_sigkill_pending_pidfd_task() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 164;
        current.tgid = 164;
        current.cred = &raw const INIT_CRED;

        let mut mm = Box::new(crate::mm::mm_types::MmStruct::new(0));
        let mut target = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        target.pid = 165;
        target.tgid = 165;
        target.mm = &mut *mm as *mut crate::mm::mm_types::MmStruct;
        target.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(target.pid)).expect("pid alloc");
        target.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            crate::kernel::files::set_task_files(
                &mut *current as *mut TaskStruct,
                FilesStruct::new(),
            );
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = crate::fs::pidfd::install_pidfd(&mut *target as *mut TaskStruct, false)
                .expect("pidfd");
            assert_eq!(sys_process_mrelease(fd, 1), -(EINVAL as i64));
            assert_eq!(sys_process_mrelease(fd, 0), -(EINVAL as i64));
            assert_eq!(sys_pidfd_send_signal(fd, SIGKILL, core::ptr::null(), 0), 0);
            assert_eq!(sys_process_mrelease(fd, 0), 0);

            let files = crate::kernel::files::get_task_files(&mut *current as *mut TaskStruct)
                .expect("files");
            files.close(fd).expect("close pidfd");
            crate::kernel::files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            target.m26.thread_pid = core::ptr::null_mut();
            target.mm = core::ptr::null_mut();
        }
    }

    #[test]
    fn syscall_m78_aio_io_uring_parity() {
        let mut ctx = 0u64;
        assert_eq!(sys_io_setup(0, &mut ctx), -(EINVAL as i64));
        assert_eq!(sys_io_setup(1, core::ptr::null_mut()), -(EFAULT as i64));
        assert_eq!(sys_io_setup(1, &mut ctx), -(ENOSYS as i64));
        assert_eq!(sys_io_destroy(0), -(EINVAL as i64));
        assert_eq!(
            sys_io_getevents(0, -1, 0, core::ptr::null_mut(), core::ptr::null()),
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_io_getevents(0, 0, 0, core::ptr::null_mut(), core::ptr::null()),
            0
        );
        assert_eq!(
            sys_io_submit(0, -1, core::ptr::null_mut()),
            -(EINVAL as i64)
        );
        assert_eq!(sys_io_submit(0, 1, core::ptr::null_mut()), -(EFAULT as i64));
        assert_eq!(
            sys_io_cancel(0, core::ptr::null_mut(), core::ptr::null_mut()),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_io_pgetevents(
                0,
                0,
                0,
                core::ptr::null_mut(),
                core::ptr::null(),
                core::ptr::null()
            ),
            0
        );
    }

    #[test]
    fn syscall_m78_security_bpf_perf_parity() {
        assert_eq!(sys_syslog(-1, core::ptr::null_mut(), 0), -(EINVAL as i64));
        assert_eq!(sys_syslog(0, core::ptr::null_mut(), 1), -(EFAULT as i64));
        assert_eq!(sys_syslog(0, core::ptr::null_mut(), 0), 0);
        assert_eq!(sys_acct(core::ptr::null()), 0);
        assert_eq!(sys_acct(b"/acct\0".as_ptr()), -(EPERM as i64));
        assert_eq!(
            sys_kexec_load(0, 17, core::ptr::null(), 0),
            -(EINVAL as i64)
        );
        assert_eq!(sys_kexec_load(0, 0, core::ptr::null(), 0), -(EPERM as i64));
        assert_eq!(
            sys_kexec_file_load(-1, -1, 0, core::ptr::null(), 8),
            -(EINVAL as i64)
        );
        assert_eq!(
            sys_kexec_file_load(-1, -1, 0, core::ptr::null(), 0),
            -(EPERM as i64)
        );
        assert_eq!(
            sys_lsm_get_self_attr(0, core::ptr::null_mut(), core::ptr::null_mut(), 0),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_lsm_set_self_attr(0, core::ptr::null(), 1, 0),
            -(EFAULT as i64)
        );
        assert_eq!(
            sys_lsm_list_modules(core::ptr::null_mut(), core::ptr::null_mut(), 0),
            -(EFAULT as i64)
        );
    }

    #[test]
    fn lsm_list_modules_returns_ids_and_e2big_size_probe() {
        use crate::security::hooks::{LSM_ID_APPARMOR, LSM_ID_CAPABILITY, LSM_ID_EVM, LSM_ID_IMA};
        use crate::security::lsm_list::{TEST_LSM_LOCK, reset_for_test};

        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        crate::security::apparmor::reset_for_test();
        crate::security::integrity::ima::reset_for_test();
        crate::security::integrity::evm::reset_for_test();
        crate::security::init();

        let mut size = 0u32;
        let mut ids = [0u64; 4];
        assert_eq!(
            sys_lsm_list_modules(ids.as_mut_ptr(), &mut size, 0),
            -(E2BIG as i64)
        );
        assert_eq!(size, (4 * core::mem::size_of::<u64>()) as u32);
        assert_eq!(sys_lsm_list_modules(ids.as_mut_ptr(), &mut size, 0), 4);
        assert_eq!(ids[0], LSM_ID_CAPABILITY);
        assert_eq!(ids[1], LSM_ID_APPARMOR);
        assert_eq!(ids[2], LSM_ID_IMA);
        assert_eq!(ids[3], LSM_ID_EVM);
    }

    #[test]
    fn lsm_list_modules_rejects_non_user_pointers() {
        use crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX;
        use crate::security::lsm_list::{TEST_LSM_LOCK, reset_for_test};

        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        crate::security::init();

        let kernel_size = TASK_SIZE_MAX as *mut u32;
        assert_eq!(
            sys_lsm_list_modules(core::ptr::null_mut(), kernel_size, 0),
            -(EFAULT as i64)
        );

        let mut size = (core::mem::size_of::<u64>() * 16) as u32;
        let kernel_ids = TASK_SIZE_MAX as *mut u64;
        assert_eq!(
            sys_lsm_list_modules(kernel_ids, &mut size, 0),
            -(EFAULT as i64)
        );
    }

    fn swap_header(pages: u32) -> alloc::vec::Vec<u8> {
        let mut header = alloc::vec![0u8; crate::mm::frame::PAGE_SIZE];
        header[SWAP_HEADER_BOOTBITS..SWAP_HEADER_BOOTBITS + 4].copy_from_slice(&1u32.to_le_bytes());
        header[SWAP_HEADER_BOOTBITS + 4..SWAP_HEADER_BOOTBITS + 8]
            .copy_from_slice(&(pages - 1).to_le_bytes());
        header[SWAP_HEADER_MAGIC_OFFSET..].copy_from_slice(b"SWAPSPACE2");
        header
    }

    #[test]
    fn swapon_swapoff_syscalls_activate_swapfile_and_sysinfo() {
        use crate::fs::dcache::d_alloc_child;
        use crate::fs::mount::{self, Mount, set_rootfs};
        use crate::fs::super_block::mount_fs;
        use crate::fs::types::InodePrivate;

        let _mount_guard = mount::TEST_MOUNT_LOCK.lock();
        crate::mm::swap::reset_swap_state_for_test();
        crate::fs::init();
        *mount::MOUNTS.root.lock() = None;
        mount::MOUNTS.by_path.lock().clear();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        set_rootfs(Mount::alloc(sb, root.clone(), 0));

        let root_inode = root.inode().expect("root inode");
        let create = root_inode.ops.create.expect("ramfs create");
        let inode = create(&root_inode, "swapfile", 0o600).expect("create swapfile");
        let pages = 2048u32;
        inode.size.store(
            pages as u64 * crate::mm::frame::PAGE_SIZE as u64,
            Ordering::Release,
        );
        match &inode.private {
            InodePrivate::RamBytes(bytes) => {
                bytes.lock().extend_from_slice(&swap_header(pages));
            }
            _ => panic!("swapfile should be ram bytes"),
        }
        let dentry = d_alloc_child(&root, "swapfile");
        dentry.instantiate(inode);

        let path = b"/swapfile\0";
        assert_eq!(sys_swapon(path.as_ptr(), (SWAP_FLAG_PREFER | 7) as i32), 0);
        assert_eq!(
            sys_swapon(path.as_ptr(), 0),
            -(crate::include::uapi::errno::EBUSY as i64)
        );
        assert!(crate::mm::swap::proc_swaps().contains("/swapfile"));

        let mut info = SysInfo::default();
        assert_eq!(unsafe { sys_sysinfo(&mut info) }, 0);
        assert_eq!(
            info.totalswap,
            pages as u64 * crate::mm::frame::PAGE_SIZE as u64
        );
        assert_eq!(info.totalswap, info.freeswap);

        assert_eq!(sys_swapoff(path.as_ptr()), 0);
        assert_eq!(crate::mm::swap::total_swap_pages(), 0);
    }

    #[test]
    fn swapon_syscall_accepts_linux_block_device_swap_header() {
        use crate::block::block_device::{BlockDevice, register_block_device};
        use crate::block::mem::{MemBlockDevice, mem_block_device_ops};
        use crate::fs::mount::{self, Mount, set_rootfs};
        use crate::fs::super_block::mount_fs;
        use crate::include::uapi::fcntl::AT_FDCWD;
        use crate::include::uapi::stat::S_IFBLK;

        let _mount_guard = mount::TEST_MOUNT_LOCK.lock();
        crate::mm::swap::reset_swap_state_for_test();
        crate::fs::init();
        *mount::MOUNTS.root.lock() = None;
        mount::MOUNTS.by_path.lock().clear();

        let sb = mount_fs("ramfs", "", 0, "").expect("ramfs mount");
        let root = sb.root().expect("root dentry");
        set_rootfs(Mount::alloc(sb, root, 0));

        assert_eq!(
            unsafe { crate::fs::syscalls::sys_mkdir(b"/dev\0".as_ptr(), 0o755) },
            0
        );
        assert_eq!(
            unsafe { crate::fs::syscalls::sys_mkdir(b"/dev/mapper\0".as_ptr(), 0o755) },
            0
        );

        let pages = 256u32;
        let mem = MemBlockDevice::new(
            "mapper/cl-swap-syscall37",
            pages as usize * crate::mm::frame::PAGE_SIZE,
        );
        mem.data.lock()[..crate::mm::frame::PAGE_SIZE].copy_from_slice(&swap_header(pages));
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        register_block_device("mapper/cl-swap-syscall37", bdev)
            .expect("register mapped swap block device");

        let path = b"/dev/mapper/cl-swap-syscall37\0";
        assert_eq!(
            unsafe {
                crate::fs::syscalls::sys_mknodat(AT_FDCWD, path.as_ptr(), S_IFBLK | 0o600, 0)
            },
            0
        );

        assert_eq!(sys_swapon(path.as_ptr(), 0), 0);
        let swaps = crate::mm::swap::proc_swaps();
        assert!(swaps.contains("/dev/mapper/cl-swap-syscall37"));
        assert!(swaps.contains("\tpartition\t\t1024\t\t0\t\t-1\n"));

        let mut info = SysInfo::default();
        assert_eq!(unsafe { sys_sysinfo(&mut info) }, 0);
        assert_eq!(
            info.totalswap,
            pages as u64 * crate::mm::frame::PAGE_SIZE as u64
        );
        assert_eq!(info.totalswap, info.freeswap);

        assert_eq!(sys_swapoff(path.as_ptr()), 0);
        assert_eq!(crate::mm::swap::total_swap_pages(), 0);
    }
}
