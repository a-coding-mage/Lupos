//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kvm/svm
//! AMD SVM (Secure Virtual Machine) per-vendor KVM backend.
//!
//! Per-file ports live in this directory; see [`avic`] and [`hyperv`].

pub mod avic;
pub mod hyperv;
pub mod nested;
pub mod sev;
pub mod svm;
pub mod svm_onhyperv;
