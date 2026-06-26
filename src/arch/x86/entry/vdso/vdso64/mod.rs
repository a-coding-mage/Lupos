//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso64
//! 64-bit vDSO wrappers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vgetcpu.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vgetrandom.c

pub mod vclock_gettime;
pub mod vgetcpu;
pub mod vgetrandom;
