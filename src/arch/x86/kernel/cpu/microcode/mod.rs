//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/cpu/microcode
//! Microcode loader skeleton (per-vendor early/late loading entry points).
//!
//! Per-file ports live in this directory; see [`amd`], [`core`], and [`intel`].

pub mod amd;
pub mod core;
pub mod intel;
