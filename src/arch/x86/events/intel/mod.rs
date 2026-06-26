//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/events/intel
//! Intel x86 PMU model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/events/intel/bts.c
//! - vendor/linux/arch/x86/events/intel/core.c
//! - vendor/linux/arch/x86/events/intel/cstate.c
//! - vendor/linux/arch/x86/events/intel/ds.c
//! - vendor/linux/arch/x86/events/intel/knc.c
//! - vendor/linux/arch/x86/events/intel/lbr.c
//! - vendor/linux/arch/x86/events/intel/p4.c
//! - vendor/linux/arch/x86/events/intel/p6.c
//! - vendor/linux/arch/x86/events/intel/pt.c
//! - vendor/linux/arch/x86/events/intel/uncore.c
//! - vendor/linux/arch/x86/events/intel/uncore_discovery.c
//! - vendor/linux/arch/x86/events/intel/uncore_nhmex.c
//! - vendor/linux/arch/x86/events/intel/uncore_snb.c
//! - vendor/linux/arch/x86/events/intel/uncore_snbep.c

pub mod bts;
pub mod core;
pub mod cstate;
pub mod ds;
pub mod knc;
pub mod lbr;
pub mod p4;
pub mod p6;
pub mod pt;
pub mod uncore;
pub mod uncore_discovery;
pub mod uncore_nhmex;
pub mod uncore_snb;
pub mod uncore_snbep;
