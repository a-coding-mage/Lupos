// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/sunrpc/xprtrdma.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S015110

// Upstream SPDX-License-Identifier: GPL-2.0 OR BSD-3-Clause
/*
 * Copyright (c) 2003-2007 Network Appliance, Inc. All rights reserved.
 */

/*
 * Constants. The maximum RPC/NFS header accounts for additional marshaling
 * buffers passed down by the Linux client. The fixed maximum RDMA header is
 * sufficient for a fully chunked NFS message, with only one chunk type per
 * message currently supported.
 */
pub const RPCRDMA_MIN_SLOT_TABLE: u32 = 4;
pub const RPCRDMA_DEF_SLOT_TABLE: u32 = 128;
pub const RPCRDMA_MAX_SLOT_TABLE: u32 = 16_384;

pub const RPCRDMA_MIN_INLINE: i32 = 1_024;
pub const RPCRDMA_DEF_INLINE: i32 = 4_096;
pub const RPCRDMA_MAX_INLINE: i32 = 65_536;

/*
 * Memory registration strategies, by number.  In C, the tag has the frozen
 * target's `int` representation, while each enumerator is an `int` constant
 * expression.  Keep this as an integer API: consumers initialize unsigned
 * values from it and perform arithmetic such as `RPCRDMA_LAST - 1`.
 */
#[allow(non_camel_case_types)]
pub type rpcrdma_memreg = i32;

pub const RPCRDMA_BOUNCEBUFFERS: rpcrdma_memreg = 0;
pub const RPCRDMA_REGISTER: rpcrdma_memreg = 1;
pub const RPCRDMA_MEMWINDOWS: rpcrdma_memreg = 2;
pub const RPCRDMA_MEMWINDOWS_ASYNC: rpcrdma_memreg = 3;
pub const RPCRDMA_MTHCAFMR: rpcrdma_memreg = 4;
pub const RPCRDMA_FRWR: rpcrdma_memreg = 5;
pub const RPCRDMA_ALLPHYSICAL: rpcrdma_memreg = 6;
pub const RPCRDMA_LAST: rpcrdma_memreg = 7;
