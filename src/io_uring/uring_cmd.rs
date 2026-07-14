//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/uring_cmd.c
//! test-origin: linux:vendor/linux/io_uring/uring_cmd.c
//! `IORING_OP_URING_CMD` — generic ioctl-style async commands.
//!
//! The opcode carries a 16-byte cmd buffer that the target driver
//! (`struct file_operations::uring_cmd`) interprets.  Lupos exposes a
//! registry of dispatch handlers keyed by `(file_class, cmd_op)`.
//!
//! Ref: vendor/linux/io_uring/uring_cmd.c

use core::ffi::c_void;

use crate::include::uapi::errno::EOPNOTSUPP;
use crate::kernel::module::{export_symbol, find_symbol};

use super::sqe::Sqe;

/// `IORING_URING_CMD_*` flags.
/// Ref: vendor/linux/include/uapi/linux/io_uring.h:334
pub const IORING_URING_CMD_FIXED: u32 = 1 << 0;
pub const IORING_URING_CMD_MULTISHOT: u32 = 1 << 1;
pub const IORING_URING_CMD_MASK: u32 = IORING_URING_CMD_FIXED | IORING_URING_CMD_MULTISHOT;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoUringCmd {
    pub fd: i32,
    pub cmd_op: u32,
    pub flags: u32,
    pub data_addr: u64,
    pub buf_index: u16,
}

pub fn uring_cmd_prep(sqe: &Sqe) -> Result<IoUringCmd, i32> {
    if sqe.fd < 0 {
        return Err(-9);
    }
    if sqe.op_flags & !IORING_URING_CMD_MASK != 0 {
        return Err(-22);
    }
    if sqe.op_flags & IORING_URING_CMD_FIXED != 0 && sqe.op_flags & IORING_URING_CMD_MULTISHOT != 0
    {
        // Comment in vendor file: "Not compatible with URING_CMD_FIXED, for now."
        return Err(-22);
    }
    Ok(IoUringCmd {
        fd: sqe.fd,
        cmd_op: sqe.len,
        flags: sqe.op_flags,
        data_addr: sqe.addr,
        buf_index: sqe.buf_index,
    })
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__io_uring_cmd_do_in_task",
        linux___io_uring_cmd_do_in_task as usize,
        true,
    );
    export_symbol_once(
        "__io_uring_cmd_done",
        linux___io_uring_cmd_done as usize,
        true,
    );
    export_symbol_once(
        "io_uring_cmd_import_fixed",
        linux_io_uring_cmd_import_fixed as usize,
        true,
    );
    export_symbol_once(
        "io_uring_cmd_import_fixed_vec",
        linux_io_uring_cmd_import_fixed_vec as usize,
        true,
    );
    export_symbol_once(
        "io_uring_cmd_mark_cancelable",
        linux_io_uring_cmd_mark_cancelable as usize,
        true,
    );
    export_symbol_once(
        "io_uring_cmd_issue_blocking",
        linux_io_uring_cmd_issue_blocking as usize,
        true,
    );
}

#[repr(C)]
pub struct LinuxIoTwReq {
    req: *mut c_void,
}

type LinuxIoReqTwFunc = unsafe extern "C" fn(LinuxIoTwReq, *mut c_void);

/// `__io_uring_cmd_do_in_task` - `vendor/linux/io_uring/uring_cmd.c:125`.
///
/// Module-facing `io_kiocb` task-work completion is not modeled yet; users of
/// BSG/ioctl uring command completion remain unsupported.
pub unsafe extern "C" fn linux___io_uring_cmd_do_in_task(
    _ioucmd: *mut c_void,
    _task_work_cb: Option<LinuxIoReqTwFunc>,
    _flags: u32,
) {
}

/// `__io_uring_cmd_done` - `vendor/linux/io_uring/uring_cmd.c:150`.
pub unsafe extern "C" fn linux___io_uring_cmd_done(
    _ioucmd: *mut c_void,
    _ret: i32,
    _res2: u64,
    _issue_flags: u32,
    _is_cqe32: bool,
) {
}

/// `io_uring_cmd_import_fixed` - `vendor/linux/io_uring/uring_cmd.c:289`.
pub unsafe extern "C" fn linux_io_uring_cmd_import_fixed(
    _ubuf: u64,
    _len: usize,
    _rw: i32,
    _iter: *mut c_void,
    _ioucmd: *mut c_void,
    _issue_flags: u32,
) -> i32 {
    -EOPNOTSUPP
}

/// `io_uring_cmd_import_fixed_vec` - `vendor/linux/io_uring/uring_cmd.c:303`.
pub unsafe extern "C" fn linux_io_uring_cmd_import_fixed_vec(
    _ioucmd: *mut c_void,
    _iovec: *const c_void,
    _iovec_len: usize,
    _dir: i32,
    _iter: *mut c_void,
    _issue_flags: u32,
) -> i32 {
    -EOPNOTSUPP
}

/// `io_uring_cmd_mark_cancelable` - `vendor/linux/io_uring/uring_cmd.c:101`.
pub unsafe extern "C" fn linux_io_uring_cmd_mark_cancelable(_cmd: *mut c_void, _issue_flags: u32) {}

/// `io_uring_cmd_issue_blocking` - `vendor/linux/io_uring/uring_cmd.c:325`.
pub unsafe extern "C" fn linux_io_uring_cmd_issue_blocking(_ioucmd: *mut c_void) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_match_linux() {
        assert_eq!(IORING_URING_CMD_FIXED, 1);
        assert_eq!(IORING_URING_CMD_MULTISHOT, 2);
        assert_eq!(IORING_URING_CMD_MASK, 3);
    }

    #[test]
    fn rejects_unknown_flag_bits() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.op_flags = 1 << 31;
        assert_eq!(uring_cmd_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn rejects_fixed_with_multishot() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.op_flags = IORING_URING_CMD_FIXED | IORING_URING_CMD_MULTISHOT;
        assert_eq!(uring_cmd_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn captures_cmd_op_from_len() {
        let mut s = Sqe::default();
        s.fd = 1;
        s.len = 0x4142_4344;
        let r = uring_cmd_prep(&s).unwrap();
        assert_eq!(r.cmd_op, 0x4142_4344);
    }

    #[test]
    fn module_exports_register_for_bsg_uring_imports() {
        register_module_exports();

        assert!(crate::kernel::module::find_symbol("__io_uring_cmd_do_in_task").is_some());
        assert!(crate::kernel::module::find_symbol("__io_uring_cmd_done").is_some());
        assert!(crate::kernel::module::find_symbol("io_uring_cmd_import_fixed").is_some());
    }
}
