//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/paravirt-spinlocks.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/paravirt-spinlocks.c
//! x86 paravirtual spinlock capability state.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/paravirt-spinlocks.c

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, Ordering};

pub static VIRT_SPIN_LOCK_KEY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
pub struct PvLockOps {
    pub queued_spin_unlock: fn(&AtomicBool),
    pub vcpu_is_preempted: fn(u32) -> bool,
}

pub fn native_queued_spin_unlock(lock: &AtomicBool) {
    lock.store(false, Ordering::Release);
}

pub const fn native_vcpu_is_preempted(_cpu: u32) -> bool {
    false
}

pub static PV_LOCK_OPS: PvLockOps = PvLockOps {
    queued_spin_unlock: native_queued_spin_unlock,
    vcpu_is_preempted: native_vcpu_is_preempted,
};

pub fn native_pv_lock_init(hypervisor_present: bool) {
    VIRT_SPIN_LOCK_KEY.store(hypervisor_present, Ordering::Release);
}

pub fn virt_spin_lock_enabled() -> bool {
    VIRT_SPIN_LOCK_KEY.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pv_lock_init_tracks_hypervisor_presence() {
        native_pv_lock_init(false);
        assert!(!virt_spin_lock_enabled());
        native_pv_lock_init(true);
        assert!(virt_spin_lock_enabled());
    }

    #[test]
    fn native_unlock_clears_lock_and_preempted_is_false() {
        let lock = AtomicBool::new(true);
        (PV_LOCK_OPS.queued_spin_unlock)(&lock);
        assert!(!lock.load(Ordering::Acquire));
        assert!(!(PV_LOCK_OPS.vcpu_is_preempted)(0));
    }
}
