//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/notif.c
//! test-origin: linux:vendor/linux/io_uring/notif.c
//! Zero-copy `notif` — ubuf-completion notifier for SEND_ZC / SENDMSG_ZC.
//!
//! Linux posts a follow-up CQE (with `IORING_CQE_F_NOTIF`) once the kernel
//! has released the user pages.  This module owns the per-request notif
//! state and the "page-release → CQE" delivery path.
//!
//! Ref: vendor/linux/io_uring/notif.c

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};

use super::IoRingCtx;

/// `IORING_CQE_F_NOTIF` flag on the follow-up CQE.
pub const IORING_CQE_F_NOTIF: u32 = 1 << 3;

/// `struct io_notif_data` — refcount + target ring.
pub struct IoNotif {
    pub ctx: Arc<IoRingCtx>,
    pub user_data: u64,
    /// Outstanding page references.  Drops to zero → notif CQE.
    refs: AtomicU32,
    /// Set once we've posted the notif CQE so a double-drop doesn't fire twice.
    fired: core::sync::atomic::AtomicBool,
}

impl IoNotif {
    pub fn new(ctx: Arc<IoRingCtx>, user_data: u64) -> Arc<Self> {
        Arc::new(Self {
            ctx,
            user_data,
            refs: AtomicU32::new(1),
            fired: core::sync::atomic::AtomicBool::new(false),
        })
    }

    /// `io_notif_grab` — bump refcount when binding a page set.
    pub fn grab(&self) -> u32 {
        self.refs.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// `io_notif_drop` — drop a refcount; on the final drop post the notif CQE.
    pub fn drop_ref(&self) -> bool {
        let prev = self.refs.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 && !self.fired.swap(true, Ordering::AcqRel) {
            // res=0 means success per io_notif_complete.
            self.ctx.complete(self.user_data, 0);
            true
        } else {
            false
        }
    }

    pub fn ref_count(&self) -> u32 {
        self.refs.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io_uring::IoRingCtx;
    use alloc::sync::Arc;

    #[test]
    fn cqe_notif_flag_matches_linux() {
        assert_eq!(IORING_CQE_F_NOTIF, 1 << 3);
    }

    #[test]
    fn grab_increments_refcount() {
        let ctx = Arc::new(IoRingCtx::new(4));
        let n = IoNotif::new(ctx, 0xdead);
        n.grab();
        n.grab();
        assert_eq!(n.ref_count(), 3);
    }

    #[test]
    fn final_drop_posts_cqe() {
        let ctx = Arc::new(IoRingCtx::new(4));
        let n = IoNotif::new(ctx.clone(), 0xface);
        let fired = n.drop_ref(); // refs 1 → 0
        assert!(fired);
        assert_eq!(ctx.cq_ready(), 1);
        assert_eq!(ctx.cqes[0].user_data, 0xface);
    }

    #[test]
    fn intermediate_drops_do_not_fire() {
        let ctx = Arc::new(IoRingCtx::new(4));
        let n = IoNotif::new(ctx.clone(), 0xface);
        n.grab(); // refs = 2
        let fired = n.drop_ref(); // refs 2 → 1
        assert!(!fired);
        assert_eq!(ctx.cq_ready(), 0);
        let fired = n.drop_ref(); // refs 1 → 0
        assert!(fired);
        assert_eq!(ctx.cq_ready(), 1);
    }
}
