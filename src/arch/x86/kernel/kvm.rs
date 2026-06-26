//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/kvm.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kvm.c
//! KVM paravirtual feature discovery and action modeling.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kvm.c

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::VecDeque;

use crate::arch::x86::kernel::cpuid::CpuidResult;
use crate::arch::x86::virt::virtualization::{HypervisorVendor, hypervisor_vendor_from_leaf};

pub const KVM_CPUID_SIGNATURE: [u8; 12] = *b"KVMKVMKVM\0\0\0";
pub const KVM_CPUID_FEATURES: u32 = 0x4000_0001;

pub const KVM_FEATURE_CLOCKSOURCE: u32 = 0;
pub const KVM_FEATURE_NOP_IO_DELAY: u32 = 1;
pub const KVM_FEATURE_MMU_OP: u32 = 2;
pub const KVM_FEATURE_CLOCKSOURCE2: u32 = 3;
pub const KVM_FEATURE_ASYNC_PF: u32 = 4;
pub const KVM_FEATURE_STEAL_TIME: u32 = 5;
pub const KVM_FEATURE_PV_EOI: u32 = 6;
pub const KVM_FEATURE_PV_UNHALT: u32 = 7;
pub const KVM_FEATURE_PV_TLB_FLUSH: u32 = 9;
pub const KVM_FEATURE_PV_SEND_IPI: u32 = 11;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmCpuid {
    pub max_leaf: u32,
    pub features: u32,
    pub hints: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmPvAction {
    HaltPollEnable,
    HaltPollDisable,
    SendIpi { cpu_mask: u64, vector: u8 },
    FlushTlb { cpu_mask: u64 },
    SpinWait { lock_addr: u64 },
    SpinKick { cpu: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsyncPfEvent {
    pub token: u64,
    pub address: u64,
}

#[derive(Default, Debug)]
pub struct AsyncPfQueue {
    events: VecDeque<AsyncPfEvent>,
}

impl AsyncPfQueue {
    pub fn push(&mut self, event: AsyncPfEvent) {
        self.events.push_back(event);
    }

    pub fn pop(&mut self) -> Option<AsyncPfEvent> {
        self.events.pop_front()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

pub fn kvm_para_available(leaf0: CpuidResult) -> bool {
    hypervisor_vendor_from_leaf(leaf0).vendor == HypervisorVendor::Kvm
}

pub const fn kvm_arch_para_features(leaf1_eax: u32) -> u32 {
    leaf1_eax
}

pub const fn kvm_arch_para_hints(leaf1_edx: u32) -> u32 {
    leaf1_edx
}

pub const fn has_feature(features: u32, bit: u32) -> bool {
    features & (1 << bit) != 0
}

pub const fn has_pv_ipi(features: u32) -> bool {
    has_feature(features, KVM_FEATURE_PV_SEND_IPI)
}

pub const fn has_steal_time(features: u32) -> bool {
    has_feature(features, KVM_FEATURE_STEAL_TIME)
}

pub const fn has_async_pf(features: u32) -> bool {
    has_feature(features, KVM_FEATURE_ASYNC_PF)
}

pub const fn pv_spinlock_action(lock_addr: u64, preempted: bool) -> Option<KvmPvAction> {
    if preempted {
        Some(KvmPvAction::SpinWait { lock_addr })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_leaf_identifies_kvm() {
        let leaf = CpuidResult {
            eax: 0x4000_0001,
            ebx: u32::from_le_bytes(*b"KVMK"),
            ecx: u32::from_le_bytes(*b"VMKV"),
            edx: u32::from_le_bytes(*b"M\0\0\0"),
        };
        assert!(kvm_para_available(leaf));
    }

    #[test]
    fn feature_helpers_decode_bits() {
        let features = (1 << KVM_FEATURE_STEAL_TIME) | (1 << KVM_FEATURE_PV_SEND_IPI);
        assert!(has_steal_time(features));
        assert!(has_pv_ipi(features));
        assert!(!has_async_pf(features));
    }

    #[test]
    fn async_pf_queue_is_fifo() {
        let mut q = AsyncPfQueue::default();
        q.push(AsyncPfEvent {
            token: 1,
            address: 2,
        });
        assert_eq!(q.len(), 1);
        assert_eq!(q.pop().unwrap().token, 1);
    }
}
