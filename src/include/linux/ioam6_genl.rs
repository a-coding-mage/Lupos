// SPDX-License-Identifier: GPL-2.0+
//! linux-source: include/linux/ioam6_genl.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014098

//! Kernel-side IPv6 IOAM generic-netlink definitions.
//!
//! The Linux header contains only this include of the IOAM6 UAPI contract.
//! Re-exporting that module preserves the same declarations without adding
//! storage, synchronization, or a separate netlink interface.

pub use crate::include::uapi::linux::ioam6_genl::*;
