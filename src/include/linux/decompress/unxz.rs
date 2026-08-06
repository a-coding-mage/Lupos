// SPDX-License-Identifier: 0BSD
//! linux-source: include/linux/decompress/unxz.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013683

//! Wrapper interface for decompressing XZ-compressed kernel, initramfs, and
//! initrd images.
//!
//! Original author: Lasse Collin <lasse.collin@tukaani.org>
//!
//! The C include guard `DECOMPRESS_UNXZ_H` is represented by this Rust module.

use core::ffi::{c_int, c_long, c_uchar, c_ulong, c_void};

unsafe extern "C" {
    /// Decompresses an XZ stream using the Linux decompressor callback ABI.
    ///
    /// `fill` and `flush` are nullable exactly as in the C declaration.
    /// `error` must be non-null and remain callable for the whole call: the
    /// Linux implementation invokes it without a null check on every decoder
    /// and allocation failure exit. Pointer validity, buffer extents, and all
    /// callback contracts are supplied by the caller.
    pub fn unxz(
        r#in: *mut c_uchar,
        in_size: c_long,
        fill: Option<unsafe extern "C" fn(dest: *mut c_void, size: c_ulong) -> c_long>,
        flush: Option<unsafe extern "C" fn(src: *mut c_void, size: c_ulong) -> c_long>,
        out: *mut c_uchar,
        in_used: *mut c_long,
        error: unsafe extern "C" fn(x: *mut c_uchar),
    ) -> c_int;
}
