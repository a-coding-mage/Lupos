//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco
//! x86 confidential-computing runtime surface.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/core.c
//! - vendor/linux/arch/x86/coco/sev/core.c
//! - vendor/linux/arch/x86/coco/sev/noinstr.c
//! - vendor/linux/arch/x86/coco/sev/svsm.c
//! - vendor/linux/arch/x86/coco/sev/vc-handle.c
//! - vendor/linux/arch/x86/coco/sev/vc-shared.c
//! - vendor/linux/arch/x86/coco/tdx/debug.c
//! - vendor/linux/arch/x86/coco/tdx/tdx-shared.c
//! - vendor/linux/arch/x86/coco/tdx/tdx.c

pub mod core;
pub mod sev;
pub mod tdx;
