//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kvm/vmx
//! Intel VMX backend model for KVM.
//!
//! Per-file ports live in this directory.

pub mod hyperv_evmcs;
pub mod main;
pub mod nested;
pub mod pmu_intel;
pub mod posted_intr;
pub mod sgx;
pub mod tdx;
pub mod vmcs12;
pub mod vmx;
pub mod vmx_onhyperv;
