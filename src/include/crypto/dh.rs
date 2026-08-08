// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/crypto/dh.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S012620

/*
 * Diffie-Hellman secret to be used with kpp API along with helper functions
 *
 * Copyright (c) 2016, Intel Corporation
 * Authors: Salvatore Benedetto <salvatore.benedetto@intel.com>
 */

use core::ffi::{c_char, c_void};

/// A Diffie-Hellman private key used with the KPP cipher API.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct dh {
    pub key: *const c_void,
    pub p: *const c_void,
    pub g: *const c_void,
    pub key_size: u32,
    pub p_size: u32,
    pub g_size: u32,
}

unsafe extern "C" {
    /// Obtains the packet DH-key size.
    ///
    /// # Safety
    /// The caller must provide `params` pointing to a readable `dh`.
    pub fn crypto_dh_key_len(params: *const dh) -> u32;

    /// Encodes a private DH key into its packet representation.
    ///
    /// # Safety
    /// The caller must provide `params` pointing to a readable `dh`, `buf`
    /// writable for its advertised `len` bytes, and readable key, `p`, and `g`
    /// regions for the lengths advertised by `params`.
    pub fn crypto_dh_encode_key(buf: *mut c_char, len: u32, params: *const dh) -> i32;

    /// Decodes a packet DH key into `params`.
    ///
    /// # Safety
    /// The caller must provide `buf` readable for `len` bytes and `params`
    /// pointing to writable `dh` storage. The decoded key, `p`, and `g` fields
    /// alias `buf`; `buf` must outlive every use of those fields, and the caller
    /// must provide the required aliasing and synchronization.
    pub fn crypto_dh_decode_key(buf: *const c_char, len: u32, params: *mut dh) -> i32;

    /// Decodes a packet DH key without the exported helper's parameter checks.
    ///
    /// # Safety
    /// The caller must provide `buf` readable for `len` bytes and `params`
    /// pointing to writable `dh` storage. The decoded key, `p`, and `g` fields
    /// alias `buf`; `buf` must outlive every use of those fields, and the caller
    /// must provide the required aliasing and synchronization. These are the
    /// same raw-pointer obligations as `crypto_dh_decode_key`.
    pub fn __crypto_dh_decode_key(buf: *const c_char, len: u32, params: *mut dh) -> i32;
}
