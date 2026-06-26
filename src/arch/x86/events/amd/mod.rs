//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/events/amd
//! AMD x86 PMU model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/events/amd/brs.c
//! - vendor/linux/arch/x86/events/amd/core.c
//! - vendor/linux/arch/x86/events/amd/ibs.c
//! - vendor/linux/arch/x86/events/amd/iommu.c
//! - vendor/linux/arch/x86/events/amd/lbr.c
//! - vendor/linux/arch/x86/events/amd/power.c
//! - vendor/linux/arch/x86/events/amd/uncore.c

pub mod brs;
pub mod core;
pub mod ibs;
pub mod iommu;
pub mod lbr;
pub mod power;
pub mod uncore;
