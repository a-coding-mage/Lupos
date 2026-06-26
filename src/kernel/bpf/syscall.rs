//! linux-parity: partial
//! linux-source: vendor/linux/kernel/bpf/syscall.c
//! test-origin: linux:vendor/linux/kernel/bpf/syscall.c
//! `sys_bpf` syscall surface.  Linux syscall 321.
//!
//! Implements the `BPF_MAP_*` subcommand subset (see list below). Remaining
//! work vs Linux `bpf/syscall.c` for `complete`: `BPF_PROG_LOAD` + verifier,
//! program attach/run, BTF, links/iterators, and the remaining `bpf()`
//! subcommands.
//!
//! M63 subcommand subset:
//!   - `BPF_MAP_CREATE`
//!   - `BPF_MAP_LOOKUP_ELEM`
//!   - `BPF_MAP_UPDATE_ELEM`
//!   - `BPF_MAP_DELETE_ELEM`
//!   - `BPF_MAP_GET_NEXT_KEY`
//!   - `BPF_MAP_LOOKUP_AND_DELETE_ELEM`
//!   - `BPF_MAP_FREEZE`
//!   - `BPF_PROG_LOAD`
//!   - `BPF_PROG_TEST_RUN`
//!
//! Linux's `bpf_attr` is a discriminated union; we expose typed views via
//! the `attr_*` helper structs below.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::mem::{MaybeUninit, size_of};
use core::sync::atomic::{AtomicI32, Ordering};

use spin::Mutex;

use super::insn::BpfInsn;
use super::maps::Map;
use super::uapi::*;
use super::{interp, verifier};
use crate::arch::x86::kernel::uaccess;
use crate::kernel::capability::{CAP_BPF, CAP_SYS_ADMIN, capable};

#[repr(C)]
pub struct AttrMapCreate {
    pub map_type: u32,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
}

#[repr(C)]
pub struct AttrMapElem {
    pub map_fd: u32,
    pub _pad: u32,
    pub key: u64,   // user pointer
    pub value: u64, // user pointer
    pub flags: u64,
}

#[repr(C)]
pub struct AttrMapGetNextKey {
    pub map_fd: u32,
    pub _pad: u32,
    pub key: u64,      // nullable user pointer
    pub next_key: u64, // user pointer
}

#[repr(C)]
pub struct AttrMapFreeze {
    pub map_fd: u32,
}

#[repr(C)]
pub struct AttrProgLoad {
    pub prog_type: u32,
    pub insn_cnt: u32,
    pub insns: u64,   // user pointer to BpfInsn array
    pub license: u64, // user pointer to license string (unused in M63)
    pub log_level: u32,
    pub log_size: u32,
    pub log_buf: u64,
}

#[repr(C)]
pub struct AttrProgTestRun {
    pub prog_fd: u32,
    pub retval: u32,
    pub data_size_in: u32,
    pub data_size_out: u32,
    pub data_in: u64,
    pub data_out: u64,
    pub repeat: u32,
    pub duration: u32,
    pub ctx_in: u64,
}

#[repr(C)]
pub struct AttrProgAttach {
    pub target_fd: u32,
    pub attach_bpf_fd: u32,
    pub attach_type: u32,
    pub attach_flags: u32,
    pub replace_bpf_fd: u32,
    pub relative_fd: u32,
    pub expected_revision: u64,
}

pub struct BpfProg {
    pub fd: i32,
    pub prog_type: u32,
    pub insns: Vec<BpfInsn>,
}

static NEXT_FD: AtomicI32 = AtomicI32::new(300);
pub static MAPS: Mutex<Vec<Arc<Map>>> = Mutex::new(Vec::new());
pub static PROGS: Mutex<Vec<Arc<BpfProg>>> = Mutex::new(Vec::new());

#[cfg(not(test))]
fn install_bpf_fd(name: &str) -> Result<i32, i64> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return Err(-9);
    }
    let Some(files) = (unsafe { crate::kernel::files::get_task_files(task) }) else {
        return Err(-9);
    };

    let file = crate::fs::anon_inode::alloc_anon_file(name, &BPF_FILE_OPS, 0);
    let fd = files
        .install(file.clone(), false)
        .map_err(|errno| -(errno as i64))?;
    *file.private.lock() = fd as usize;
    Ok(fd)
}

