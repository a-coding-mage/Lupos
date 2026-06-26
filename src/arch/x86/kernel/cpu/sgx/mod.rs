//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/cpu/sgx
//! Intel SGX (Software Guard Extensions) skeleton.
//!
//! Per-file ports live in this directory; see [`driver`], [`encl`],
//! [`ioctl`], [`main`], and [`virt`].

pub mod driver;
pub mod encl;
pub mod ioctl;
pub mod main;
pub mod virt;
