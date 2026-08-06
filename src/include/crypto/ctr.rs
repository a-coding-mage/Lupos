// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/crypto/ctr.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S012618

// CTR: Counter mode
// Copyright (c) 2007 Herbert Xu <herbert@gondor.apana.org.au>

use core::ffi::c_int;

/// Size in bytes of the RFC 3686 nonce prefix.
///
/// Corresponds to C macro `CTR_RFC3686_NONCE_SIZE`.
pub const CTR_RFC3686_NONCE_SIZE: c_int = 4;

/// Size in bytes of the RFC 3686 IV portion of a counter block.
///
/// Corresponds to C macro `CTR_RFC3686_IV_SIZE`.
pub const CTR_RFC3686_IV_SIZE: c_int = 8;

/// Size in bytes of the RFC 3686 counter block.
///
/// Corresponds to C macro `CTR_RFC3686_BLOCK_SIZE`.
pub const CTR_RFC3686_BLOCK_SIZE: c_int = 16;
