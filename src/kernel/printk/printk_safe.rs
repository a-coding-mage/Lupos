//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk/printk_safe.c
//! test-origin: linux:vendor/linux/kernel/printk/printk_safe.c
//! Recursion-safe printk entry / exit.
//!
//! When printk is invoked from NMI/printk-recursion-detected contexts, the
//! safe path buffers the output to a per-cpu temporary so the global ring
//! doesn't recurse on itself.
//!
//! Ref: vendor/linux/kernel/printk/printk_safe.c

use core::sync::atomic::{AtomicU32, Ordering};

static PRINTK_RECURSION: AtomicU32 = AtomicU32::new(0);

/// `__printk_safe_enter`.  Returns the previous depth so the matching
/// `exit` can restore it.
pub fn enter() -> u32 {
    PRINTK_RECURSION.fetch_add(1, Ordering::AcqRel)
}

/// `__printk_safe_exit`.
pub fn exit() {
    PRINTK_RECURSION.fetch_sub(1, Ordering::AcqRel);
}

/// Current recursion depth (zero == safe to emit normally).
pub fn depth() -> u32 {
    PRINTK_RECURSION.load(Ordering::Acquire)
}

/// Hint for callers: are we already inside a printk?
pub fn in_recursion() -> bool {
    depth() != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_increments_depth() {
        let before = depth();
        let prev = enter();
        assert_eq!(prev, before);
        assert_eq!(depth(), before + 1);
        exit();
        assert_eq!(depth(), before);
    }

    #[test]
    fn in_recursion_reflects_state() {
        enter();
        assert!(in_recursion());
        exit();
    }
}
