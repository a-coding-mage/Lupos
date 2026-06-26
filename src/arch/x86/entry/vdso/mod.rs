//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry/vdso
//! x86 vDSO entry helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/common/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/common/vgetcpu.c
//! - vendor/linux/arch/x86/entry/vdso/extable.c
//! - vendor/linux/arch/x86/entry/vdso/vdso32-setup.c
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vgetcpu.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vgetcpu.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vgetrandom.c

pub mod common;
pub mod extable;
pub mod vdso32;
pub mod vdso32_setup;
pub mod vdso64;
