//! linux-parity: partial
//! linux-source: vendor/linux/io_uring/opdef.c
//! test-origin: linux:vendor/linux/io_uring/opdef.c
//! `io_op_def` table — per-opcode metadata and dispatch.
//!
//! Implements a subset of opdef.c's per-opcode metadata, capability flags, and
//! dispatch. Remaining work vs Linux for `complete`: the opcodes and `IOU_F_*`
//! flags not yet covered.
//!
//! Ref: vendor/linux/io_uring/opdef.c
//! Ref: vendor/linux/io_uring/opdef.h

use super::sqe::Sqe;
use super::uapi::IoringOp;

/// `IOU_F_OP_*` per-op capability flags.  Subset of Linux's flag set; values
/// match the upstream bit positions so a flag word stays drop-in compatible.
///
/// Ref: vendor/linux/io_uring/opdef.h
pub mod flag {
    /// Op needs a real file (sqe->fd valid).
    pub const NEEDS_FILE: u32 = 1 << 0;
    /// Op is non-blocking (does not need an io-wq worker).
    pub const UNBOUND_NONREG_FILE: u32 = 1 << 1;
    /// Op can run from poll context.
    pub const POLL_RING: u32 = 1 << 2;
    /// Op supports IORING_OP_BUFFER_SELECT.
    pub const BUFFER_SELECT: u32 = 1 << 3;
    /// Op needs the issue ring lock.
    pub const NEEDS_LOCKED: u32 = 1 << 4;
    /// Op consumes a uring_cmd.
    pub const URING_CMD: u32 = 1 << 5;
    /// Op is a multishot op (re-arms after completion).
    pub const MULTISHOT: u32 = 1 << 6;
}

/// `struct io_op_def` — metadata for one opcode.
/// Ref: vendor/linux/io_uring/opdef.h::io_op_def
pub struct IoOpDef {
    pub name: &'static str,
    pub flags: u32,
    /// `prep` — validate the SQE; returns 0 on success or a negative errno.
    /// `None` means the op is reserved or not yet ported.
    pub prep: Option<fn(&Sqe) -> i32>,
    /// `issue` — run the op; returns the value to record in `cqe.res`.
    /// `None` means the op completes with `-ENOSYS`.
    pub issue: Option<fn(&Sqe) -> i32>,
}

const NI: IoOpDef = IoOpDef {
    name: "<not-implemented>",
    flags: 0,
    prep: None,
    issue: None,
};

/// Build a constant table indexed by `IoringOp` u8 value.
///
/// Each entry routes to its per-op module's `prep` validator (Layer 2/3).
/// `issue` is wired only for ops with no external dependencies (NOP today);
/// Layer 2/3 ops complete with `-ENOSYS` once prep accepts the SQE.  Prep
/// rejects (`-EINVAL`, `-EBADF`, etc.) flow straight to the CQE.
pub static IO_OP_DEFS: [IoOpDef; IoringOp::COUNT] = build_op_defs();

