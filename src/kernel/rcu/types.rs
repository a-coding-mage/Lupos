//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu
//! test-origin: linux:vendor/linux/kernel/rcu
//! RCU core types — `struct rcu_head`.

/// Linux `struct rcu_head` — 16 bytes.
///
/// Embedded in any structure that wants to be reclaimed via `call_rcu`.
#[repr(C)]
pub struct RcuHead {
    pub next: *mut RcuHead,
    pub func: Option<unsafe extern "C" fn(*mut RcuHead)>,
}

const _: () = assert!(core::mem::size_of::<RcuHead>() == 16);

unsafe impl Send for RcuHead {}
unsafe impl Sync for RcuHead {}

impl RcuHead {
    pub const fn new() -> Self {
        Self {
            next: core::ptr::null_mut(),
            func: None,
        }
    }
}

/// Linux `init_rcu_head(head)`.
#[inline]
pub fn rcu_head_init(head: &mut RcuHead) {
    head.next = core::ptr::null_mut();
    head.func = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rcu_head_size_matches_linux() {
        assert_eq!(core::mem::size_of::<RcuHead>(), 16);
    }
}
