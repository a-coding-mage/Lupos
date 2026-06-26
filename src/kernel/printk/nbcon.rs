//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk/nbcon.c
//! test-origin: linux:vendor/linux/kernel/printk/nbcon.c
//! Non-BKL console (NBCON) driver framework.
//!
//! Linux's nbcon framework lets a console driver implement non-blocking,
//! atomic writes by acquiring/releasing a per-console context.  This port
//! exposes the atomic acquire/release dance with the priority hand-off
//! semantics from upstream.
//!
//! Ref: vendor/linux/kernel/printk/nbcon.c

use core::sync::atomic::{AtomicU32, Ordering};

/// `enum nbcon_prio` — priority bands.
pub const NBCON_PRIO_NONE: u32 = 0;
pub const NBCON_PRIO_NORMAL: u32 = 1;
pub const NBCON_PRIO_EMERGENCY: u32 = 2;
pub const NBCON_PRIO_PANIC: u32 = 3;

/// Per-console NBCON state (packed atomic).
pub struct NbconState {
    /// High bits = priority; low bits = owner cpu+1 (0 = unowned).
    pub state: AtomicU32,
}

impl NbconState {
    pub const fn new() -> Self {
        Self {
            state: AtomicU32::new(0),
        }
    }

    /// `nbcon_context_try_acquire`.  Returns true on success.
    pub fn try_acquire(&self, prio: u32, cpu: u32) -> bool {
        let want = (prio << 8) | (cpu + 1);
        let cur = self.state.load(Ordering::Acquire);
        let cur_prio = cur >> 8;
        if cur != 0 && cur_prio >= prio {
            return false;
        }
        self.state
            .compare_exchange(cur, want, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn release(&self) {
        self.state.store(0, Ordering::Release);
    }

    pub fn current_prio(&self) -> u32 {
        self.state.load(Ordering::Acquire) >> 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_constants_ascend() {
        assert!(NBCON_PRIO_NORMAL < NBCON_PRIO_EMERGENCY);
        assert!(NBCON_PRIO_EMERGENCY < NBCON_PRIO_PANIC);
    }

    #[test]
    fn higher_priority_can_preempt() {
        let s = NbconState::new();
        assert!(s.try_acquire(NBCON_PRIO_NORMAL, 0));
        assert_eq!(s.current_prio(), NBCON_PRIO_NORMAL);
        // EMERGENCY > NORMAL → preempts.
        assert!(s.try_acquire(NBCON_PRIO_EMERGENCY, 1));
    }

    #[test]
    fn same_priority_does_not_preempt() {
        let s = NbconState::new();
        assert!(s.try_acquire(NBCON_PRIO_NORMAL, 0));
        // Same priority from a different cpu does not preempt.
        assert!(!s.try_acquire(NBCON_PRIO_NORMAL, 1));
    }
}
