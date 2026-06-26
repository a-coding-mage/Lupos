//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kvm
//! x86 KVM virtualization subsystem.

pub mod cpuid;
pub mod debugfs;
pub mod emulate;
pub mod hyperv;
pub mod i8254;
pub mod i8259;
pub mod ioapic;
pub mod irq;
pub mod kvm_asm_offsets;
pub mod kvm_onhyperv;
pub mod lapic;
pub mod mmu;
pub mod mtrr;
pub mod pmu;
pub mod smm;
pub mod svm;
pub mod vmx;
pub mod x86;
pub mod xen;
