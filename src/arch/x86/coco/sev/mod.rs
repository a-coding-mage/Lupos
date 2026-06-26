//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/sev
//! AMD SEV/SEV-ES/SEV-SNP runtime helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/sev/core.c
//! - vendor/linux/arch/x86/coco/sev/internal.h
//! - vendor/linux/arch/x86/coco/sev/noinstr.c
//! - vendor/linux/arch/x86/coco/sev/svsm.c
//! - vendor/linux/arch/x86/coco/sev/vc-handle.c
//! - vendor/linux/arch/x86/coco/sev/vc-shared.c

pub mod core;
pub mod internal;
pub mod noinstr;
pub mod svsm;
pub mod vc_handle;
pub mod vc_shared;