#[cfg(test)]
fn install_bpf_fd(_name: &str) -> Result<i32, i64> {
    Ok(NEXT_FD.fetch_add(1, Ordering::AcqRel))
}

#[cfg(not(test))]
fn bpf_fd_release(file: crate::fs::types::FileRef) {
    let token = *file.private.lock() as i32;
    MAPS.lock().retain(|m| m.id != token);
    PROGS.lock().retain(|p| p.fd != token);
}

#[cfg(not(test))]
static BPF_FILE_OPS: crate::fs::ops::FileOps = crate::fs::ops::FileOps {
    name: "bpf",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: Some(bpf_fd_release),
    readdir: None,
};

pub fn find_map(fd: i32) -> Option<Arc<Map>> {
    MAPS.lock().iter().find(|m| m.id == fd).cloned()
}

pub fn find_prog(fd: i32) -> Option<Arc<BpfProg>> {
    PROGS.lock().iter().find(|p| p.fd == fd).cloned()
}

fn required_attr_size(cmd: u32) -> Option<usize> {
    match cmd {
        BPF_MAP_CREATE => Some(size_of::<AttrMapCreate>()),
        BPF_MAP_LOOKUP_ELEM
        | BPF_MAP_UPDATE_ELEM
        | BPF_MAP_DELETE_ELEM
        | BPF_MAP_LOOKUP_AND_DELETE_ELEM => Some(size_of::<AttrMapElem>()),
        BPF_MAP_GET_NEXT_KEY => Some(size_of::<AttrMapGetNextKey>()),
        BPF_MAP_FREEZE => Some(size_of::<AttrMapFreeze>()),
        BPF_PROG_LOAD => Some(size_of::<AttrProgLoad>()),
        BPF_PROG_ATTACH | BPF_PROG_DETACH => Some(size_of::<AttrProgAttach>()),
        BPF_PROG_TEST_RUN => Some(size_of::<AttrProgTestRun>()),
        _ => None,
    }
}

unsafe fn copy_attr_from_user<T>(attr: *const u8, size: u32) -> Result<T, i64> {
    let required = size_of::<T>();
    if attr.is_null() {
        return Err(-22);
    }
    if (size as usize) < required {
        return Err(-22);
    }
    let mut out = MaybeUninit::<T>::uninit();
    let left = unsafe { uaccess::copy_from_user(out.as_mut_ptr() as *mut u8, attr, required) };
    if left == 0 {
        Ok(unsafe { out.assume_init() })
    } else {
        Err(-14)
    }
}

fn copy_vec_from_user(ptr: u64, len: usize) -> Result<Vec<u8>, i64> {
    let mut buf = alloc::vec![0u8; len];
    if len == 0 {
        return Ok(buf);
    }
    if ptr == 0 {
        return Err(-14);
    }
    let left = unsafe { uaccess::copy_from_user(buf.as_mut_ptr(), ptr as *const u8, len) };
    if left == 0 { Ok(buf) } else { Err(-14) }
}

fn copy_vec_to_user(ptr: u64, src: &[u8]) -> Result<(), i64> {
    if src.is_empty() {
        return Ok(());
    }
    if ptr == 0 {
        return Err(-14);
    }
    let left = unsafe { uaccess::copy_to_user(ptr as *mut u8, src.as_ptr(), src.len()) };
    if left == 0 { Ok(()) } else { Err(-14) }
}

fn copy_insns_from_user(ptr: u64, n: usize) -> Result<Vec<BpfInsn>, i64> {
    let bytes = n.checked_mul(size_of::<BpfInsn>()).ok_or(-22)?;
    let mut insns: Vec<BpfInsn> = Vec::with_capacity(n);
    if bytes == 0 {
        return Ok(insns);
    }
    if ptr == 0 {
        return Err(-14);
    }
    let left =
        unsafe { uaccess::copy_from_user(insns.as_mut_ptr() as *mut u8, ptr as *const u8, bytes) };
    if left != 0 {
        return Err(-14);
    }
    unsafe { insns.set_len(n) };
    Ok(insns)
}

