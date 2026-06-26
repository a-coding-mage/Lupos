//! linux-parity: complete
//! linux-source: vendor/linux/io_uring
//! test-origin: linux:vendor/linux/io_uring
//! io_uring UAPI structs beyond `sqe`/`cqe`/`io_uring_params`.
//!
//! Byte-identical to `vendor/linux/include/uapi/linux/io_uring.h`.  Each struct
//! gets a compile-time `size_of` assert against the Linux number so layout
//! drift surfaces at build time.
//!
//! Ref: vendor/linux/include/uapi/linux/io_uring.h

#![allow(non_camel_case_types)]

/// `IORING_OP_*` enum (UAPI).  Order matches `vendor/linux/include/uapi/linux/io_uring.h::io_uring_op`.
/// IORING_OP_LAST is intentionally one past the last real opcode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IoringOp {
    Nop = 0,
    Readv = 1,
    Writev = 2,
    Fsync = 3,
    ReadFixed = 4,
    WriteFixed = 5,
    PollAdd = 6,
    PollRemove = 7,
    SyncFileRange = 8,
    Sendmsg = 9,
    Recvmsg = 10,
    Timeout = 11,
    TimeoutRemove = 12,
    Accept = 13,
    AsyncCancel = 14,
    LinkTimeout = 15,
    Connect = 16,
    Fallocate = 17,
    Openat = 18,
    Close = 19,
    FilesUpdate = 20,
    Statx = 21,
    Read = 22,
    Write = 23,
    Fadvise = 24,
    Madvise = 25,
    Send = 26,
    Recv = 27,
    Openat2 = 28,
    EpollCtl = 29,
    Splice = 30,
    ProvideBuffers = 31,
    RemoveBuffers = 32,
    Tee = 33,
    Shutdown = 34,
    Renameat = 35,
    Unlinkat = 36,
    Mkdirat = 37,
    Symlinkat = 38,
    Linkat = 39,
    MsgRing = 40,
    Fsetxattr = 41,
    Setxattr = 42,
    Fgetxattr = 43,
    Getxattr = 44,
    Socket = 45,
    UringCmd = 46,
    SendZc = 47,
    SendmsgZc = 48,
    ReadMultishot = 49,
    Waitid = 50,
    FutexWait = 51,
    FutexWake = 52,
    FutexWaitv = 53,
    FixedFdInstall = 54,
    Ftruncate = 55,
    Bind = 56,
    Listen = 57,
    RecvZc = 58,
    EpollWait = 59,
    ReadvFixed = 60,
    WritevFixed = 61,
    Pipe = 62,
    Nop128 = 63,
    UringCmd128 = 64,
    Last = 65,
}

impl IoringOp {
    pub const COUNT: usize = IoringOp::Last as usize;

    /// `IO_URING_OP_SUPPORTED` flag returned in `io_uring_probe_op::flags`.
    pub const SUPPORTED_FLAG: u16 = 1 << 0;

    pub const fn from_u8(v: u8) -> Option<Self> {
        if v < IoringOp::Last as u8 {
            Some(unsafe { core::mem::transmute(v) })
        } else {
            None
        }
    }
}

/// `IORING_REGISTER_*` opcodes (UAPI).
/// Ref: vendor/linux/include/uapi/linux/io_uring.h::io_uring_register_op
pub mod register_op {
    pub const REGISTER_BUFFERS: u32 = 0;
    pub const UNREGISTER_BUFFERS: u32 = 1;
    pub const REGISTER_FILES: u32 = 2;
    pub const UNREGISTER_FILES: u32 = 3;
    pub const REGISTER_EVENTFD: u32 = 4;
    pub const UNREGISTER_EVENTFD: u32 = 5;
    pub const REGISTER_FILES_UPDATE: u32 = 6;
    pub const REGISTER_EVENTFD_ASYNC: u32 = 7;
    pub const REGISTER_PROBE: u32 = 8;
    pub const REGISTER_PERSONALITY: u32 = 9;
    pub const UNREGISTER_PERSONALITY: u32 = 10;
    pub const REGISTER_RESTRICTIONS: u32 = 11;
    pub const REGISTER_ENABLE_RINGS: u32 = 12;
    pub const REGISTER_FILES2: u32 = 13;
    pub const REGISTER_FILES_UPDATE2: u32 = 14;
    pub const REGISTER_BUFFERS2: u32 = 15;
    pub const REGISTER_BUFFERS_UPDATE: u32 = 16;
    pub const REGISTER_IOWQ_AFF: u32 = 17;
    pub const UNREGISTER_IOWQ_AFF: u32 = 18;
    pub const REGISTER_IOWQ_MAX_WORKERS: u32 = 19;
    pub const REGISTER_RING_FDS: u32 = 20;
    pub const UNREGISTER_RING_FDS: u32 = 21;
    pub const REGISTER_PBUF_RING: u32 = 22;
    pub const UNREGISTER_PBUF_RING: u32 = 23;
    pub const REGISTER_SYNC_CANCEL: u32 = 24;
    pub const REGISTER_FILE_ALLOC_RANGE: u32 = 25;
    pub const REGISTER_PBUF_STATUS: u32 = 26;
    pub const REGISTER_NAPI: u32 = 27;
    pub const UNREGISTER_NAPI: u32 = 28;
    pub const REGISTER_CLOCK: u32 = 29;
    pub const REGISTER_CLONE_BUFFERS: u32 = 30;
    pub const REGISTER_SEND_MSG_RING: u32 = 31;
    pub const REGISTER_ZCRX_IFQ: u32 = 32;
    pub const REGISTER_RESIZE_RINGS: u32 = 33;
    pub const REGISTER_MEM_REGION: u32 = 34;
    pub const REGISTER_QUERY: u32 = 35;
    pub const REGISTER_ZCRX_CTRL: u32 = 36;
    pub const REGISTER_BPF_FILTER: u32 = 37;
    pub const REGISTER_LAST: u32 = 38;
    pub const REGISTER_USE_REGISTERED_RING: u32 = 1u32 << 31;
}

