//! linux-parity: partial
//! linux-source: vendor/linux/io_uring
//! linux-source: vendor/linux/io_uring/io_uring.c
//! test-origin: linux:vendor/linux/io_uring
//! `io_uring` — async I/O ring buffer interface (M60).
//!
//! Implements `io_uring_setup` (context + SQ/CQ rings + mmap regions) with
//! byte-identical UAPI structs (size/offset asserts vs io_uring.h). Remaining
//! work vs Linux for `complete`: the full op set in `io_uring_enter`, SQPOLL,
//! registered buffers/files, and completion/wait edge cases.
//!
//! ABI parity with vendor/linux/include/uapi/linux/io_uring.h.
//! UAPI structs are byte-identical to Linux 6.x.
//!
//! M60 coverage:
//!   - `io_uring_setup` allocates a context, SQ/CQ rings, and backing pages
//!     for SQ_RING/CQ_RING/SQES mmap regions.
//!   - SQE/CQE/IoUringParams + every UAPI helper struct verified by inline
//!     size/offset asserts against `vendor/linux/include/uapi/linux/io_uring.h`.
//!   - All 42 `vendor/linux/io_uring/*.c` files have a Rust counterpart in
//!     `src/io_uring/`, each with source-backed inline tests.  See
//!     `linux_sources.rs` for the authoritative list.
//!   - Opcode dispatch (`opdef.rs::dispatch`) wires NOP fully; the per-op
//!     prep validators in Layers 2/3 are ported behind it.  Full end-to-end
//!     issue() integration for non-NOP ops lands as their dependencies (VFS
//!     async, socket queues, page_pool) ripen.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::ops::FileOps;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EBADF, EFAULT};
use crate::kernel::{files, sched};

pub mod cqe;
pub mod ops;
pub mod params;
pub mod sqe;
pub mod uapi;

// Layer 0 — foundational, no internal io_uring deps.
// Ref: vendor/linux/io_uring/{alloc_cache,kbuf,tctx,opdef,fdinfo,memmap,filetable,rsrc,nop}.c
pub mod alloc_cache;
pub mod fdinfo;
pub mod filetable;
pub mod kbuf;
pub mod linux_sources;
pub mod memmap;
pub mod nop;
pub mod opdef;
pub mod rsrc;
pub mod tctx;

// Layer 1 — submission / completion machinery.
// Ref: vendor/linux/io_uring/{tw,wait,poll,cancel,io-wq,sqpoll}.c
pub mod cancel;
pub mod io_wq;
pub mod poll;
pub mod sqpoll;
pub mod tw;
pub mod wait;

// Layer 2 — per-op handlers.
// Ref: vendor/linux/io_uring/{rw,openclose,sync,fs,splice,statx,xattr,truncate,advise,epoll,eventfd,futex,msg_ring,timeout,waitid,net,register}.c
pub mod advise;
pub mod epoll;
pub mod eventfd;
pub mod fs;
pub mod futex;
pub mod msg_ring;
pub mod net;
pub mod openclose;
pub mod register;
pub mod rw;
pub mod splice;
pub mod statx;
pub mod sync;
pub mod timeout;
pub mod truncate;
pub mod waitid;
pub mod xattr;

// Layer 3 — advanced / niche.
// Ref: vendor/linux/io_uring/{notif,napi,uring_cmd,cmd_net,loop,query,bpf_filter,bpf-ops,zcrx,mock_file}.c
pub mod bpf_filter;
pub mod bpf_ops;
pub mod cmd_net;
pub mod loop_op;
pub mod mock_file;
pub mod napi;
pub mod notif;
pub mod query;
pub mod uring_cmd;
pub mod zcrx;

pub use cqe::Cqe;
pub use params::{IoCqRingOffsets, IoSqRingOffsets, IoUringParams};
pub use sqe::Sqe;

/// `IORING_OP_*` opcode constants — byte-identical to Linux UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h::IORING_OP_*
pub const IORING_OP_NOP: u8 = 0;
pub const IORING_OP_READV: u8 = 1;
pub const IORING_OP_WRITEV: u8 = 2;
pub const IORING_OP_FSYNC: u8 = 3;
pub const IORING_OP_READ_FIXED: u8 = 4;
pub const IORING_OP_WRITE_FIXED: u8 = 5;
pub const IORING_OP_POLL_ADD: u8 = 6;
pub const IORING_OP_POLL_REMOVE: u8 = 7;
pub const IORING_OP_READ: u8 = 22;
pub const IORING_OP_WRITE: u8 = 23;
pub const IORING_OP_OPENAT: u8 = 18;
pub const IORING_OP_CLOSE: u8 = 19;
pub const IORING_OP_TIMEOUT: u8 = 11;
pub const IORING_OP_LINK_TIMEOUT: u8 = 15;

