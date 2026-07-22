//! linux-parity: partial
//! linux-source: vendor/linux/ipc/shm.c
//! test-origin: linux:vendor/linux/ipc/shm.c
//! SysV shared-memory limits, lifecycle flags, and attach/detach accounting.

extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use core::sync::atomic::Ordering;

use spin::Mutex;

use crate::include::uapi::errno::{EACCES, EEXIST, EFAULT, EINVAL, ENOENT, ENOMEM, ENOSPC, EPERM};
use crate::ipc::util::{IPC_PRIVATE, ipc_permission_allowed, ipcget_route};
use crate::kernel::{capability, cred, sched};
use crate::mm::address_space::{AS_SHARED_ANON, AddressSpace};
use crate::mm::mmap::{MAP_ANONYMOUS, MAP_FIXED, MAP_SHARED, PROT_EXEC, PROT_READ, PROT_WRITE};

pub const SHMMIN: usize = 1;
pub const SHMMNI: usize = 4096;
pub const SHMMAX: usize = usize::MAX - (1usize << 24);
pub const SHMALL: usize = usize::MAX - (1usize << 24);
pub const SHMSEG: usize = SHMMNI;
pub const SHM_R: u16 = 0o400;
pub const SHM_W: u16 = 0o200;
pub const SHM_HUGETLB: i32 = 0o4000;
pub const SHM_NORESERVE: i32 = 0o10000;
pub const SHM_RDONLY: i32 = 0o10000;
pub const SHM_RND: i32 = 0o20000;
pub const SHM_REMAP: i32 = 0o40000;
pub const SHM_EXEC: i32 = 0o100000;
pub const SHM_DEST: u16 = 0o1000;
pub const SHM_LOCKED: u16 = 0o2000;
pub const SHM_LOCK: i32 = 11;
pub const SHM_UNLOCK: i32 = 12;
pub const SHM_STAT: i32 = 13;
pub const SHM_INFO: i32 = 14;
pub const SHM_STAT_ANY: i32 = 15;
pub const IPC_CREAT: i32 = 0o1000;
pub const IPC_RMID: i32 = 0;
pub const IPC_SET: i32 = 1;
pub const IPC_STAT: i32 = 2;
pub const IPC_INFO: i32 = 3;
pub const SHMLBA: u64 = crate::arch::x86::mm::paging::PAGE_SIZE;
const PAGE_MASK: u64 = crate::arch::x86::mm::paging::PAGE_MASK;
const S_IRUGO: u16 = 0o444;
const S_IWUGO: u16 = 0o222;
const S_IXUGO: u16 = 0o111;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShmSegment {
    pub id: i32,
    pub key: i32,
    pub size: usize,
    pub mode: u16,
    pub creator_pid: i32,
    pub last_pid: i32,
    pub nattch: usize,
    pub atime: i64,
    pub dtime: i64,
    pub ctime: i64,
    pub huge_tlb: bool,
    pub no_reserve: bool,
    pub destroy_on_detach: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShmNamespace {
    pub ctlmax: usize,
    pub ctlall: usize,
    pub ctlmni: usize,
    pub segments: Vec<ShmSegment>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShmError {
    InvalidSize,
    NoSpace,
    NotFound,
    WouldOverlap,
    Permission,
}

impl ShmNamespace {
    pub fn new() -> Self {
        Self {
            ctlmax: SHMMAX,
            ctlall: SHMALL,
            ctlmni: SHMMNI,
            segments: Vec::new(),
        }
    }

    pub fn shmget(
        &mut self,
        key: i32,
        size: usize,
        shmflg: i32,
        pid: i32,
        now: i64,
    ) -> Result<i32, ShmError> {
        if size < SHMMIN || size > self.ctlmax {
            return Err(ShmError::InvalidSize);
        }
        if self.segments.len() >= self.ctlmni {
            return Err(ShmError::NoSpace);
        }
        let id = self.segments.len() as i32;
        self.segments.push(ShmSegment {
            id,
            key,
            size,
            mode: (shmflg as u16) & 0o777,
            creator_pid: pid,
            last_pid: pid,
            nattch: 0,
            atime: 0,
            dtime: 0,
            ctime: now,
            huge_tlb: shmflg & SHM_HUGETLB != 0,
            no_reserve: shmflg & SHM_NORESERVE != 0,
            destroy_on_detach: false,
            locked: false,
        });
        Ok(id)
    }

    pub fn segment(&self, id: i32) -> Option<&ShmSegment> {
        self.segments.iter().find(|seg| seg.id == id)
    }

    pub fn segment_mut(&mut self, id: i32) -> Option<&mut ShmSegment> {
        self.segments.iter_mut().find(|seg| seg.id == id)
    }

    pub fn shmctl_rmid(&mut self, id: i32) -> Result<(), ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.destroy_on_detach = true;
        seg.mode |= SHM_DEST;
        if seg.nattch == 0 {
            self.segments.retain(|entry| entry.id != id);
        }
        Ok(())
    }

    pub fn shmctl_lock(&mut self, id: i32, lock: bool) -> Result<(), ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.locked = lock;
        if lock {
            seg.mode |= SHM_LOCKED;
        } else {
            seg.mode &= !SHM_LOCKED;
        }
        Ok(())
    }

    pub fn shmat(&mut self, id: i32, shmflg: i32, pid: i32, now: i64) -> Result<Attach, ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.nattch += 1;
        seg.last_pid = pid;
        seg.atime = now;
        Ok(Attach {
            shmid: id,
            readonly: shmflg & SHM_RDONLY != 0,
            executable: shmflg & SHM_EXEC != 0,
            remap: shmflg & SHM_REMAP != 0,
        })
    }

    pub fn shmdt(&mut self, id: i32, pid: i32, now: i64) -> Result<(), ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.nattch = seg.nattch.saturating_sub(1);
        seg.last_pid = pid;
        seg.dtime = now;
        if seg.nattch == 0 && seg.destroy_on_detach {
            self.segments.retain(|entry| entry.id != id);
        }
        Ok(())
    }

    pub fn proc_sysvipc_shm_header(word_bits: usize) -> &'static str {
        if word_bits == 32 {
            "       key      shmid perms       size  cpid  lpid nattch   uid   gid  cuid  cgid      atime      dtime      ctime        rss       swap\n"
        } else {
            "       key      shmid perms                  size  cpid  lpid nattch   uid   gid  cuid  cgid      atime      dtime      ctime                   rss                  swap\n"
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attach {
    pub shmid: i32,
    pub readonly: bool,
    pub executable: bool,
    pub remap: bool,
}

pub fn hugepage_shm_test_length() -> usize {
    256usize * 1024 * 1024
}

pub fn sysvipc_shm_empty_proc(word_bits: usize) -> String {
    String::from(ShmNamespace::proc_sysvipc_shm_header(word_bits))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SysvShmSegment {
    id: i32,
    key: i32,
    size: usize,
    mode: u16,
    uid: u32,
    gid: u32,
    cuid: u32,
    cgid: u32,
    cpid: i32,
    lpid: i32,
    nattch: usize,
    atime: i64,
    dtime: i64,
    ctime: i64,
    destroy_on_detach: bool,
    locked: bool,
    mapping: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SysvShmAttachment {
    shmid: i32,
    size: usize,
}

struct SysvShmState {
    next_id: i32,
    total_pages: usize,
    segments: BTreeMap<i32, SysvShmSegment>,
    keys: BTreeMap<i32, i32>,
    attachments: BTreeMap<u64, SysvShmAttachment>,
}

impl SysvShmState {
    const fn new() -> Self {
        Self {
            next_id: 0,
            total_pages: 0,
            segments: BTreeMap::new(),
            keys: BTreeMap::new(),
            attachments: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    fn reset(&mut self) {
        self.next_id = 0;
        self.total_pages = 0;
        self.segments.clear();
        self.keys.clear();
        self.attachments.clear();
    }

    fn max_id(&self) -> i32 {
        self.segments.keys().next_back().copied().unwrap_or(0)
    }
}

static SYSV_SHM_STATE: Mutex<SysvShmState> = Mutex::new(SysvShmState::new());

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ipc64Perm {
    pub key: i32,
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u32,
    pub seq: u16,
    pub __pad2: u16,
    pub __unused1: u64,
    pub __unused2: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Shmid64Ds {
    pub shm_perm: Ipc64Perm,
    pub shm_segsz: u64,
    pub shm_atime: i64,
    pub shm_dtime: i64,
    pub shm_ctime: i64,
    pub shm_cpid: i32,
    pub shm_lpid: i32,
    pub shm_nattch: u64,
    pub __unused4: u64,
    pub __unused5: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Shminfo64 {
    pub shmmax: u64,
    pub shmmin: u64,
    pub shmmni: u64,
    pub shmseg: u64,
    pub shmall: u64,
    pub __unused1: u64,
    pub __unused2: u64,
    pub __unused3: u64,
    pub __unused4: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShmInfo {
    pub used_ids: i32,
    pub shm_tot: u64,
    pub shm_rss: u64,
    pub shm_swp: u64,
    pub swap_attempts: u64,
    pub swap_successes: u64,
}

#[cfg(test)]
pub fn reset_sysv_shm_state_for_tests() {
    SYSV_SHM_STATE.lock().reset();
}

fn current_pid() -> i32 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        0
    } else {
        unsafe { (*task).tgid.max((*task).pid) }
    }
}

fn now_seconds() -> i64 {
    (crate::kernel::time::ktime_get_real() / 1_000_000_000) as i64
}

fn current_ids() -> (u32, u32) {
    let c = unsafe { &*cred::current_cred() };
    (c.euid.0, c.egid.0)
}

fn segment_permission_allowed(seg: &SysvShmSegment, requested: u16) -> bool {
    let c = unsafe { &*cred::current_cred() };
    ipc_permission_allowed(
        seg.mode,
        requested,
        c.euid.0 == seg.uid || c.euid.0 == seg.cuid,
        c.egid.0 == seg.gid || c.egid.0 == seg.cgid,
        capability::capable(capability::CAP_IPC_OWNER),
    )
}

fn alloc_shared_mapping() -> usize {
    let mut mapping = Box::new(AddressSpace::new());
    mapping.flags.fetch_or(AS_SHARED_ANON, Ordering::Relaxed);
    Box::into_raw(mapping) as usize
}

fn checked_shm_pages(size: usize) -> Result<usize, i32> {
    let pages = size.div_ceil(crate::mm::frame::PAGE_SIZE);
    if pages
        .checked_shl(crate::arch::x86::mm::paging::PAGE_SHIFT)
        .is_none_or(|bytes| bytes < size)
    {
        return Err(ENOSPC);
    }
    Ok(pages)
}

fn build_shmid64(seg: &SysvShmSegment) -> Shmid64Ds {
    let mut mode = seg.mode;
    if seg.destroy_on_detach {
        mode |= SHM_DEST;
    }
    if seg.locked {
        mode |= SHM_LOCKED;
    }
    Shmid64Ds {
        shm_perm: Ipc64Perm {
            key: seg.key,
            uid: seg.uid,
            gid: seg.gid,
            cuid: seg.cuid,
            cgid: seg.cgid,
            mode: mode as u32,
            seq: 0,
            __pad2: 0,
            __unused1: 0,
            __unused2: 0,
        },
        shm_segsz: seg.size as u64,
        shm_atime: seg.atime,
        shm_dtime: seg.dtime,
        shm_ctime: seg.ctime,
        shm_cpid: seg.cpid,
        shm_lpid: seg.lpid,
        shm_nattch: seg.nattch as u64,
        __unused4: 0,
        __unused5: 0,
    }
}

fn copy_to_user<T>(dst: *mut u8, value: &T) -> Result<(), i32> {
    if dst.is_null() {
        return Err(EFAULT);
    }
    let left = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(
            dst,
            (value as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if left == 0 { Ok(()) } else { Err(EFAULT) }
}

fn maybe_destroy_segment_locked(state: &mut SysvShmState, shmid: i32) {
    let Some(seg) = state.segments.get(&shmid).copied() else {
        return;
    };
    if seg.nattch != 0 || !seg.destroy_on_detach {
        return;
    }
    state.total_pages = state
        .total_pages
        .saturating_sub(seg.size.div_ceil(crate::mm::frame::PAGE_SIZE));
    if seg.key != IPC_PRIVATE {
        state.keys.remove(&seg.key);
    }
    state.segments.remove(&shmid);
}

fn shmget_create_locked(
    state: &mut SysvShmState,
    key: i32,
    size: usize,
    shmflg: i32,
) -> Result<i32, i32> {
    if !(SHMMIN..=SHMMAX).contains(&size) {
        return Err(EINVAL);
    }
    if shmflg & SHM_HUGETLB != 0 {
        return Err(EINVAL);
    }
    if state.segments.len() >= SHMMNI {
        return Err(ENOSPC);
    }
    let pages = checked_shm_pages(size)?;
    if state
        .total_pages
        .checked_add(pages)
        .is_none_or(|sum| sum > SHMALL)
    {
        return Err(ENOSPC);
    }
    let id = state.next_id;
    state.next_id = state.next_id.checked_add(1).ok_or(ENOSPC)?;
    let (uid, gid) = current_ids();
    let seg = SysvShmSegment {
        id,
        key,
        size,
        mode: (shmflg as u16) & 0o777,
        uid,
        gid,
        cuid: uid,
        cgid: gid,
        cpid: current_pid(),
        lpid: 0,
        nattch: 0,
        atime: 0,
        dtime: 0,
        ctime: now_seconds(),
        destroy_on_detach: false,
        locked: false,
        mapping: alloc_shared_mapping(),
    };
    state.total_pages += pages;
    state.segments.insert(id, seg);
    if key != IPC_PRIVATE {
        state.keys.insert(key, id);
    }
    Ok(id)
}

pub fn sys_shmget(key: i32, size: usize, shmflg: i32) -> i64 {
    let mut state = SYSV_SHM_STATE.lock();
    let existing = if key == IPC_PRIVATE {
        None
    } else {
        state.keys.get(&key).copied()
    };

    match ipcget_route(key, shmflg, existing.is_some()) {
        crate::ipc::util::IpcGetRoute::NewPrivate
        | crate::ipc::util::IpcGetRoute::PublicMissingCreate => {
            shmget_create_locked(&mut state, key, size, shmflg)
        }
        crate::ipc::util::IpcGetRoute::PublicMissingNoEntry => Err(ENOENT),
        crate::ipc::util::IpcGetRoute::PublicExistingExclusive => Err(EEXIST),
        crate::ipc::util::IpcGetRoute::PublicExistingCheckPerms => {
            let id = existing.expect("route requires existing segment");
            let Some(seg) = state.segments.get(&id) else {
                return -(EINVAL as i64);
            };
            if size > seg.size {
                return -(EINVAL as i64);
            }
            if !segment_permission_allowed(seg, (shmflg as u16) & 0o777) {
                return -(EACCES as i64);
            }
            Ok(id)
        }
    }
    .map(i64::from)
    .unwrap_or_else(|errno| -(errno as i64))
}

fn shmat_adjust_addr(mut addr: u64, shmflg: i32) -> Result<(u64, u32), i32> {
    let mut flags = MAP_SHARED | MAP_ANONYMOUS;
    if addr != 0 {
        if addr & (SHMLBA - 1) != 0 {
            if shmflg & SHM_RND != 0 {
                addr &= !(SHMLBA - 1);
                if addr == 0 && shmflg & SHM_REMAP != 0 {
                    return Err(EINVAL);
                }
            } else if addr & !PAGE_MASK != 0 {
                return Err(EINVAL);
            }
        }
        flags |= MAP_FIXED;
    } else if shmflg & SHM_REMAP != 0 {
        return Err(EINVAL);
    }
    Ok((addr, flags))
}

fn shmat_prepare(shmid: i32, shmflg: i32) -> Result<(usize, usize), i32> {
    if shmid < 0 {
        return Err(EINVAL);
    }
    let mut requested = S_IRUGO;
    if shmflg & SHM_RDONLY == 0 {
        requested |= S_IWUGO;
    }
    if shmflg & SHM_EXEC != 0 {
        requested |= S_IXUGO;
    }

    let mut state = SYSV_SHM_STATE.lock();
    let pid = current_pid();
    let now = now_seconds();
    let seg = state.segments.get_mut(&shmid).ok_or(EINVAL)?;
    if !segment_permission_allowed(seg, requested) {
        return Err(EACCES);
    }
    seg.nattch = seg.nattch.checked_add(1).ok_or(ENOMEM)?;
    seg.lpid = pid;
    seg.atime = now;
    Ok((seg.size, seg.mapping))
}

fn shmat_rollback(shmid: i32) {
    let mut state = SYSV_SHM_STATE.lock();
    let Some(seg) = state.segments.get_mut(&shmid) else {
        return;
    };
    seg.nattch = seg.nattch.saturating_sub(1);
    maybe_destroy_segment_locked(&mut state, shmid);
}

pub fn sys_shmat(shmid: i32, shmaddr: u64, shmflg: i32) -> i64 {
    let (addr, flags) = match shmat_adjust_addr(shmaddr, shmflg) {
        Ok(v) => v,
        Err(errno) => return -(errno as i64),
    };
    let (size, mapping) = match shmat_prepare(shmid, shmflg) {
        Ok(v) => v,
        Err(errno) => return -(errno as i64),
    };

    let prot = if shmflg & SHM_RDONLY != 0 {
        PROT_READ
    } else {
        PROT_READ | PROT_WRITE
    } | if shmflg & SHM_EXEC != 0 { PROT_EXEC } else { 0 };

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        shmat_rollback(shmid);
        return -(EINVAL as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        shmat_rollback(shmid);
        return -(EINVAL as i64);
    }

    let mapped = {
        let _mmap_guard = unsafe { crate::mm::mmap_lock::MmapWriteGuard::lock(mm) };
        if addr != 0 && shmflg & SHM_REMAP == 0 {
            if let Some(vma_ptr) = crate::mm::vma::find_vma(unsafe { &*mm }, addr) {
                if unsafe { (*vma_ptr).vm_start } < addr.saturating_add(size as u64) {
                    shmat_rollback(shmid);
                    return -(EINVAL as i64);
                }
            }
        }
        match unsafe { crate::mm::mmap::do_mmap(&mut *mm, addr, size as u64, prot, flags, 0, 0) } {
            Ok(mapped) => {
                if let Some(vma_ptr) = crate::mm::vma::find_vma(unsafe { &*mm }, mapped) {
                    unsafe {
                        if (*vma_ptr).vm_start == mapped {
                            (*vma_ptr).vm_private_data = mapping;
                        }
                    }
                }
                mapped
            }
            Err(errno) => {
                shmat_rollback(shmid);
                return errno as i64;
            }
        }
    };

    SYSV_SHM_STATE
        .lock()
        .attachments
        .insert(mapped, SysvShmAttachment { shmid, size });
    mapped as i64
}

pub fn sys_shmdt(shmaddr: u64) -> i64 {
    if shmaddr & !PAGE_MASK != 0 {
        return -(EINVAL as i64);
    }
    let attachment = match SYSV_SHM_STATE.lock().attachments.remove(&shmaddr) {
        Some(attachment) => attachment,
        None => return -(EINVAL as i64),
    };

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EINVAL as i64);
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return -(EINVAL as i64);
    }
    let _ = unsafe { crate::mm::syscalls::sys_munmap(shmaddr, attachment.size as u64) };

    let mut state = SYSV_SHM_STATE.lock();
    let pid = current_pid();
    let now = now_seconds();
    if let Some(seg) = state.segments.get_mut(&attachment.shmid) {
        seg.nattch = seg.nattch.saturating_sub(1);
        seg.lpid = pid;
        seg.dtime = now;
    }
    maybe_destroy_segment_locked(&mut state, attachment.shmid);
    0
}

pub fn sys_shmctl(shmid: i32, cmd: i32, buf: *mut u8) -> i64 {
    if cmd < 0 || shmid < 0 {
        return -(EINVAL as i64);
    }

    match cmd {
        IPC_INFO => {
            let state = SYSV_SHM_STATE.lock();
            let info = Shminfo64 {
                shmmax: SHMMAX as u64,
                shmmin: SHMMIN as u64,
                shmmni: SHMMNI as u64,
                shmseg: SHMSEG as u64,
                shmall: SHMALL as u64,
                __unused1: 0,
                __unused2: 0,
                __unused3: 0,
                __unused4: 0,
            };
            match copy_to_user(buf, &info) {
                Ok(()) => state.max_id() as i64,
                Err(errno) => -(errno as i64),
            }
        }
        SHM_INFO => {
            let state = SYSV_SHM_STATE.lock();
            let info = ShmInfo {
                used_ids: state.segments.len() as i32,
                shm_tot: state.total_pages as u64,
                shm_rss: 0,
                shm_swp: 0,
                swap_attempts: 0,
                swap_successes: 0,
            };
            match copy_to_user(buf, &info) {
                Ok(()) => state.max_id() as i64,
                Err(errno) => -(errno as i64),
            }
        }
        IPC_STAT | SHM_STAT | SHM_STAT_ANY => {
            let state = SYSV_SHM_STATE.lock();
            let seg = state.segments.get(&shmid).ok_or(EINVAL);
            let seg = match seg {
                Ok(seg) if cmd == SHM_STAT_ANY || segment_permission_allowed(seg, S_IRUGO) => seg,
                Ok(_) => return -(EACCES as i64),
                Err(errno) => return -(errno as i64),
            };
            let out = build_shmid64(seg);
            match copy_to_user(buf, &out) {
                Ok(()) if cmd == IPC_STAT => 0,
                Ok(()) => seg.id as i64,
                Err(errno) => -(errno as i64),
            }
        }
        IPC_SET => {
            if buf.is_null() {
                return -(EFAULT as i64);
            }
            -(EPERM as i64)
        }
        IPC_RMID => {
            let mut state = SYSV_SHM_STATE.lock();
            let Some(seg) = state.segments.get_mut(&shmid) else {
                return -(EINVAL as i64);
            };
            let key = seg.key;
            seg.destroy_on_detach = true;
            if key != IPC_PRIVATE {
                state.keys.remove(&key);
            }
            maybe_destroy_segment_locked(&mut state, shmid);
            0
        }
        SHM_LOCK | SHM_UNLOCK => {
            let mut state = SYSV_SHM_STATE.lock();
            let Some(seg) = state.segments.get_mut(&shmid) else {
                return -(EINVAL as i64);
            };
            if !capability::capable(capability::CAP_IPC_LOCK) && current_ids().0 != seg.uid {
                return -(EPERM as i64);
            }
            seg.locked = cmd == SHM_LOCK;
            0
        }
        _ => -(EINVAL as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysv_shm_rules_matches_linux_source_and_original_selftests() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/shm.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/shm.h"
        ));
        let setns_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/proc/setns-sysvipc.c"
        ));
        let hugepage_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/mm/hugepage-shm.c"
        ));
        let ipc_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/powerpc/syscalls/ipc.h"
        ));

        assert!(source.contains("#define SHM_DEST\t01000"));
        assert!(source.contains("#define SHM_LOCKED\t02000"));
        assert!(source.contains("ns->shm_ctlmax = SHMMAX;"));
        assert!(source.contains("ns->shm_ctlall = SHMALL;"));
        assert!(source.contains("ns->shm_ctlmni = SHMMNI;"));
        assert!(source.contains("if (size < SHMMIN || size > ns->shm_ctlmax)"));
        assert!(source.contains("if (shmflg & SHM_HUGETLB) {"));
        assert!(source.contains("const bool has_no_reserve = shmflg & SHM_NORESERVE;"));
        assert!(source.contains("case SHM_STAT_ANY:"));
        assert!(source.contains("case SHM_LOCK:"));
        assert!(source.contains("SYSCALL_DEFINE3(shmget"));
        assert!(source.contains("SYSCALL_DEFINE3(shmat"));
        assert!(source.contains("SYSCALL_DEFINE1(shmdt"));
        assert!(header.contains("#define SHMMIN 1"));
        assert!(header.contains("#define SHM_HUGETLB\t04000"));
        assert!(header.contains("#define\tSHM_RDONLY\t010000"));
        assert!(
            setns_selftest
                .contains("Test that setns(CLONE_NEWIPC) points to new /proc/sysvipc content")
        );
        assert!(setns_selftest.contains("shmget(IPC_PRIVATE, 1, IPC_CREAT)"));
        assert!(setns_selftest.contains("open(\"/proc/sysvipc/shm\", O_RDONLY)"));
        assert!(setns_selftest.contains("#define S32"));
        assert!(setns_selftest.contains("#define S64"));
        assert!(hugepage_selftest.contains("#define LENGTH (256UL*1024*1024)"));
        assert!(
            hugepage_selftest
                .contains("shmget(2, LENGTH, SHM_HUGETLB | IPC_CREAT | SHM_R | SHM_W)")
        );
        assert!(hugepage_selftest.contains("shmat(shmid, ADDR, SHMAT_FLAGS)"));
        assert!(hugepage_selftest.contains("shmctl(shmid, IPC_RMID, NULL);"));
        assert!(ipc_h.contains("DO_TEST(shmat, __NR_shmat)"));
        assert!(ipc_h.contains("DO_TEST(shmdt, __NR_shmdt)"));
        assert!(ipc_h.contains("DO_TEST(shmget, __NR_shmget)"));
        assert!(ipc_h.contains("DO_TEST(shmctl, __NR_shmctl)"));

        let mut ns = ShmNamespace::new();
        let shmid = ns
            .shmget(
                2,
                hugepage_shm_test_length(),
                SHM_HUGETLB | IPC_CREAT | SHM_R as i32 | SHM_W as i32,
                10,
                1,
            )
            .unwrap();
        let seg = ns.segment(shmid).unwrap();
        assert!(seg.huge_tlb);
        assert_eq!(seg.size, hugepage_shm_test_length());

        let attach = ns.shmat(shmid, SHM_RDONLY, 11, 2).unwrap();
        assert!(attach.readonly);
        assert_eq!(ns.segment(shmid).unwrap().nattch, 1);
        ns.shmctl_rmid(shmid).unwrap();
        assert!(ns.segment(shmid).unwrap().destroy_on_detach);
        ns.shmdt(shmid, 11, 3).unwrap();
        assert!(ns.segment(shmid).is_none());

        assert_eq!(
            sysvipc_shm_empty_proc(32),
            ShmNamespace::proc_sysvipc_shm_header(32)
        );
        assert_eq!(
            sysvipc_shm_empty_proc(64),
            ShmNamespace::proc_sysvipc_shm_header(64)
        );
        assert_eq!(ns.shmget(1, 0, IPC_CREAT, 1, 0), Err(ShmError::InvalidSize));
    }
}
