// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/dqblk_v1.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013801

//! In-memory constants for the old quota format.

/// C `V1_INIT_ALLOC`: blocks needed when initializing quota allocation.
pub const V1_INIT_ALLOC: core::ffi::c_int = 1;

/// C `V1_INIT_REWRITE`: blocks needed when rewriting initialized quota data.
pub const V1_INIT_REWRITE: core::ffi::c_int = 1;

/// C `V1_DEL_ALLOC`: blocks needed when allocating for quota deletion.
pub const V1_DEL_ALLOC: core::ffi::c_int = 0;

/// C `V1_DEL_REWRITE`: blocks needed when rewriting deleted quota data.
pub const V1_DEL_REWRITE: core::ffi::c_int = 2;
