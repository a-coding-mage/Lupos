//! linux-parity: complete
//! linux-source: vendor/linux/io_uring
//! test-origin: linux:vendor/linux/io_uring
//! `struct io_uring_params` and offset structs.
//! Byte-identical to vendor/linux/include/uapi/linux/io_uring.h.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoSqRingOffsets {
    pub head: u32,
    pub tail: u32,
    pub ring_mask: u32,
    pub ring_entries: u32,
    pub flags: u32,
    pub dropped: u32,
    pub array: u32,
    pub resv1: u32,
    pub user_addr: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoCqRingOffsets {
    pub head: u32,
    pub tail: u32,
    pub ring_mask: u32,
    pub ring_entries: u32,
    pub overflow: u32,
    pub cqes: u32,
    pub flags: u32,
    pub resv1: u32,
    pub user_addr: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringParams {
    pub sq_entries: u32,
    pub cq_entries: u32,
    pub flags: u32,
    pub sq_thread_cpu: u32,
    pub sq_thread_idle: u32,
    pub features: u32,
    pub wq_fd: u32,
    pub resv: [u32; 3],
    pub sq_off: IoSqRingOffsets,
    pub cq_off: IoCqRingOffsets,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sq_offsets_size_40() {
        assert_eq!(core::mem::size_of::<IoSqRingOffsets>(), 40);
    }

    #[test]
    fn cq_offsets_size_40() {
        assert_eq!(core::mem::size_of::<IoCqRingOffsets>(), 40);
    }

    #[test]
    fn params_size_120() {
        // Linux: sizeof(struct io_uring_params) == 120
        assert_eq!(core::mem::size_of::<IoUringParams>(), 120);
    }
}
