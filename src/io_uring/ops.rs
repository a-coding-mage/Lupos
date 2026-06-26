//! linux-parity: partial
//! linux-source: vendor/linux/io_uring
//! `IORING_OP_*` dispatcher entry point.
//!
//! Thin shim over [`crate::io_uring::opdef::dispatch`] kept for callers that
//! still import `ops::dispatch`.  The actual op table lives in `opdef.rs`.
//!
//! Ref: vendor/linux/io_uring/io_uring.c::io_op_defs

use super::sqe::Sqe;

/// Dispatch a single SQE.  Returns the value to put in `cqe.res`.
pub fn dispatch(sqe: &Sqe) -> i32 {
    super::opdef::dispatch(sqe)
}