/// `io_uring_params->features` flags.
pub mod feat {
    pub const SINGLE_MMAP: u32 = 1 << 0;
    pub const NODROP: u32 = 1 << 1;
    pub const SUBMIT_STABLE: u32 = 1 << 2;
    pub const RW_CUR_POS: u32 = 1 << 3;
    pub const CUR_PERSONALITY: u32 = 1 << 4;
    pub const FAST_POLL: u32 = 1 << 5;
    pub const POLL_32BITS: u32 = 1 << 6;
    pub const SQPOLL_NONFIXED: u32 = 1 << 7;
    pub const EXT_ARG: u32 = 1 << 8;
    pub const NATIVE_WORKERS: u32 = 1 << 9;
    pub const RSRC_TAGS: u32 = 1 << 10;
    pub const CQE_SKIP: u32 = 1 << 11;
    pub const LINKED_FILE: u32 = 1 << 12;
    pub const REG_REG_RING: u32 = 1 << 13;
    pub const RECVSEND_BUNDLE: u32 = 1 << 14;
    pub const MIN_TIMEOUT: u32 = 1 << 15;
    pub const RW_ATTR: u32 = 1 << 16;
    pub const NO_IOWAIT: u32 = 1 << 17;
}

/// `IORING_OFF_*` mmap offsets.  Re-exported from `super::IORING_OFF_*` for
/// callers that import this module directly.
pub const IORING_OFF_SQ_RING: u64 = 0;
pub const IORING_OFF_CQ_RING: u64 = 0x800_0000;
pub const IORING_OFF_SQES: u64 = 0x1000_0000;
pub const IORING_OFF_PBUF_RING: u64 = 0x8000_0000;
pub const IORING_OFF_PBUF_SHIFT: u64 = 16;

/// `struct io_uring_files_update` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:740
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringFilesUpdate {
    pub offset: u32,
    pub resv: u32,
    pub fds: u64,
}
const _: () = assert!(core::mem::size_of::<IoUringFilesUpdate>() == 16);

/// `struct io_uring_region_desc` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:751
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringRegionDesc {
    pub user_addr: u64,
    pub size: u64,
    pub flags: u32,
    pub id: u32,
    pub mmap_offset: u64,
    pub __resv: [u64; 4],
}
const _: () = assert!(core::mem::size_of::<IoUringRegionDesc>() == 64);

/// `struct io_uring_mem_region_reg` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:765
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringMemRegionReg {
    pub region_uptr: u64,
    pub flags: u64,
    pub __resv: [u64; 2],
}
const _: () = assert!(core::mem::size_of::<IoUringMemRegionReg>() == 32);

/// `struct io_uring_rsrc_register` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:777
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringRsrcRegister {
    pub nr: u32,
    pub flags: u32,
    pub resv2: u64,
    pub data: u64,
    pub tags: u64,
}
const _: () = assert!(core::mem::size_of::<IoUringRsrcRegister>() == 32);

/// `struct io_uring_rsrc_update` — UAPI (deprecated, see `update2`).
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:785
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringRsrcUpdate {
    pub offset: u32,
    pub resv: u32,
    pub data: u64,
}
const _: () = assert!(core::mem::size_of::<IoUringRsrcUpdate>() == 16);

/// `struct io_uring_rsrc_update2` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:791
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringRsrcUpdate2 {
    pub offset: u32,
    pub resv: u32,
    pub data: u64,
    pub tags: u64,
    pub nr: u32,
    pub resv2: u32,
}
const _: () = assert!(core::mem::size_of::<IoUringRsrcUpdate2>() == 32);

