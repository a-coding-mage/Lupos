// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/events/amd/iommu.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000426

// Copyright (C) 2013 Advanced Micro Devices, Inc.
//
// Author: Steven Kinney <Steven.Kinney@amd.com>
// Author: Suravee Suthikulpanit <Suraveee.Suthikulpanit@amd.com>

//! AMD IOMMU performance-counter MMIO register selectors and hardware limits.
//!
//! The C include guard has no Rust item: this path is the one canonical Rust
//! module for the header, so Rust's module identity supplies its single-
//! definition behavior without a runtime, ABI, or linkage effect.
//!
//! Each object-like C macro is an expression macro so that the literal remains
//! contextually typed at its Rust use site.  In the only selected consumer,
//! the five used register selectors are arguments for the AMD IOMMU `u8` `fxn`
//! parameter and therefore have the same in-range `u8` value as after C's
//! `int`-to-`u8` conversion.  `IOMMU_PC_COUNTER_REPORT_REG`,
//! `PC_MAX_SPEC_BNKS`, and `PC_MAX_SPEC_CNTRS` have no selected use.

/* IOMMU PC MMIO region register indexes. */
macro_rules! iommu_pc_counter_reg {
    () => { 0x00 };
}
pub(crate) use iommu_pc_counter_reg as IOMMU_PC_COUNTER_REG;

macro_rules! iommu_pc_counter_src_reg {
    () => { 0x08 };
}
pub(crate) use iommu_pc_counter_src_reg as IOMMU_PC_COUNTER_SRC_REG;

macro_rules! iommu_pc_pasid_match_reg {
    () => { 0x10 };
}
pub(crate) use iommu_pc_pasid_match_reg as IOMMU_PC_PASID_MATCH_REG;

macro_rules! iommu_pc_domid_match_reg {
    () => { 0x18 };
}
pub(crate) use iommu_pc_domid_match_reg as IOMMU_PC_DOMID_MATCH_REG;

macro_rules! iommu_pc_devid_match_reg {
    () => { 0x20 };
}
pub(crate) use iommu_pc_devid_match_reg as IOMMU_PC_DEVID_MATCH_REG;

macro_rules! iommu_pc_counter_report_reg {
    () => { 0x28 };
}
pub(crate) use iommu_pc_counter_report_reg as IOMMU_PC_COUNTER_REPORT_REG;

/* Maximum hardware-specified performance-counter banks and counters. */
macro_rules! pc_max_spec_bnks {
    () => { 64 };
}
pub(crate) use pc_max_spec_bnks as PC_MAX_SPEC_BNKS;

macro_rules! pc_max_spec_cntrs {
    () => { 16 };
}
pub(crate) use pc_max_spec_cntrs as PC_MAX_SPEC_CNTRS;