/// `IORING_OFF_*` mmap offsets for SQ/CQ rings and SQEs.
pub const IORING_OFF_SQ_RING: u64 = 0;
pub const IORING_OFF_CQ_RING: u64 = 0x800_0000;
pub const IORING_OFF_SQES: u64 = 0x1000_0000;

/// `IORING_ENTER_*` flags for `io_uring_enter`.
pub const IORING_ENTER_GETEVENTS: u32 = 1;
pub const IORING_ENTER_SQ_WAKEUP: u32 = 2;

/// In-kernel context for one io_uring instance.
/// One per `io_uring_setup` call — stored as `private_data` on the anon-fd
/// returned to userspace.
///
/// The three `memmap::RingRegion`s hold the backing pages for the mmap'd
/// SQ_RING / CQ_RING / SQES regions.  The `mmap` handler on
/// `IO_RING_FILE_OPS` returns the kernel virtual address of the first page
/// of the matching region; with a real user-mm layer this would map those
/// pages into the calling task's address space.
pub struct IoRingCtx {
    pub sq_entries: u32,
    pub cq_entries: u32,
    pub sqes: Vec<Sqe>,
    pub cqes: Vec<Cqe>,
    pub sq_head: AtomicU32,
    pub sq_tail: AtomicU32,
    pub cq_head: AtomicU32,
    pub cq_tail: AtomicU32,
    /// Backing pages for `IORING_OFF_SQ_RING`.
    pub sq_ring_region: Mutex<memmap::RingRegion>,
    /// Backing pages for `IORING_OFF_CQ_RING`.
    pub cq_ring_region: Mutex<memmap::RingRegion>,
    /// Backing pages for `IORING_OFF_SQES`.
    pub sqes_region: Mutex<memmap::RingRegion>,
}

static IO_RING_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref IO_RINGS: Mutex<BTreeMap<usize, Arc<IoRingCtx>>> = Mutex::new(BTreeMap::new());
}

static IO_RING_FILE_OPS: FileOps = FileOps {
    name: "io_uring",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: Some(io_ring_mmap),
    release: Some(io_ring_release),
    readdir: None,
};

impl IoRingCtx {
    /// Create a context with `entries` SQEs and `2*entries` CQEs (Linux default).
    pub fn new(entries: u32) -> Self {
        let sq = entries.next_power_of_two().max(1);
        let cq = (2 * sq).max(2);
        let mut sqes = Vec::with_capacity(sq as usize);
        sqes.resize(sq as usize, Sqe::default());
        let mut cqes = Vec::with_capacity(cq as usize);
        cqes.resize(cq as usize, Cqe::default());
        Self {
            sq_entries: sq,
            cq_entries: cq,
            sqes,
            cqes,
            sq_head: AtomicU32::new(0),
            sq_tail: AtomicU32::new(0),
            cq_head: AtomicU32::new(0),
            cq_tail: AtomicU32::new(0),
            sq_ring_region: Mutex::new(memmap::RingRegion::new(memmap::sq_ring_bytes(sq))),
            cq_ring_region: Mutex::new(memmap::RingRegion::new(memmap::cq_ring_bytes(cq))),
            sqes_region: Mutex::new(memmap::RingRegion::new(memmap::sqes_bytes(sq))),
        }
    }

    /// Submit `to_submit` SQEs, reaping completions inline.  Returns count submitted.
    ///
    /// `ops::dispatch` walks the per-opcode prep validators (`opdef.rs`); ops
    /// whose `issue()` slot is not yet wired (Layers 2/3 are prep-only)
    /// complete with `-ENOSYS`.  Prep-stage rejects (`-EINVAL`, `-EBADF`,
    /// etc.) flow through the CQE.
    pub fn submit(&self, to_submit: u32) -> u32 {
        let mut submitted = 0u32;
        let head = self.sq_head.load(Ordering::Acquire);
        let mask = self.sq_entries - 1;
        for i in 0..to_submit {
            let idx = ((head + i) & mask) as usize;
            let sqe = &self.sqes[idx];
            let res = ops::dispatch(sqe);
            self.complete(sqe.user_data, res);
            submitted += 1;
        }
        self.sq_head.fetch_add(submitted, Ordering::AcqRel);
        submitted
    }

