// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/kernel-page-flags.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014173

//! Kernel-only page-flag bit positions.

// The C header includes the stable UAPI bit positions, so consumers of this
// kernel header receive those names as well.
pub use crate::include::uapi::linux::kernel_page_flags::*;

// Kernel hacking assistances. Subject to change; never rely on them.
pub const KPF_RESERVED: i32 = 32;
pub const KPF_MLOCKED: i32 = 33;
pub const KPF_OWNER_2: i32 = 34;
pub const KPF_PRIVATE: i32 = 35;
pub const KPF_PRIVATE_2: i32 = 36;
pub const KPF_OWNER_PRIVATE: i32 = 37;
pub const KPF_ARCH: i32 = 38;
pub const KPF_SOFTDIRTY: i32 = 40;
pub const KPF_ARCH_2: i32 = 41;
pub const KPF_ARCH_3: i32 = 42;