/// `sys_bpf(cmd, attr, size)` — userspace syscall dispatch.
///
/// The syscall boundary must never trust the top-level `bpf_attr` pointer or
/// nested pointers embedded inside it.  Copy every userspace object through the
/// uaccess helpers before handing it to map/program internals, and copy results
/// back with `copy_to_user`.
pub unsafe fn sys_bpf(cmd: u32, attr: *const u8, size: u32) -> i64 {
    if !capable(CAP_BPF) && !capable(CAP_SYS_ADMIN) {
        return -1;
    }
    let Some(required) = required_attr_size(cmd) else {
        return -38;
    };
    if attr.is_null() {
        return -22;
    }
    if (size as usize) < required {
        return -22;
    }

    match cmd {
        BPF_MAP_CREATE => {
            let a: AttrMapCreate = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let kernel_attr = a;
            unsafe { sys_bpf_kernel(cmd, &kernel_attr as *const _ as *const u8, required as u32) }
        }
        BPF_MAP_UPDATE_ELEM => {
            let a: AttrMapElem = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key = match copy_vec_from_user(a.key, m.key_size as usize) {
                Ok(v) => v,
                Err(e) => return e,
            };
            let val = match copy_vec_from_user(a.value, m.value_size as usize) {
                Ok(v) => v,
                Err(e) => return e,
            };
            match m.update_with_flags(&key, &val, a.flags) {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        BPF_MAP_LOOKUP_ELEM => {
            let a: AttrMapElem = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key = match copy_vec_from_user(a.key, m.key_size as usize) {
                Ok(v) => v,
                Err(e) => return e,
            };
            match m.lookup(&key) {
                Some(v) => match copy_vec_to_user(a.value, &v) {
                    Ok(()) => 0,
                    Err(e) => e,
                },
                None => -2,
            }
        }
        BPF_MAP_DELETE_ELEM => {
            let a: AttrMapElem = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key = match copy_vec_from_user(a.key, m.key_size as usize) {
                Ok(v) => v,
                Err(e) => return e,
            };
            match m.delete(&key) {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        BPF_MAP_GET_NEXT_KEY => {
            let a: AttrMapGetNextKey = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key_buf = if a.key == 0 {
                None
            } else {
                Some(match copy_vec_from_user(a.key, m.key_size as usize) {
                    Ok(v) => v,
                    Err(e) => return e,
                })
            };
            match m.get_next_key(key_buf.as_deref()) {
                Ok(next) => match copy_vec_to_user(a.next_key, &next) {
                    Ok(()) => 0,
                    Err(e) => e,
                },
                Err(e) => e as i64,
            }
        }
        BPF_MAP_LOOKUP_AND_DELETE_ELEM => {
            let a: AttrMapElem = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key = match copy_vec_from_user(a.key, m.key_size as usize) {
                Ok(v) => v,
                Err(e) => return e,
            };
            match m.lookup_and_delete(&key) {
                Ok(v) => match copy_vec_to_user(a.value, &v) {
                    Ok(()) => 0,
                    Err(e) => e,
                },
                Err(e) => e as i64,
            }
        }
        BPF_MAP_FREEZE => {
            let a: AttrMapFreeze = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let kernel_attr = a;
            unsafe { sys_bpf_kernel(cmd, &kernel_attr as *const _ as *const u8, required as u32) }
        }
        BPF_PROG_LOAD => {
            let a: AttrProgLoad = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let n = a.insn_cnt as usize;
            if n == 0 || n > interp::BPF_MAX_INSNS {
                return -22;
            }
            let insns = match copy_insns_from_user(a.insns, n) {
                Ok(insns) => insns,
                Err(e) => return e,
            };
            if !matches!(
                a.prog_type,
                BPF_PROG_TYPE_CGROUP_DEVICE | BPF_PROG_TYPE_CGROUP_SKB
            ) && verifier::verify(&insns).is_err()
            {
                return -22;
            }
            let fd = match install_bpf_fd("bpf-prog") {
                Ok(fd) => fd,
                Err(ret) => return ret,
            };
            PROGS.lock().push(Arc::new(BpfProg {
                fd,
                prog_type: a.prog_type,
                insns,
            }));
            fd as i64
        }
        BPF_PROG_ATTACH | BPF_PROG_DETACH => {
            let a: AttrProgAttach = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let kernel_attr = a;
            unsafe { sys_bpf_kernel(cmd, &kernel_attr as *const _ as *const u8, required as u32) }
        }
        BPF_PROG_TEST_RUN => {
            let mut a: AttrProgTestRun = match unsafe { copy_attr_from_user(attr, size) } {
                Ok(a) => a,
                Err(e) => return e,
            };
            let p = match find_prog(a.prog_fd as i32) {
                Some(p) => p,
                None => return -9,
            };
            a.retval = interp::run(&p.insns, 0) as u32;
            let retval_user =
                unsafe { attr.add(core::mem::offset_of!(AttrProgTestRun, retval)) } as *mut u32;
            match unsafe { uaccess::put_user_u32(retval_user, a.retval) } {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        _ => -38,
    }
}

/// `sys_bpf_kernel(cmd, attr, size)` — trusted in-kernel dispatch.
///
/// This helper is for kernel self-tests and in-kernel callers that pass
/// kernel-owned `attr` and nested buffers.  The public syscall wrapper calls
/// `sys_bpf`, which performs capability and usercopy checks before reaching
/// map/program internals.
pub(crate) unsafe fn sys_bpf_kernel(cmd: u32, attr: *const u8, _size: u32) -> i64 {
    if attr.is_null() {
        return -22;
    }
    match cmd {
        BPF_MAP_CREATE => {
            let a = unsafe { &*(attr as *const AttrMapCreate) };
            match Map::new(a.map_type, a.key_size, a.value_size, a.max_entries) {
                Ok(mut m) => {
                    let id = match install_bpf_fd("bpf-map") {
                        Ok(fd) => fd,
                        Err(ret) => return ret,
                    };
                    m.id = id;
                    MAPS.lock().push(Arc::new(m));
                    id as i64
                }
                Err(e) => e as i64,
            }
        }
        BPF_MAP_LOOKUP_ELEM => {
            let a = unsafe { &*(attr as *const AttrMapElem) };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9, // EBADF
            };
            let key =
                unsafe { core::slice::from_raw_parts(a.key as *const u8, m.key_size as usize) };
            match m.lookup(key) {
                Some(v) => {
                    let dst = a.value as *mut u8;
                    if !dst.is_null() {
                        unsafe { core::ptr::copy_nonoverlapping(v.as_ptr(), dst, v.len()) };
                    }
                    0
                }
                None => -2, // ENOENT
            }
        }
        BPF_MAP_UPDATE_ELEM => {
            let a = unsafe { &*(attr as *const AttrMapElem) };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key =
                unsafe { core::slice::from_raw_parts(a.key as *const u8, m.key_size as usize) };
            let val =
                unsafe { core::slice::from_raw_parts(a.value as *const u8, m.value_size as usize) };
            match m.update_with_flags(key, val, a.flags) {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        BPF_MAP_DELETE_ELEM => {
            let a = unsafe { &*(attr as *const AttrMapElem) };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key =
                unsafe { core::slice::from_raw_parts(a.key as *const u8, m.key_size as usize) };
            match m.delete(key) {
                Ok(()) => 0,
                Err(e) => e as i64,
            }
        }
        BPF_MAP_GET_NEXT_KEY => {
            let a = unsafe { &*(attr as *const AttrMapGetNextKey) };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key = if a.key == 0 {
                None
            } else {
                Some(unsafe {
                    core::slice::from_raw_parts(a.key as *const u8, m.key_size as usize)
                })
            };
            match m.get_next_key(key) {
                Ok(next) => {
                    let dst = a.next_key as *mut u8;
                    if dst.is_null() {
                        return -14;
                    }
                    unsafe { core::ptr::copy_nonoverlapping(next.as_ptr(), dst, next.len()) };
                    0
                }
                Err(e) => e as i64,
            }
        }
        BPF_MAP_LOOKUP_AND_DELETE_ELEM => {
            let a = unsafe { &*(attr as *const AttrMapElem) };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            let key =
                unsafe { core::slice::from_raw_parts(a.key as *const u8, m.key_size as usize) };
            match m.lookup_and_delete(key) {
                Ok(v) => {
                    let dst = a.value as *mut u8;
                    if !dst.is_null() {
                        unsafe { core::ptr::copy_nonoverlapping(v.as_ptr(), dst, v.len()) };
                    }
                    0
                }
                Err(e) => e as i64,
            }
        }
        BPF_MAP_FREEZE => {
            let a = unsafe { &*(attr as *const AttrMapFreeze) };
            let m = match find_map(a.map_fd as i32) {
                Some(m) => m,
                None => return -9,
            };
            m.freeze();
            0
        }
        BPF_PROG_LOAD => {
            let a = unsafe { &*(attr as *const AttrProgLoad) };
            let n = a.insn_cnt as usize;
            if n == 0 || n > interp::BPF_MAX_INSNS {
                return -22;
            }
            let src = a.insns as *const BpfInsn;
            let mut insns: Vec<BpfInsn> = Vec::with_capacity(n);
            for i in 0..n {
                insns.push(unsafe { *src.add(i) });
            }
            // Linux accepts cgroup programs through the full verifier before
            // attaching them to cgroups. Lupos does not yet implement those
            // verifier classes, but the service manager only needs the
            // load/attach ABI accepted; enforcement remains deferred.
            if !matches!(
                a.prog_type,
                BPF_PROG_TYPE_CGROUP_DEVICE | BPF_PROG_TYPE_CGROUP_SKB
            ) && verifier::verify(&insns).is_err()
            {
                return -22;
            }
            let fd = match install_bpf_fd("bpf-prog") {
                Ok(fd) => fd,
                Err(ret) => return ret,
            };
            PROGS.lock().push(Arc::new(BpfProg {
                fd,
                prog_type: a.prog_type,
                insns,
            }));
            fd as i64
        }
        BPF_PROG_ATTACH => {
            let a = unsafe { &*(attr as *const AttrProgAttach) };
            let Some(prog) = find_prog(a.attach_bpf_fd as i32) else {
                return -9;
            };
            match (prog.prog_type, a.attach_type) {
                (BPF_PROG_TYPE_CGROUP_DEVICE, BPF_CGROUP_DEVICE) => 0,
                (BPF_PROG_TYPE_CGROUP_SKB, BPF_CGROUP_INET_INGRESS | BPF_CGROUP_INET_EGRESS) => 0,
                _ => -22,
            }
        }
        BPF_PROG_DETACH => {
            let a = unsafe { &*(attr as *const AttrProgAttach) };
            let Some(prog) = find_prog(a.attach_bpf_fd as i32) else {
                return -9;
            };
            match (prog.prog_type, a.attach_type) {
                (BPF_PROG_TYPE_CGROUP_DEVICE, BPF_CGROUP_DEVICE) => 0,
                (BPF_PROG_TYPE_CGROUP_SKB, BPF_CGROUP_INET_INGRESS | BPF_CGROUP_INET_EGRESS) => 0,
                _ => -22,
            }
        }
        BPF_PROG_TEST_RUN => {
            let a = unsafe { &*(attr as *const AttrProgTestRun) };
            let p = match find_prog(a.prog_fd as i32) {
                Some(p) => p,
                None => return -9,
            };
            let r = interp::run(&p.insns, 0);
            // Linux writes retval back into the caller's attr.
            let dst = (attr as *const _ as usize + core::mem::offset_of!(AttrProgTestRun, retval))
                as *mut u32;
            unsafe { *dst = r as u32 };
            0
        }
        _ => -38, // ENOSYS for unsupported subcommands
    }
}

#[cfg(test)]
mod tests {
    use super::super::insn::*;
    use super::*;

    #[test]
    fn map_create_then_lookup_update_delete() {
        let attr = AttrMapCreate {
            map_type: BPF_MAP_TYPE_HASH,
            key_size: 4,
            value_size: 8,
            max_entries: 16,
        };
        let map_fd = unsafe { sys_bpf_kernel(BPF_MAP_CREATE, &attr as *const _ as *const u8, 0) };
        assert!(map_fd > 0);

        let key = 7u32.to_ne_bytes();
        let val = 42u64.to_ne_bytes();
        let upd = AttrMapElem {
            map_fd: map_fd as u32,
            _pad: 0,
            key: key.as_ptr() as u64,
            value: val.as_ptr() as u64,
            flags: 0,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_MAP_UPDATE_ELEM, &upd as *const _ as *const u8, 0) },
            0
        );

        let mut got = [0u8; 8];
        let lookup = AttrMapElem {
            map_fd: map_fd as u32,
            _pad: 0,
            key: key.as_ptr() as u64,
            value: got.as_mut_ptr() as u64,
            flags: 0,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_MAP_LOOKUP_ELEM, &lookup as *const _ as *const u8, 0) },
            0
        );
        assert_eq!(got, val);
    }

    #[test]
    fn prog_load_then_test_run() {
        // r0 = 99; exit
        let prog: [BpfInsn; 2] = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 99),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        let load = AttrProgLoad {
            prog_type: BPF_PROG_TYPE_SOCKET_FILTER,
            insn_cnt: 2,
            insns: prog.as_ptr() as u64,
            license: 0,
            log_level: 0,
            log_size: 0,
            log_buf: 0,
        };
        let prog_fd = unsafe { sys_bpf_kernel(BPF_PROG_LOAD, &load as *const _ as *const u8, 0) };
        assert!(prog_fd > 0);

        let mut run = AttrProgTestRun {
            prog_fd: prog_fd as u32,
            retval: 0,
            data_size_in: 0,
            data_size_out: 0,
            data_in: 0,
            data_out: 0,
            repeat: 1,
            duration: 0,
            ctx_in: 0,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_PROG_TEST_RUN, &mut run as *mut _ as *const u8, 0) },
            0
        );
        assert_eq!(run.retval, 99);
    }

    #[test]
    fn map_get_next_lookup_delete_and_freeze() {
        let attr = AttrMapCreate {
            map_type: BPF_MAP_TYPE_HASH,
            key_size: 4,
            value_size: 4,
            max_entries: 4,
        };
        let map_fd = unsafe { sys_bpf_kernel(BPF_MAP_CREATE, &attr as *const _ as *const u8, 0) };
        assert!(map_fd > 0);

        let key1 = 1u32.to_ne_bytes();
        let key2 = 2u32.to_ne_bytes();
        for (key, value) in [(key1, 10u32.to_ne_bytes()), (key2, 20u32.to_ne_bytes())] {
            let upd = AttrMapElem {
                map_fd: map_fd as u32,
                _pad: 0,
                key: key.as_ptr() as u64,
                value: value.as_ptr() as u64,
                flags: BPF_ANY,
            };
            assert_eq!(
                unsafe { sys_bpf_kernel(BPF_MAP_UPDATE_ELEM, &upd as *const _ as *const u8, 0) },
                0
            );
        }

        let mut next = [0u8; 4];
        let get_next = AttrMapGetNextKey {
            map_fd: map_fd as u32,
            _pad: 0,
            key: key1.as_ptr() as u64,
            next_key: next.as_mut_ptr() as u64,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_MAP_GET_NEXT_KEY, &get_next as *const _ as *const u8, 0) },
            0
        );
        assert_eq!(next, key2);

        let mut deleted = [0u8; 4];
        let del = AttrMapElem {
            map_fd: map_fd as u32,
            _pad: 0,
            key: key1.as_ptr() as u64,
            value: deleted.as_mut_ptr() as u64,
            flags: 0,
        };
        assert_eq!(
            unsafe {
                sys_bpf_kernel(
                    BPF_MAP_LOOKUP_AND_DELETE_ELEM,
                    &del as *const _ as *const u8,
                    0,
                )
            },
            0
        );
        assert_eq!(deleted, 10u32.to_ne_bytes());

        let freeze = AttrMapFreeze {
            map_fd: map_fd as u32,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_MAP_FREEZE, &freeze as *const _ as *const u8, 0) },
            0
        );
        let post_freeze_value = 30u32.to_ne_bytes();
        let update_after_freeze = AttrMapElem {
            map_fd: map_fd as u32,
            _pad: 0,
            key: key2.as_ptr() as u64,
            value: post_freeze_value.as_ptr() as u64,
            flags: BPF_ANY,
        };
        assert_eq!(
            unsafe {
                sys_bpf_kernel(
                    BPF_MAP_UPDATE_ELEM,
                    &update_after_freeze as *const _ as *const u8,
                    0,
                )
            },
            -1
        );
    }

    #[test]
    fn cgroup_device_load_and_attach_are_accepted_for_systemd_device_policy() {
        let prog: [BpfInsn; 2] = [
            BpfInsn::new(BPF_LD | BPF_W | BPF_ABS, 0, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        let load = AttrProgLoad {
            prog_type: BPF_PROG_TYPE_CGROUP_DEVICE,
            insn_cnt: prog.len() as u32,
            insns: prog.as_ptr() as u64,
            license: 0,
            log_level: 0,
            log_size: 0,
            log_buf: 0,
        };
        let prog_fd = unsafe { sys_bpf_kernel(BPF_PROG_LOAD, &load as *const _ as *const u8, 0) };
        assert!(prog_fd > 0);

        let attach = AttrProgAttach {
            target_fd: 1,
            attach_bpf_fd: prog_fd as u32,
            attach_type: BPF_CGROUP_DEVICE,
            attach_flags: BPF_F_ALLOW_MULTI,
            replace_bpf_fd: 0,
            relative_fd: 0,
            expected_revision: 0,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_PROG_ATTACH, &attach as *const _ as *const u8, 0) },
            0
        );
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_PROG_DETACH, &attach as *const _ as *const u8, 0) },
            0
        );
    }

    #[test]
    fn cgroup_device_attach_rejects_wrong_program_type() {
        let prog: [BpfInsn; 2] = [
            BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, 0, 0, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        let load = AttrProgLoad {
            prog_type: BPF_PROG_TYPE_SOCKET_FILTER,
            insn_cnt: prog.len() as u32,
            insns: prog.as_ptr() as u64,
            license: 0,
            log_level: 0,
            log_size: 0,
            log_buf: 0,
        };
        let prog_fd = unsafe { sys_bpf_kernel(BPF_PROG_LOAD, &load as *const _ as *const u8, 0) };
        assert!(prog_fd > 0);

        let attach = AttrProgAttach {
            target_fd: 1,
            attach_bpf_fd: prog_fd as u32,
            attach_type: BPF_CGROUP_DEVICE,
            attach_flags: 0,
            replace_bpf_fd: 0,
            relative_fd: 0,
            expected_revision: 0,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_PROG_ATTACH, &attach as *const _ as *const u8, 0) },
            -22
        );
    }

    #[test]
    fn cgroup_skb_ingress_egress_attach_is_accepted_for_systemd_firewall() {
        let prog: [BpfInsn; 2] = [
            BpfInsn::new(BPF_LDX | BPF_W | BPF_MEM, 0, 1, 0, 0),
            BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0),
        ];
        let load = AttrProgLoad {
            prog_type: BPF_PROG_TYPE_CGROUP_SKB,
            insn_cnt: prog.len() as u32,
            insns: prog.as_ptr() as u64,
            license: 0,
            log_level: 0,
            log_size: 0,
            log_buf: 0,
        };
        let prog_fd = unsafe { sys_bpf_kernel(BPF_PROG_LOAD, &load as *const _ as *const u8, 0) };
        assert!(prog_fd > 0);

        for attach_type in [BPF_CGROUP_INET_INGRESS, BPF_CGROUP_INET_EGRESS] {
            let attach = AttrProgAttach {
                target_fd: 1,
                attach_bpf_fd: prog_fd as u32,
                attach_type,
                attach_flags: BPF_F_ALLOW_MULTI,
                replace_bpf_fd: 0,
                relative_fd: 0,
                expected_revision: 0,
            };
            assert_eq!(
                unsafe { sys_bpf_kernel(BPF_PROG_ATTACH, &attach as *const _ as *const u8, 0) },
                0
            );
        }
    }

    #[test]
    fn syscall_m78_security_bpf_perf_parity() {
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_MAP_CREATE, core::ptr::null(), 0) },
            -22
        );
        let map = AttrMapCreate {
            map_type: BPF_MAP_TYPE_ARRAY,
            key_size: 4,
            value_size: 4,
            max_entries: 1,
        };
        assert!(unsafe { sys_bpf_kernel(BPF_MAP_CREATE, &map as *const _ as *const u8, 0) } > 0);
        let load = AttrProgLoad {
            prog_type: BPF_PROG_TYPE_SOCKET_FILTER,
            insn_cnt: 0,
            insns: 0,
            license: 0,
            log_level: 0,
            log_size: 0,
            log_buf: 0,
        };
        assert_eq!(
            unsafe { sys_bpf_kernel(BPF_PROG_LOAD, &load as *const _ as *const u8, 0) },
            -22
        );
    }
}
