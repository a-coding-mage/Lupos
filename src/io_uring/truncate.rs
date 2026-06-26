//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/truncate.c
//! test-origin: linux:vendor/linux/io_uring/truncate.c
//! `IORING_OP_FTRUNCATE`.
//!
//! Ref: vendor/linux/io_uring/truncate.c

use crate::include::uapi::errno::EINVAL;

use super::sqe::Sqe;

pub type IoReqFlags = u64;
pub type DoFtruncate = fn(fd: i32, len: u64, mode: u32) -> i32;

/// Ref: vendor/linux/include/uapi/linux/io_uring.h::IOSQE_ASYNC_BIT
pub const IOSQE_ASYNC_BIT: u32 = 4;
/// Ref: vendor/linux/include/linux/io_uring_types.h::REQ_F_FORCE_ASYNC
pub const REQ_F_FORCE_ASYNC: IoReqFlags = 1u64 << IOSQE_ASYNC_BIT;
/// Ref: vendor/linux/include/linux/io_uring_types.h::IO_URING_F_NONBLOCK
pub const IO_URING_F_NONBLOCK: u32 = 1u32 << 31;
/// Ref: vendor/linux/io_uring/io_uring.h::IOU_COMPLETE
pub const IOU_COMPLETE: i32 = 0;
/// Ref: vendor/linux/io_uring/truncate.c::do_ftruncate(..., 0)
pub const DO_FTRUNCATE_MODE: u32 = 0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IoFtruncate {
    /// Local SQE-level stand-in for Linux's resolved `req->file`.
    pub fd: i32,
    pub len: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IoFtruncatePrep {
    pub cmd: IoFtruncate,
    pub req_flags: IoReqFlags,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IoFtruncateIssue {
    pub fd: i32,
    pub len: u64,
    pub mode: u32,
    pub cqe_res: i32,
    pub cqe_flags: u32,
    pub return_code: i32,
    pub warned_nonblock: bool,
}

pub fn ftruncate_prep(sqe: &Sqe) -> Result<IoFtruncate, i32> {
    io_ftruncate_prep(sqe, 0).map(|prep| prep.cmd)
}

pub fn io_ftruncate_prep(sqe: &Sqe, req_flags: IoReqFlags) -> Result<IoFtruncatePrep, i32> {
    if sqe.op_flags != 0
        || sqe.addr != 0
        || sqe.len != 0
        || sqe.buf_index != 0
        || sqe.splice_fd_in != 0
        || sqe.addr3 != 0
    {
        return Err(-EINVAL);
    }

    Ok(IoFtruncatePrep {
        cmd: IoFtruncate {
            fd: sqe.fd,
            len: sqe.off,
        },
        req_flags: req_flags | REQ_F_FORCE_ASYNC,
    })
}

pub fn io_ftruncate(
    cmd: &IoFtruncate,
    issue_flags: u32,
    do_ftruncate: DoFtruncate,
) -> IoFtruncateIssue {
    let ret = do_ftruncate(cmd.fd, cmd.len, DO_FTRUNCATE_MODE);

    IoFtruncateIssue {
        fd: cmd.fd,
        len: cmd.len,
        mode: DO_FTRUNCATE_MODE,
        cqe_res: ret,
        cqe_flags: 0,
        return_code: IOU_COMPLETE,
        warned_nonblock: issue_flags & IO_URING_F_NONBLOCK != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX_TRUNCATE_C: &str = include_str!("../../vendor/linux/io_uring/truncate.c");
    const LINUX_IO_URING_H: &str = include_str!("../../vendor/linux/io_uring/io_uring.h");
    const LINUX_IO_URING_TYPES_H: &str =
        include_str!("../../vendor/linux/include/linux/io_uring_types.h");
    const LINUX_UAPI_IO_URING_H: &str =
        include_str!("../../vendor/linux/include/uapi/linux/io_uring.h");

    #[test]
    fn prep_takes_new_size_from_off_and_forces_async() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.off = 12_345;
        let r = io_ftruncate_prep(&s, 0x200).unwrap();
        assert_eq!(r.cmd.len, 12_345);
        assert_eq!(r.req_flags, 0x200 | REQ_F_FORCE_ASYNC);
    }

    #[test]
    fn prep_does_not_reject_fd_before_file_resolution() {
        let mut s = Sqe::default();
        s.fd = -1;
        s.off = 64;
        assert_eq!(ftruncate_prep(&s).unwrap(), IoFtruncate { fd: -1, len: 64 });
    }

    #[test]
    fn prep_rejects_linux_invalid_sqe_fields() {
        let invalid = [
            Sqe {
                op_flags: 1,
                ..Sqe::default()
            },
            Sqe {
                addr: 1,
                ..Sqe::default()
            },
            Sqe {
                len: 1,
                ..Sqe::default()
            },
            Sqe {
                buf_index: 1,
                ..Sqe::default()
            },
            Sqe {
                splice_fd_in: 1,
                ..Sqe::default()
            },
            Sqe {
                addr3: 1,
                ..Sqe::default()
            },
        ];

        for sqe in invalid {
            assert_eq!(io_ftruncate_prep(&sqe, 0).unwrap_err(), -EINVAL);
        }
    }

    #[test]
    fn issue_calls_do_ftruncate_and_completes() {
        fn do_ftruncate(fd: i32, len: u64, mode: u32) -> i32 {
            assert_eq!(fd, 7);
            assert_eq!(len, 4096);
            assert_eq!(mode, DO_FTRUNCATE_MODE);
            -EINVAL
        }

        let cmd = IoFtruncate { fd: 7, len: 4096 };
        let issue = io_ftruncate(&cmd, 0, do_ftruncate);
        assert_eq!(issue.cqe_res, -EINVAL);
        assert_eq!(issue.cqe_flags, 0);
        assert_eq!(issue.return_code, IOU_COMPLETE);
        assert!(!issue.warned_nonblock);
    }

    #[test]
    fn issue_records_nonblock_warning_condition() {
        fn do_ftruncate(_fd: i32, _len: u64, _mode: u32) -> i32 {
            0
        }

        let cmd = IoFtruncate { fd: 3, len: 1 };
        let issue = io_ftruncate(&cmd, IO_URING_F_NONBLOCK, do_ftruncate);
        assert!(issue.warned_nonblock);
    }

    #[test]
    fn linux_source_contract_is_present() {
        assert!(LINUX_TRUNCATE_C.contains("sqe->rw_flags || sqe->addr || sqe->len"));
        assert!(LINUX_TRUNCATE_C.contains("sqe->buf_index"));
        assert!(LINUX_TRUNCATE_C.contains("sqe->splice_fd_in"));
        assert!(LINUX_TRUNCATE_C.contains("sqe->addr3"));
        assert!(LINUX_TRUNCATE_C.contains("READ_ONCE(sqe->off)"));
        assert!(LINUX_TRUNCATE_C.contains("req->flags |= REQ_F_FORCE_ASYNC"));
        assert!(LINUX_TRUNCATE_C.contains("WARN_ON_ONCE(issue_flags & IO_URING_F_NONBLOCK)"));
        assert!(LINUX_TRUNCATE_C.contains("do_ftruncate(req->file, ft->len, 0)"));
        assert!(LINUX_TRUNCATE_C.contains("io_req_set_res(req, ret, 0)"));
        assert!(LINUX_TRUNCATE_C.contains("return IOU_COMPLETE"));
    }

    #[test]
    fn linux_constant_values_match_headers() {
        assert!(LINUX_UAPI_IO_URING_H.contains("IOSQE_ASYNC_BIT"));
        assert!(LINUX_IO_URING_TYPES_H.contains("REQ_F_FORCE_ASYNC_BIT\t= IOSQE_ASYNC_BIT"));
        assert!(LINUX_IO_URING_TYPES_H.contains("REQ_F_FORCE_ASYNC\t= IO_REQ_FLAG"));
        assert!(LINUX_IO_URING_TYPES_H.contains("IO_URING_F_NONBLOCK\t\t= INT_MIN"));
        assert!(LINUX_IO_URING_H.contains("IOU_COMPLETE\t\t= 0"));
        assert_eq!(IOSQE_ASYNC_BIT, 4);
        assert_eq!(REQ_F_FORCE_ASYNC, 16);
        assert_eq!(IO_URING_F_NONBLOCK, 0x8000_0000);
        assert_eq!(IOU_COMPLETE, 0);
    }
}
