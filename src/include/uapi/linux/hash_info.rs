// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/hash_info.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016143

/*
 * Hash Info: Hash algorithms information
 *
 * Copyright (c) 2013 Dmitry Kasatkin <d.kasatkin@samsung.com>
 */

use core::ffi::c_int;

/// C `enum hash_algo`: a distinct tag with the frozen signed-`int` ABI.
///
/// The transparent representation accepts every C `int` bit pattern, including
/// the negative result temporarily stored by UBIFS before it diagnoses an
/// unsuccessful `match_string()` lookup.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct hash_algo(pub c_int);

// C enumerators are unqualified `int` constant expressions, not scoped
// members of the `enum hash_algo` tag.
pub const HASH_ALGO_MD4: c_int = 0;
pub const HASH_ALGO_MD5: c_int = 1;
pub const HASH_ALGO_SHA1: c_int = 2;
pub const HASH_ALGO_RIPE_MD_160: c_int = 3;
pub const HASH_ALGO_SHA256: c_int = 4;
pub const HASH_ALGO_SHA384: c_int = 5;
pub const HASH_ALGO_SHA512: c_int = 6;
pub const HASH_ALGO_SHA224: c_int = 7;
pub const HASH_ALGO_RIPE_MD_128: c_int = 8;
pub const HASH_ALGO_RIPE_MD_256: c_int = 9;
pub const HASH_ALGO_RIPE_MD_320: c_int = 10;
pub const HASH_ALGO_WP_256: c_int = 11;
pub const HASH_ALGO_WP_384: c_int = 12;
pub const HASH_ALGO_WP_512: c_int = 13;
pub const HASH_ALGO_TGR_128: c_int = 14;
pub const HASH_ALGO_TGR_160: c_int = 15;
pub const HASH_ALGO_TGR_192: c_int = 16;
pub const HASH_ALGO_SM3_256: c_int = 17;
pub const HASH_ALGO_STREEBOG_256: c_int = 18;
pub const HASH_ALGO_STREEBOG_512: c_int = 19;
pub const HASH_ALGO_SHA3_256: c_int = 20;
pub const HASH_ALGO_SHA3_384: c_int = 21;
pub const HASH_ALGO_SHA3_512: c_int = 22;
pub const HASH_ALGO__LAST: c_int = 23;
