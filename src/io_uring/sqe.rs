//! linux-parity: complete
//! linux-source: vendor/linux/io_uring
//! test-origin: linux:vendor/linux/io_uring
//! `struct io_uring_sqe` — Submission Queue Entry.
//! Byte-identical to vendor/linux/include/uapi/linux/io_uring.h::io_uring_sqe.
//! Total size: 64 bytes.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Sqe {
    pub opcode: u8,        // 0:    IORING_OP_*
    pub flags: u8,         // 1:    IOSQE_*
    pub ioprio: u16,       // 2:    ioprio for the request
    pub fd: i32,           // 4:    file descriptor to do IO on
    pub off: u64,          // 8:    offset into file (or addr2 for some ops)
    pub addr: u64,         // 16:   pointer to buffer or iovec
    pub len: u32,          // 24:   buffer size or number of iovecs
    pub op_flags: u32,     // 28:   per-op flags
    pub user_data: u64,    // 32:   data to be passed back at completion time
    pub buf_index: u16,    // 40:   buf_index/buf_group/personality
    pub personality: u16,  // 42
    pub splice_fd_in: i32, // 44
    pub addr3: u64,        // 48
    pub _pad2: [u64; 1],   // 56
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_64() {
        assert_eq!(core::mem::size_of::<Sqe>(), 64);
    }

    #[test]
    fn field_offsets_match_linux() {
        assert_eq!(core::mem::offset_of!(Sqe, opcode), 0);
        assert_eq!(core::mem::offset_of!(Sqe, flags), 1);
        assert_eq!(core::mem::offset_of!(Sqe, ioprio), 2);
        assert_eq!(core::mem::offset_of!(Sqe, fd), 4);
        assert_eq!(core::mem::offset_of!(Sqe, off), 8);
        assert_eq!(core::mem::offset_of!(Sqe, addr), 16);
        assert_eq!(core::mem::offset_of!(Sqe, len), 24);
        assert_eq!(core::mem::offset_of!(Sqe, op_flags), 28);
        assert_eq!(core::mem::offset_of!(Sqe, user_data), 32);
        assert_eq!(core::mem::offset_of!(Sqe, addr3), 48);
    }
}
