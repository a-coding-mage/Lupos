//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/cpu/mtrr
//! Per-vendor MTRR initialization and /proc interface.
//!
//! Per-file ports live in this directory; see [`amd`], [`centaur`],
//! [`cleanup`], [`cyrix`], and [`mtrr_if`]. The architectural decoder for
//! the generic MTRR memory-type register lives at
//! `crate::arch::x86::kernel::mtrr` and is intentionally not re-exported here.

pub mod amd;
pub mod centaur;
pub mod cleanup;
pub mod cyrix;
pub mod legacy;
pub mod mtrr_core;
pub mod mtrr_if;
