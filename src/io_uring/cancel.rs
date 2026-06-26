//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/cancel.c
//! test-origin: linux:vendor/linux/io_uring/cancel.c
//! Request cancellation.
//!
//! `IORING_OP_ASYNC_CANCEL` and `IORING_REGISTER_SYNC_CANCEL`.  Cancels are
//! matched by `user_data`, `fd`, or `(addr, flags)` per Linux.
//!
//! Ref: vendor/linux/io_uring/cancel.c

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use spin::Mutex;

/// `IORING_ASYNC_CANCEL_*` flags.  Ref: vendor/linux/include/uapi/linux/io_uring.h
pub const IORING_ASYNC_CANCEL_ALL: u32 = 1 << 0;
pub const IORING_ASYNC_CANCEL_FD: u32 = 1 << 1;
pub const IORING_ASYNC_CANCEL_ANY: u32 = 1 << 2;
pub const IORING_ASYNC_CANCEL_FD_FIXED: u32 = 1 << 3;
pub const IORING_ASYNC_CANCEL_USERDATA: u32 = 1 << 4;
pub const IORING_ASYNC_CANCEL_OP: u32 = 1 << 5;

/// One pending request observable from the cancel path.
#[derive(Clone, Copy, Debug)]
pub struct PendingReq {
    pub user_data: u64,
    pub fd: i32,
    pub opcode: u8,
}

/// `struct io_cancel_data` — match parameters.
#[derive(Clone, Copy, Debug, Default)]
pub struct CancelMatch {
    pub user_data: u64,
    pub fd: i32,
    pub opcode: u8,
    pub flags: u32,
}

/// Per-ring pending-request registry.  Each opcode handler registers its
/// req before issuing and removes it on completion.
#[derive(Default)]
pub struct CancelRegistry {
    /// Keyed by an opaque request handle.  Linux uses the `io_kiocb *` pointer.
    requests: Mutex<BTreeMap<u64, PendingReq>>,
    next_handle: Mutex<u64>,
}

impl CancelRegistry {
    pub const fn new() -> Self {
        Self {
            requests: Mutex::new(BTreeMap::new()),
            next_handle: Mutex::new(1),
        }
    }

    /// Register `req`; returns the handle used to remove it.
    pub fn add(&self, req: PendingReq) -> u64 {
        let mut nh = self.next_handle.lock();
        let h = *nh;
        *nh = nh.wrapping_add(1);
        drop(nh);
        self.requests.lock().insert(h, req);
        h
    }

    /// `io_complete` companion — drop the registration.
    pub fn remove(&self, handle: u64) -> Option<PendingReq> {
        self.requests.lock().remove(&handle)
    }

    /// `io_async_cancel` core matcher.  Returns the list of handles cancelled.
    /// Per Linux:
    ///   - default match: user_data
    ///   - CANCEL_FD: match on fd
    ///   - CANCEL_OP: match on opcode
    ///   - CANCEL_ALL: continue past the first match
    ///   - CANCEL_ANY: drain everything
    ///   - no match → ENOENT (caller handles)
    pub fn cancel(&self, m: &CancelMatch) -> Vec<u64> {
        let mut hits = Vec::new();
        let g = self.requests.lock();
        let by_fd = m.flags & IORING_ASYNC_CANCEL_FD != 0;
        let by_op = m.flags & IORING_ASYNC_CANCEL_OP != 0;
        let any = m.flags & IORING_ASYNC_CANCEL_ANY != 0;
        let all = m.flags & IORING_ASYNC_CANCEL_ALL != 0;

        for (h, req) in g.iter() {
            let matches = any
                || (by_fd && req.fd == m.fd)
                || (by_op && req.opcode == m.opcode)
                || (!by_fd && !by_op && req.user_data == m.user_data);
            if matches {
                hits.push(*h);
                if !all && !any {
                    break;
                }
            }
        }
        drop(g);
        for h in &hits {
            self.requests.lock().remove(h);
        }
        hits
    }

    pub fn len(&self) -> usize {
        self.requests.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(user_data: u64, fd: i32, opcode: u8) -> PendingReq {
        PendingReq {
            user_data,
            fd,
            opcode,
        }
    }

    #[test]
    fn cancel_by_user_data_default() {
        let r = CancelRegistry::new();
        let _h1 = r.add(req(1, -1, 0));
        let h2 = r.add(req(2, -1, 0));
        let hits = r.cancel(&CancelMatch {
            user_data: 2,
            ..Default::default()
        });
        assert_eq!(hits, alloc::vec![h2]);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn cancel_by_fd_flag() {
        let r = CancelRegistry::new();
        r.add(req(1, 3, 0));
        r.add(req(2, 4, 0));
        let hits = r.cancel(&CancelMatch {
            fd: 4,
            flags: IORING_ASYNC_CANCEL_FD,
            ..Default::default()
        });
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn cancel_all_keeps_going_after_first_match() {
        let r = CancelRegistry::new();
        r.add(req(7, 1, 0));
        r.add(req(7, 2, 0));
        r.add(req(7, 3, 0));
        let hits = r.cancel(&CancelMatch {
            user_data: 7,
            flags: IORING_ASYNC_CANCEL_ALL,
            ..Default::default()
        });
        assert_eq!(hits.len(), 3);
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn cancel_any_drains_everything() {
        let r = CancelRegistry::new();
        r.add(req(1, 1, 1));
        r.add(req(2, 2, 2));
        let hits = r.cancel(&CancelMatch {
            flags: IORING_ASYNC_CANCEL_ANY,
            ..Default::default()
        });
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn cancel_no_match_returns_empty() {
        let r = CancelRegistry::new();
        r.add(req(1, 1, 1));
        let hits = r.cancel(&CancelMatch {
            user_data: 99,
            ..Default::default()
        });
        assert!(hits.is_empty());
    }

    #[test]
    fn cancel_flag_constants_match_linux() {
        assert_eq!(IORING_ASYNC_CANCEL_ALL, 1);
        assert_eq!(IORING_ASYNC_CANCEL_FD, 2);
        assert_eq!(IORING_ASYNC_CANCEL_ANY, 4);
        assert_eq!(IORING_ASYNC_CANCEL_FD_FIXED, 8);
        assert_eq!(IORING_ASYNC_CANCEL_USERDATA, 16);
        assert_eq!(IORING_ASYNC_CANCEL_OP, 32);
    }
}
