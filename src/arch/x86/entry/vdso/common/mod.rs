//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry/vdso/common
//! Shared vDSO C-library entry points.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/common/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/common/vgetcpu.c

pub mod vclock_gettime;
pub mod vgetcpu;
