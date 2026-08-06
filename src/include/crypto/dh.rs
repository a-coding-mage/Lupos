// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/crypto/dh.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S012620

// Copyright (c) 2016, Intel Corporation

use core::ffi::{c_char, c_uint, c_void};

/// C `struct dh`: a caller-owned Diffie-Hellman private-key description.
///
/// `key`, `p`, and `g` are non-owning byte-addressable pointers.  The C
/// contract supplies their respective byte lengths; this representation does
/// not establish Rust reference validity, ownership, or a lifetime for them.
#[repr(C)]
pub struct dh {
    pub key: *const c_void,
    pub p: *const c_void,
    pub g: *const c_void,
    pub key_size: c_uint,
    pub p_size: c_uint,
    pub g_size: c_uint,
}

unsafe extern "C" {
    /// Returns the encoded packet-key length for `params`.
    pub fn crypto_dh_key_len(params: *const dh) -> c_uint;

    /// Encodes `params` into the caller-owned `buf` of `len` bytes.
    pub fn crypto_dh_encode_key(buf: *mut c_char, len: c_uint, params: *const dh) -> core::ffi::c_int;

    /// Decodes `buf` into `params`, after validating the DH parameters.
    ///
    /// On success, the output pointers in `params` alias storage in `buf`; the
    /// caller must retain that storage for every subsequent use of `params`.
    pub fn crypto_dh_decode_key(buf: *const c_char, len: c_uint, params: *mut dh) -> core::ffi::c_int;

    /// Decodes `buf` into `params` without the public decoder's parameter checks.
    ///
    /// On success, the output pointers in `params` alias storage in `buf`.
    pub fn __crypto_dh_decode_key(buf: *const c_char, len: c_uint, params: *mut dh) -> core::ffi::c_int;
}
