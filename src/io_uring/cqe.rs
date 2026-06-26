//! linux-parity: complete
//! linux-source: vendor/linux/io_uring
//! test-origin: linux:vendor/linux/io_uring
//! `struct io_uring_cqe` — Completion Queue Entry.
//! Byte-identical to vendor/linux/include/uapi/linux/io_uring.h::io_uring_cqe.
//! Total size: 16 bytes.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Cqe {
    pub user_data: u64, // 0:  sqe.user_data submitted with the request
    pub res: i32,       // 8:  result code (negative errno on error)
    pub flags: u32,     // 12: IORING_CQE_F_*
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_16() {
        assert_eq!(core::mem::size_of::<Cqe>(), 16);
    }

    #[test]
    fn field_offsets_match_linux() {
        assert_eq!(core::mem::offset_of!(Cqe, user_data), 0);
        assert_eq!(core::mem::offset_of!(Cqe, res), 8);
        assert_eq!(core::mem::offset_of!(Cqe, flags), 12);
    }
}