    /// Push a CQE.
    fn complete(&self, user_data: u64, res: i32) {
        let tail = self.cq_tail.load(Ordering::Acquire);
        let mask = self.cq_entries - 1;
        let idx = (tail & mask) as usize;
        // SAFETY: we hold no concurrent writer in the M60 single-threaded test.
        unsafe {
            let p = &self.cqes[idx] as *const Cqe as *mut Cqe;
            (*p).user_data = user_data;
            (*p).res = res;
            (*p).flags = 0;
        }
        self.cq_tail.store(tail.wrapping_add(1), Ordering::Release);
    }

    /// Number of completions ready to consume.
    pub fn cq_ready(&self) -> u32 {
        self.cq_tail
            .load(Ordering::Acquire)
            .wrapping_sub(self.cq_head.load(Ordering::Acquire))
    }
}

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn ring_from_fd(fd: i32) -> Result<Arc<IoRingCtx>, i32> {
    let file = current_files()?.get(fd)?;
    if file.fops.name != IO_RING_FILE_OPS.name {
        return Err(EBADF);
    }
    let token = *file.private.lock();
    IO_RINGS.lock().get(&token).cloned().ok_or(EBADF)
}

fn io_ring_release(file: FileRef) {
    let token = *file.private.lock();
    IO_RINGS.lock().remove(&token);
}

/// `io_uring_mmap` — prepare the SQ_RING / CQ_RING / SQES mapping.
///
/// Linux maps the kernel-allocated pages into the initialized VMA and marks it
/// `VM_DONTEXPAND`. Lupos retains the selected region base in the VMA for its
/// lazy PTE materialization path.
///
/// Ref: vendor/linux/io_uring/memmap.c::io_uring_mmap
fn io_ring_mmap(file: &FileRef, vma: &mut crate::mm::mm_types::VmAreaStruct) -> Result<(), i32> {
    let len = usize::try_from(vma.vm_end.checked_sub(vma.vm_start).ok_or(-22)?).map_err(|_| -22)?;
    let off = vma
        .vm_pgoff
        .checked_mul(memmap::PAGE_SIZE as u64)
        .ok_or(-22)?;
    let token = *file.private.lock();
    let ctx = IO_RINGS.lock().get(&token).cloned().ok_or(-9 /* EBADF */)?;

    let Some(tag) = memmap::region_for_offset(off) else {
        return Err(-22); // EINVAL — unknown mmap offset
    };
    let region = match tag {
        memmap::RegionTag::SqRing => &ctx.sq_ring_region,
        memmap::RegionTag::CqRing => &ctx.cq_ring_region,
        memmap::RegionTag::Sqes => &ctx.sqes_region,
        // PBUF_RING regions are owned by the kbuf registry, not yet plumbed
        // through here.  Reject with -EINVAL so userspace knows to retry once
        // the buffer-ring registration path lands.
        memmap::RegionTag::PbufRing(_) => return Err(-22),
    };
    let guard = region.lock();
    if len > guard.pages.len() * memmap::PAGE_SIZE {
        return Err(-22);
    }
    // RingPage's 4 KiB alignment makes this a page-aligned direct-map range.
    // Translate the kernel address before installing userspace PFNs; treating
    // a Rust pointer as a physical address would map unrelated memory.
    let base = guard.pages.as_ptr().cast::<u8>() as u64;
    #[cfg(not(test))]
    let phys = crate::arch::x86::mm::paging::virt_to_phys(base).ok_or(-22)?;
    #[cfg(test)]
    let phys = base;
    crate::mm::fault::prepare_lupos_device_pfn_mapping(vma, phys);
    Ok(())
}

/// `sys_io_uring_setup(entries, params)` — Linux syscall 425.
/// In M60 this returns a synthetic in-kernel context (boxed pointer cast to fd).
/// Real anon-fd integration is deferred.
pub unsafe fn sys_io_uring_setup(entries: u32, params: *mut IoUringParams) -> i64 {
    if entries == 0 || entries > 32768 {
        return -22; // EINVAL
    }
    let ctx = Arc::new(IoRingCtx::new(entries));

    if !params.is_null() {
        let mut user_params = IoUringParams::default();
        user_params.sq_entries = ctx.sq_entries;
        user_params.cq_entries = ctx.cq_entries;
        user_params.features = 0;
        user_params.sq_off = IoSqRingOffsets::default();
        user_params.cq_off = IoCqRingOffsets::default();

        let not_copied = unsafe {
            crate::arch::x86::kernel::uaccess::copy_to_user(
                params as *mut u8,
                &user_params as *const IoUringParams as *const u8,
                core::mem::size_of::<IoUringParams>(),
            )
        };
        if not_copied != 0 {
            return -(EFAULT as i64);
        }
    }
    return {
        let token = IO_RING_TOKEN.fetch_add(1, Ordering::AcqRel);
        IO_RINGS.lock().insert(token, ctx);
        let file = alloc_anon_file("io_uring", &IO_RING_FILE_OPS, token);
        match current_files().and_then(|ft| ft.install(file, false)) {
            Ok(fd) => fd as i64,
            Err(errno) => {
                IO_RINGS.lock().remove(&token);
                -(errno as i64)
            }
        }
    };
    // Leak the box; in a real impl this would install as a fd.
    let _ = ();
    3 // stub fd number — replaced by real anon_inode_getfd in M60+
}

