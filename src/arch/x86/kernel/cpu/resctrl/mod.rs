//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/cpu/resctrl
//! Intel/AMD Resource Director Technology (RDT) skeleton.
//!
//! Per-file ports live in this directory; see [`core`], [`ctrlmondata`],
//! [`intel_aet`], [`monitor`], [`pseudo_lock`], and [`rdtgroup`].

pub mod core;
pub mod ctrlmondata;
pub mod intel_aet;
pub mod monitor;
pub mod pseudo_lock;
pub mod rdtgroup;
