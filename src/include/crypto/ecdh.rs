// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/crypto/ecdh.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S012622

//! ECDH parameters and KPP packet-key interface.

/// NIST P-192 curve identifier.
pub const ECC_CURVE_NIST_P192: u32 = 0x0001;
/// NIST P-256 curve identifier.
pub const ECC_CURVE_NIST_P256: u32 = 0x0002;
/// NIST P-384 curve identifier.
pub const ECC_CURVE_NIST_P384: u32 = 0x0003;
/// NIST P-521 curve identifier.
pub const ECC_CURVE_NIST_P521: u32 = 0x0004;

/// ECDH private-key parameters passed to the KPP API.
#[repr(C)]
pub struct Ecdh {
    /// Private ECDH key. The pointed-to storage is owned by the caller.
    pub key: *mut core::ffi::c_char,
    /// Size of the private key in bytes.
    pub key_size: u16,
}

extern "C" {
    /// Return the packet representation size for an ECDH private key.
    pub fn crypto_ecdh_key_len(params: *const Ecdh) -> u32;

    /// Encode a private key into the caller-provided packet buffer.
    pub fn crypto_ecdh_encode_key(
        buf: *mut core::ffi::c_char,
        len: u32,
        params: *const Ecdh,
    ) -> core::ffi::c_int;

    /// Decode a packet private key, making `params.key` point into `buf`.
    pub fn crypto_ecdh_decode_key(
        buf: *const core::ffi::c_char,
        len: u32,
        params: *mut Ecdh,
    ) -> core::ffi::c_int;
}