const fn build_op_defs() -> [IoOpDef; IoringOp::COUNT] {
    let mut t: [IoOpDef; IoringOp::COUNT] = [NI; IoringOp::COUNT];
    t[IoringOp::Nop as usize] = IoOpDef {
        name: "NOP",
        flags: 0,
        prep: Some(nop_prep),
        issue: Some(nop_issue),
    };
    // Layer 2 per-op prep validators.
    t[IoringOp::Readv as usize] = entry("READV", readv_prep);
    t[IoringOp::Writev as usize] = entry("WRITEV", writev_prep);
    t[IoringOp::Fsync as usize] = entry("FSYNC", fsync_prep);
    t[IoringOp::ReadFixed as usize] = entry("READ_FIXED", read_fixed_prep);
    t[IoringOp::WriteFixed as usize] = entry("WRITE_FIXED", write_fixed_prep);
    t[IoringOp::PollAdd as usize] = entry("POLL_ADD", poll_add_prep);
    t[IoringOp::PollRemove as usize] = entry("POLL_REMOVE", poll_remove_prep);
    t[IoringOp::SyncFileRange as usize] = entry("SYNC_FILE_RANGE", sync_file_range_prep);
    t[IoringOp::Sendmsg as usize] = entry("SENDMSG", sendmsg_prep);
    t[IoringOp::Recvmsg as usize] = entry("RECVMSG", recvmsg_prep);
    t[IoringOp::Timeout as usize] = entry("TIMEOUT", timeout_prep);
    t[IoringOp::TimeoutRemove as usize] = entry("TIMEOUT_REMOVE", timeout_remove_prep);
    t[IoringOp::Accept as usize] = entry("ACCEPT", accept_prep);
    t[IoringOp::AsyncCancel as usize] = entry("ASYNC_CANCEL", async_cancel_prep);
    t[IoringOp::LinkTimeout as usize] = entry("LINK_TIMEOUT", link_timeout_prep);
    t[IoringOp::Connect as usize] = entry("CONNECT", connect_prep);
    t[IoringOp::Fallocate as usize] = entry("FALLOCATE", fallocate_prep);
    t[IoringOp::Openat as usize] = entry("OPENAT", openat_prep);
    t[IoringOp::Close as usize] = entry("CLOSE", close_prep);
    t[IoringOp::Statx as usize] = entry("STATX", statx_prep);
    t[IoringOp::Read as usize] = entry("READ", read_prep);
    t[IoringOp::Write as usize] = entry("WRITE", write_prep);
    t[IoringOp::Fadvise as usize] = entry("FADVISE", fadvise_prep);
    t[IoringOp::Madvise as usize] = entry("MADVISE", madvise_prep);
    t[IoringOp::Send as usize] = entry("SEND", send_prep);
    t[IoringOp::Recv as usize] = entry("RECV", recv_prep);
    t[IoringOp::Openat2 as usize] = entry("OPENAT2", openat2_prep);
    t[IoringOp::EpollCtl as usize] = entry("EPOLL_CTL", epoll_ctl_prep);
    t[IoringOp::Splice as usize] = entry("SPLICE", splice_prep);
    t[IoringOp::Tee as usize] = entry("TEE", tee_prep);
    t[IoringOp::Renameat as usize] = entry("RENAMEAT", renameat_prep);
    t[IoringOp::Unlinkat as usize] = entry("UNLINKAT", unlinkat_prep);
    t[IoringOp::Mkdirat as usize] = entry("MKDIRAT", mkdirat_prep);
    t[IoringOp::Symlinkat as usize] = entry("SYMLINKAT", symlinkat_prep);
    t[IoringOp::Linkat as usize] = entry("LINKAT", linkat_prep);
    t[IoringOp::MsgRing as usize] = entry("MSG_RING", msg_ring_prep);
    t[IoringOp::Fsetxattr as usize] = entry("FSETXATTR", fsetxattr_prep);
    t[IoringOp::Setxattr as usize] = entry("SETXATTR", setxattr_prep);
    t[IoringOp::Fgetxattr as usize] = entry("FGETXATTR", fgetxattr_prep);
    t[IoringOp::Getxattr as usize] = entry("GETXATTR", getxattr_prep);
    t[IoringOp::Socket as usize] = entry("SOCKET", socket_prep);
    t[IoringOp::UringCmd as usize] = entry("URING_CMD", uring_cmd_prep);
    t[IoringOp::SendZc as usize] = entry("SEND_ZC", send_zc_prep);
    t[IoringOp::SendmsgZc as usize] = entry("SENDMSG_ZC", sendmsg_zc_prep);
    t[IoringOp::Waitid as usize] = entry("WAITID", waitid_prep);
    t[IoringOp::FutexWait as usize] = entry("FUTEX_WAIT", futex_wait_prep);
    t[IoringOp::FutexWake as usize] = entry("FUTEX_WAKE", futex_wake_prep);
    t[IoringOp::FutexWaitv as usize] = entry("FUTEX_WAITV", futex_waitv_prep);
    t[IoringOp::Ftruncate as usize] = entry("FTRUNCATE", ftruncate_prep);
    t[IoringOp::Bind as usize] = entry("BIND", bind_prep);
    t[IoringOp::Listen as usize] = entry("LISTEN", listen_prep);
    t[IoringOp::Shutdown as usize] = entry("SHUTDOWN", shutdown_prep);
    t
}