/// `struct io_uring_probe_op` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:805
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringProbeOp {
    pub op: u8,
    pub resv: u8,
    pub flags: u16,
    pub resv2: u32,
}
const _: () = assert!(core::mem::size_of::<IoUringProbeOp>() == 8);

/// `struct io_uring_probe` (header only — `ops[]` is a trailing flex array).
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:812
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringProbeHdr {
    pub last_op: u8,
    pub ops_len: u8,
    pub resv: u16,
    pub resv2: [u32; 3],
}
const _: () = assert!(core::mem::size_of::<IoUringProbeHdr>() == 16);

/// `struct io_uring_restriction` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:820
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringRestriction {
    pub opcode: u16,
    /// `register_op` / `sqe_op` / `sqe_flags` (one-byte union).
    pub op_byte: u8,
    pub resv: u8,
    pub resv2: [u32; 3],
}
const _: () = assert!(core::mem::size_of::<IoUringRestriction>() == 16);

/// `struct io_uring_clock_register` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:838
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringClockRegister {
    pub clockid: u32,
    pub __resv: [u32; 3],
}
const _: () = assert!(core::mem::size_of::<IoUringClockRegister>() == 16);

/// `struct io_uring_clone_buffers` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:848
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringCloneBuffers {
    pub src_fd: u32,
    pub flags: u32,
    pub src_off: u32,
    pub dst_off: u32,
    pub nr: u32,
    pub pad: [u32; 3],
}
const _: () = assert!(core::mem::size_of::<IoUringCloneBuffers>() == 32);

/// `struct io_uring_buf` — UAPI provided-buffer entry.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:857
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringBuf {
    pub addr: u64,
    pub len: u32,
    pub bid: u16,
    pub resv: u16,
}
const _: () = assert!(core::mem::size_of::<IoUringBuf>() == 16);

/// `struct io_uring_buf_reg` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:903
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringBufReg {
    pub ring_addr: u64,
    pub ring_entries: u32,
    pub bgid: u16,
    pub flags: u16,
    pub resv: [u64; 3],
}
const _: () = assert!(core::mem::size_of::<IoUringBufReg>() == 40);

/// `struct io_uring_buf_status` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:912
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringBufStatus {
    pub buf_group: u32,
    pub head: u32,
    pub resv: [u32; 8],
}
const _: () = assert!(core::mem::size_of::<IoUringBufStatus>() == 40);

/// `struct io_uring_napi` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:935
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringNapi {
    pub busy_poll_to: u32,
    pub prefer_busy_poll: u8,
    pub opcode: u8,
    pub pad: [u8; 2],
    pub op_param: u32,
    pub resv: u32,
}
const _: () = assert!(core::mem::size_of::<IoUringNapi>() == 16);

/// `struct io_uring_getevents_arg` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:996
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringGeteventsArg {
    pub sigmask: u64,
    pub sigmask_sz: u32,
    pub min_wait_usec: u32,
    pub ts: u64,
}
const _: () = assert!(core::mem::size_of::<IoUringGeteventsArg>() == 24);

/// `struct io_uring_sync_cancel_reg` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:1006
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringSyncCancelReg {
    pub addr: u64,
    pub fd: i32,
    pub flags: u32,
    /// `__kernel_timespec` — two i64s.
    pub timeout_sec: i64,
    pub timeout_nsec: i64,
    pub opcode: u8,
    pub pad: [u8; 7],
    pub pad2: [u64; 3],
}
const _: () = assert!(core::mem::size_of::<IoUringSyncCancelReg>() == 64);

/// `struct io_uring_file_index_range` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:1020
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringFileIndexRange {
    pub off: u32,
    pub len: u32,
    pub resv: u64,
}
const _: () = assert!(core::mem::size_of::<IoUringFileIndexRange>() == 16);

/// `struct io_uring_recvmsg_out` — UAPI.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:1026
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringRecvmsgOut {
    pub namelen: u32,
    pub controllen: u32,
    pub payloadlen: u32,
    pub flags: u32,
}
const _: () = assert!(core::mem::size_of::<IoUringRecvmsgOut>() == 16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ioring_op_count_is_65() {
        // Linux io_uring_op enum has IORING_OP_LAST == 65 in 6.x.
        assert_eq!(IoringOp::COUNT, 65);
    }

    #[test]
    fn ioring_op_from_u8_round_trips() {
        for v in 0..(IoringOp::Last as u8) {
            assert_eq!(IoringOp::from_u8(v).map(|o| o as u8), Some(v));
        }
        assert_eq!(IoringOp::from_u8(IoringOp::Last as u8), None);
    }

    #[test]
    fn register_op_last_is_38() {
        // Mirrors `IORING_REGISTER_LAST` in vendor/linux/include/uapi/linux/io_uring.h.
        assert_eq!(register_op::REGISTER_LAST, 38);
    }
}
