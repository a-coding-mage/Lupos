//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso32
//! 32-bit vDSO wrappers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vgetcpu.c

pub mod vclock_gettime;
pub mod vgetcpu;