const fn entry(name: &'static str, prep: fn(&Sqe) -> i32) -> IoOpDef {
    IoOpDef {
        name,
        flags: 0,
        prep: Some(prep),
        issue: None,
    }
}

fn nop_prep(sqe: &Sqe) -> i32 {
    match super::nop::io_nop_prep(sqe) {
        Ok(_) => 0,
        Err(e) => e,
    }
}

fn nop_issue(sqe: &Sqe) -> i32 {
    match super::nop::io_nop_prep(sqe) {
        Ok(nop) => super::nop::io_nop_issue(&nop),
        Err(e) => e,
    }
}

// Layer 2 adapter functions — each converts a Result<_, i32> into the
// flat `i32` shape `IoOpDef::prep` expects.

macro_rules! prep_adapter {
    ($name:ident, $module:ident :: $func:ident) => {
        fn $name(sqe: &Sqe) -> i32 {
            match super::$module::$func(sqe) {
                Ok(_) => 0,
                Err(e) => e,
            }
        }
    };
}

prep_adapter!(read_prep, rw::read_prep);
prep_adapter!(write_prep, rw::write_prep);
prep_adapter!(readv_prep, rw::readv_prep);
prep_adapter!(writev_prep, rw::writev_prep);
prep_adapter!(read_fixed_prep, rw::read_fixed_prep);
prep_adapter!(write_fixed_prep, rw::write_fixed_prep);

prep_adapter!(openat_prep, openclose::openat_prep);
prep_adapter!(openat2_prep, openclose::openat2_prep);

fn close_prep(sqe: &Sqe) -> i32 {
    match super::openclose::close_prep(sqe) {
        Ok(_) => 0,
        Err(e) => e,
    }
}

prep_adapter!(fsync_prep, sync::fsync_prep);
prep_adapter!(sync_file_range_prep, sync::sync_file_range_prep);
prep_adapter!(fallocate_prep, sync::fallocate_prep);

prep_adapter!(renameat_prep, fs::renameat_prep);
prep_adapter!(unlinkat_prep, fs::unlinkat_prep);
prep_adapter!(mkdirat_prep, fs::mkdirat_prep);
prep_adapter!(symlinkat_prep, fs::symlinkat_prep);
prep_adapter!(linkat_prep, fs::linkat_prep);

prep_adapter!(splice_prep, splice::splice_prep);
prep_adapter!(tee_prep, splice::tee_prep);

prep_adapter!(statx_prep, statx::statx_prep);
prep_adapter!(fgetxattr_prep, xattr::fgetxattr_prep);
prep_adapter!(fsetxattr_prep, xattr::fsetxattr_prep);
prep_adapter!(getxattr_prep, xattr::getxattr_prep);
prep_adapter!(setxattr_prep, xattr::setxattr_prep);

prep_adapter!(ftruncate_prep, truncate::ftruncate_prep);
prep_adapter!(fadvise_prep, advise::fadvise_prep);
prep_adapter!(madvise_prep, advise::madvise_prep);

prep_adapter!(epoll_ctl_prep, epoll::epoll_ctl_prep);

prep_adapter!(futex_wait_prep, futex::futex_wait_prep);
prep_adapter!(futex_wake_prep, futex::futex_wake_prep);
prep_adapter!(futex_waitv_prep, futex::futex_waitv_prep);

prep_adapter!(msg_ring_prep, msg_ring::msg_ring_prep);

prep_adapter!(timeout_prep, timeout::timeout_prep);

fn timeout_remove_prep(sqe: &Sqe) -> i32 {
    match super::timeout::timeout_remove_prep(sqe) {
        Ok(_) => 0,
        Err(e) => e,
    }
}

prep_adapter!(link_timeout_prep, timeout::link_timeout_prep);

