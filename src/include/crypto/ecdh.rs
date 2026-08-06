// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/crypto/ecdh.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S012622

// Copyright (c) 2016, Intel Corporation
// Authors: Salvatore Benedetto <salvatore.benedetto@intel.com>

use core::ffi::{c_char, c_int, c_uint, c_ushort};

/// NIST P-192 curve identifier used by the ECDH implementation.
pub const ECC_CURVE_NIST_P192: c_int = 0x0001;
/// NIST P-256 curve identifier used by the ECDH implementation.
pub const ECC_CURVE_NIST_P256: c_int = 0x0002;
/// NIST P-384 curve identifier used by the ECDH implementation.
pub const ECC_CURVE_NIST_P384: c_int = 0x0003;
/// NIST P-521 curve identifier used by the ECDH implementation.
pub const ECC_CURVE_NIST_P521: c_int = 0x0004;

/// C `struct ecdh`: a caller-owned ECDH private-key description.
///
/// `key` is a mutable, non-owning C-character pointer.  Its valid byte range
/// is `key_size`; neither the pointer nor its storage is owned or freed by
/// this structure.  After successful [`crypto_ecdh_decode_key`], `key` aliases
/// storage in that function's `buf` argument, so the caller must retain the
/// packet buffer for every subsequent use of this structure.
#[repr(C)]
pub struct ecdh {
    pub key: *mut c_char,
    pub key_size: c_ushort,
}

unsafe extern "C" {
    /// Returns the encoded packet-key size for `params` in bytes.
    ///
    /// `params` must be a valid pointer to an initialized [`ecdh`].
    pub fn crypto_ecdh_key_len(params: *const ecdh) -> c_uint;

    /// Encodes `params` into the caller-owned `buf` packet buffer of `len` bytes.
    ///
    /// The ECDH helper requires the supplied length to equal
    /// [`crypto_ecdh_key_len`] for `params`; it returns `-EINVAL` otherwise.
    /// `params.key` must designate at least `params.key_size` readable bytes.
    pub fn crypto_ecdh_encode_key(buf: *mut c_char, len: c_uint, p: *const ecdh) -> c_int;

    /// Decodes a packet key in `buf` into caller-owned `p`.
    ///
    /// On success the function does not allocate or copy the private-key
    /// bytes: it sets `p.key` to alias storage within `buf`.  The caller keeps
    /// the buffer storage alive for as long as `p.key` is used.  Returns
    /// `-EINVAL` for an insufficient or invalid packet buffer, otherwise zero.
    pub fn crypto_ecdh_decode_key(buf: *const c_char, len: c_uint, p: *mut ecdh) -> c_int;
}
