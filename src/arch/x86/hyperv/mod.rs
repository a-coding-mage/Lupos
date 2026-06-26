//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/hyperv
//! test-origin: linux:vendor/linux/arch/x86/hyperv
//! Microsoft Hyper-V x86 enlightenment models.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/hyperv/hv_apic.c
//! - vendor/linux/arch/x86/hyperv/hv_crash.c
//! - vendor/linux/arch/x86/hyperv/hv_init.c
//! - vendor/linux/arch/x86/hyperv/hv_spinlock.c
//! - vendor/linux/arch/x86/hyperv/hv_vtl.c
//! - vendor/linux/arch/x86/hyperv/irqdomain.c
//! - vendor/linux/arch/x86/hyperv/ivm.c
//! - vendor/linux/arch/x86/hyperv/mmu.c
//! - vendor/linux/arch/x86/hyperv/nested.c

pub mod hv_apic;
pub mod hv_crash;
pub mod hv_init;
pub mod hv_spinlock;
pub mod hv_vtl;
pub mod irqdomain;
pub mod ivm;
pub mod mmu;
pub mod mshv_asm_offsets;
pub mod nested;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::EOPNOTSUPP;

    #[test]
    fn hyperv_models_decode_vendor_and_stay_fail_closed() {
        assert!(hv_init::is_hyperv_vendor(*b"Microsoft Hv"));
        assert_eq!(hv_init::hyperv_programming_errno(), EOPNOTSUPP);
        assert_eq!(hv_apic::synthetic_vector_valid(0x20), Ok(()));
        assert!(!hv_spinlock::hv_vcpu_is_preempted(0));
        assert_eq!(hv_spinlock::hv_qlock_kick(0).vector, 0xf7);
    }

    #[test]
    fn hyperv_interrupt_and_flush_models_are_deterministic() {
        let route = irqdomain::HvIrqRoute::new(2, 0x31, 4).unwrap();
        assert_eq!(route.vector, 0x31);
        let flush = mmu::HvTlbFlush::single_address(1, 0x4000);
        assert_eq!(flush.address_space, 1);
        assert_eq!(flush.address_count, 1);
    }
}