prep_adapter!(waitid_prep, waitid::waitid_prep);

prep_adapter!(send_prep, net::send_prep);
prep_adapter!(recv_prep, net::recv_prep);
prep_adapter!(sendmsg_prep, net::sendmsg_prep);
prep_adapter!(recvmsg_prep, net::recvmsg_prep);
prep_adapter!(send_zc_prep, net::send_zc_prep);
prep_adapter!(sendmsg_zc_prep, net::sendmsg_zc_prep);
prep_adapter!(accept_prep, net::accept_prep);
prep_adapter!(connect_prep, net::connect_prep);
prep_adapter!(socket_prep, net::socket_prep);
prep_adapter!(bind_prep, net::connect_prep); // BIND prep is structurally identical (struct sockaddr *).

fn shutdown_prep(sqe: &Sqe) -> i32 {
    match super::net::shutdown_prep(sqe) {
        Ok(_) => 0,
        Err(e) => e,
    }
}

fn listen_prep(sqe: &Sqe) -> i32 {
    match super::net::listen_prep(sqe) {
        Ok(_) => 0,
        Err(e) => e,
    }
}

prep_adapter!(uring_cmd_prep, uring_cmd::uring_cmd_prep);

// Poll & cancel — Layer 1 modules expose typed types; we accept the SQE if
// its required fields are present.  Real arming happens during issue.
fn poll_add_prep(sqe: &Sqe) -> i32 {
    if sqe.fd < 0 {
        return -9;
    }
    0
}

fn poll_remove_prep(_sqe: &Sqe) -> i32 {
    // sqe.addr carries the user_data of the poll to remove — any value (incl 0).
    0
}

fn async_cancel_prep(_sqe: &Sqe) -> i32 {
    // sqe.addr is the user_data to cancel (may be 0 when CANCEL_ANY is set).
    0
}

/// Dispatch wrapper used by [`crate::io_uring::ops::dispatch`].
pub fn dispatch(sqe: &Sqe) -> i32 {
    let Some(op) = IoringOp::from_u8(sqe.opcode) else {
        return -38; // -ENOSYS — unknown opcode
    };
    let def = &IO_OP_DEFS[op as usize];
    let Some(prep) = def.prep else { return -38 };
    let r = prep(sqe);
    if r < 0 {
        return r;
    }
    match def.issue {
        Some(issue) => issue(sqe),
        None => -38,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_one_entry_per_opcode() {
        assert_eq!(IO_OP_DEFS.len(), 65);
    }

    #[test]
    fn layer2_entries_are_wired() {
        // Each Layer 2 op must have a prep slot populated.
        for op in [
            IoringOp::Read,
            IoringOp::Write,
            IoringOp::Openat,
            IoringOp::Close,
            IoringOp::Statx,
            IoringOp::Timeout,
            IoringOp::FutexWait,
            IoringOp::Send,
            IoringOp::Recv,
            IoringOp::Accept,
            IoringOp::Connect,
            IoringOp::MsgRing,
        ] {
            assert!(
                IO_OP_DEFS[op as usize].prep.is_some(),
                "missing prep slot for {:?}",
                op
            );
        }
    }

    #[test]
    fn nop_entry_is_wired() {
        let def = &IO_OP_DEFS[IoringOp::Nop as usize];
        assert_eq!(def.name, "NOP");
        assert!(def.prep.is_some());
        assert!(def.issue.is_some());
    }

    #[test]
    fn unknown_opcode_returns_enosys() {
        let mut s = Sqe::default();
        s.opcode = 250;
        assert_eq!(dispatch(&s), -38);
    }

    #[test]
    fn nop_dispatches_to_zero() {
        let s = Sqe::default();
        assert_eq!(dispatch(&s), 0);
    }

    #[test]
    fn nop_inject_result_returns_len() {
        let mut s = Sqe::default();
        s.opcode = 0;
        s.op_flags = super::super::nop::IORING_NOP_INJECT_RESULT;
        s.len = 123;
        assert_eq!(dispatch(&s), 123);
    }
}
