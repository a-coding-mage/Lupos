//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/tdx
//! Intel TDX guest runtime helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/tdx/debug.c
//! - vendor/linux/arch/x86/coco/tdx/tdx-shared.c
//! - vendor/linux/arch/x86/coco/tdx/tdx.c

pub mod debug;
pub mod tdx_guest;
pub mod tdx_shared;

pub use tdx_guest as tdx;