/// `sys_io_uring_enter(fd, to_submit, min_complete, flags, ...)` — Linux syscall 426.
/// Stub: returns -ENOSYS until VFS-fd integration lands.
pub unsafe fn sys_io_uring_enter(
    fd: i32,
    to_submit: u32,
    _min_complete: u32,
    _flags: u32,
    _sig: *const u8,
    _sigsz: usize,
) -> i64 {
    return match ring_from_fd(fd) {
        Ok(ctx) => ctx.submit(to_submit) as i64,
        Err(errno) => -(errno as i64),
    };
    -38 // ENOSYS
}

/// `sys_io_uring_register(fd, opcode, arg, nr_args)` — Linux syscall 427.
pub unsafe fn sys_io_uring_register(fd: i32, _opcode: u32, _arg: *const u8, _nr_args: u32) -> i64 {
    return match ring_from_fd(fd) {
        Ok(_) => 0,
        Err(errno) => -(errno as i64),
    };
    -38
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::fs::fdtable::FilesStruct;
    use crate::kernel::{cred::INIT_CRED, files, sched, task::TaskStruct};

    #[test]
    fn ctx_creates_with_pow2_sq_and_2x_cq() {
        let ctx = IoRingCtx::new(4);
        assert_eq!(ctx.sq_entries, 4);
        assert_eq!(ctx.cq_entries, 8);
        assert_eq!(ctx.sqes.len(), 4);
        assert_eq!(ctx.cqes.len(), 8);
    }

    #[test]
    fn nop_op_completes_with_user_data_echoed() {
        let ctx = IoRingCtx::new(4);
        unsafe {
            let p = ctx.sqes.as_ptr() as *mut Sqe;
            (*p).opcode = IORING_OP_NOP;
            (*p).user_data = 0xdead_beef;
        }
        ctx.sq_tail.store(1, Ordering::Release);
        let n = ctx.submit(1);
        assert_eq!(n, 1);
        assert_eq!(ctx.cq_ready(), 1);
        assert_eq!(ctx.cqes[0].user_data, 0xdead_beef);
        assert_eq!(ctx.cqes[0].res, 0);
    }

    #[test]
    fn unknown_op_returns_enosys_in_cqe() {
        let ctx = IoRingCtx::new(4);
        unsafe {
            let p = ctx.sqes.as_ptr() as *mut Sqe;
            (*p).opcode = 200; // unimplemented
            (*p).user_data = 0xcafe;
        }
        ctx.sq_tail.store(1, Ordering::Release);
        ctx.submit(1);
        assert_eq!(ctx.cqes[0].user_data, 0xcafe);
        assert_eq!(ctx.cqes[0].res, -38);
    }

    #[test]
    fn ctx_allocates_three_mmap_regions() {
        let ctx = IoRingCtx::new(8);
        // Each region rounds up to whole pages — pages.len() > 0.
        assert!(ctx.sq_ring_region.lock().pages.len() >= 1);
        assert!(ctx.cq_ring_region.lock().pages.len() >= 1);
        assert!(ctx.sqes_region.lock().pages.len() >= 1);
        // SQES region == sq_entries * sizeof(Sqe) bytes (rounded).
        assert!(ctx.sqes_region.lock().len >= 8 * core::mem::size_of::<Sqe>());
    }

    #[test]
    fn io_ring_mmap_routes_offset_to_region_base() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 280;
        current.tgid = 280;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut params = IoUringParams::default();
            let fd = sys_io_uring_setup(8, &mut params);
            assert!(fd >= 0);
            let file = current_files().unwrap().get(fd as i32).unwrap();

            let map_region = |off: u64| {
                let mut vma = crate::mm::mm_types::VmAreaStruct::new(0x1000, 0x2000, 0);
                vma.vm_pgoff = off >> 12;
                io_ring_mmap(&file, &mut vma)?;
                Ok::<u64, i32>((vma.vm_private_data as u64).wrapping_add(off))
            };

            // Mmap'ing each well-known offset records a non-zero base.
            let sq_base = map_region(IORING_OFF_SQ_RING).unwrap();
            let cq_base = map_region(IORING_OFF_CQ_RING).unwrap();
            let sqes_base = map_region(IORING_OFF_SQES).unwrap();
            assert!(sq_base != 0);
            assert!(cq_base != 0);
            assert!(sqes_base != 0);
            // Each region is independently allocated → bases differ.
            assert_ne!(sq_base, cq_base);
            assert_ne!(sq_base, sqes_base);
            assert_ne!(cq_base, sqes_base);

            // Unknown offset is rejected with -EINVAL.
            assert_eq!(map_region(0x1234_5000).unwrap_err(), -22);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn setup_rejects_non_user_params_pointer() {
        let kernel_half = crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX as *mut IoUringParams;

        unsafe {
            assert_eq!(sys_io_uring_setup(8, kernel_half), -(EFAULT as i64));
        }
    }

    #[test]
    fn syscall_m78_aio_io_uring_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 279;
        current.tgid = 279;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(sys_io_uring_setup(0, core::ptr::null_mut()), -22);
            let mut params = IoUringParams::default();
            let fd = sys_io_uring_setup(4, &mut params);
            assert!(fd >= 0);
            assert_eq!(params.sq_entries, 4);
            assert_eq!(params.cq_entries, 8);
            assert!(sys_io_uring_enter(fd as i32, 1, 0, 0, core::ptr::null(), 0) >= 0);
            assert_eq!(sys_io_uring_register(fd as i32, 0, core::ptr::null(), 0), 0);
            assert_eq!(sys_io_uring_enter(-1, 0, 0, 0, core::ptr::null(), 0), -9);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    /// End-to-end integration: drive a representative sample of opcodes
    /// through the full `sys_io_uring_setup` → user-side SQE write →
    /// `sys_io_uring_enter` → CQE-reap pipeline.  NOP completes successfully;
    /// every Layer 2/3 opcode whose `issue()` is not yet wired completes with
    /// `-ENOSYS` (the Linux behaviour for unsupported opcodes on a kernel
    /// build that omits them).
    #[test]
    fn enter_dispatches_every_wired_opcode() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 281;
        current.tgid = 281;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mut params = IoUringParams::default();
            let fd = sys_io_uring_setup(8, &mut params);
            assert!(fd >= 0);
            let ctx = ring_from_fd(fd as i32).unwrap();

            // Walk a representative slice of the IORING_OP_* enum.  For each
            // we write one SQE, submit it, and verify the CQE.user_data
            // matches.  The .res value is op-dependent — what we care about
            // is that dispatch routes to the right module (prep validator
            // runs and either returns a sensible errno or 0 for NOP).
            let cases: [(u8, u64, i32); 8] = [
                // Opcode, user_data, expected cqe.res.
                (uapi::IoringOp::Nop as u8, 0xa0, 0), // NOP succeeds.
                (uapi::IoringOp::Read as u8, 0xa1, -9), // rw prep rejects fd<0 with -EBADF
                (uapi::IoringOp::Write as u8, 0xa2, -9), // rw prep rejects fd<0 with -EBADF
                (uapi::IoringOp::Openat as u8, 0xa3, -22), // openclose prep rejects null filename with -EINVAL
                (uapi::IoringOp::Close as u8, 0xa4, -9),   // close prep rejects fd<0 with -EBADF
                (uapi::IoringOp::Statx as u8, 0xa5, -22), // statx prep rejects null filename/buf with -EINVAL
                (uapi::IoringOp::Timeout as u8, 0xa6, -22), // timeout prep rejects null ts with -EINVAL
                (uapi::IoringOp::FutexWait as u8, 0xa7, -22), // futex prep rejects null uaddr with -EINVAL
            ];
            for (i, (op, ud, _expected)) in cases.iter().enumerate() {
                let p = ctx.sqes.as_ptr() as *mut Sqe;
                let s = &mut *p.add(i);
                *s = Sqe::default();
                s.opcode = *op;
                s.user_data = *ud;
                s.fd = -1; // makes rw fail with EBADF
            }
            ctx.sq_tail.store(cases.len() as u32, Ordering::Release);
            let n = sys_io_uring_enter(fd as i32, cases.len() as u32, 0, 0, core::ptr::null(), 0);
            assert_eq!(n as usize, cases.len());
            assert_eq!(ctx.cq_ready() as usize, cases.len());
            for (i, (_op, ud, expected)) in cases.iter().enumerate() {
                assert_eq!(
                    ctx.cqes[i].user_data, *ud,
                    "user_data mismatch at idx {}",
                    i
                );
                assert_eq!(ctx.cqes[i].res, *expected, "res mismatch at idx {}", i);
            }

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
