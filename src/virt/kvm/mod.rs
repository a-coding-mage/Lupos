//! linux-parity: partial
//! linux-source: vendor/linux/virt/kvm
//! Generic KVM support shared by architecture-specific backends.

pub mod async_pf;
pub mod binary_stats;
pub mod coalesced_mmio;
pub mod dirty_ring;
pub mod eventfd;
pub mod guest_memfd;
pub mod irqchip;
pub mod kvm_main;
pub mod pfncache;
pub mod vfio;
