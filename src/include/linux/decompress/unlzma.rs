// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/decompress/unlzma.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013681

//! LZMA decompressor entry-point declaration.
//!
//! The C include guard `DECOMPRESS_UNLZMA_H` is represented by this Rust
//! module. This declaration intentionally supplies only the header ABI; the
//! LZMA implementation belongs to `lib/decompress_unlzma.c`.

use core::ffi::{c_int, c_long, c_uchar, c_ulong, c_void};

unsafe extern "C" {
    /// Decompresses an LZMA stream using the Linux decompressor callback ABI.
    ///
    /// `buf`, `fill`, `flush`, `output`, and `posp` preserve the nullable C
    /// parameter contracts. `error` must be non-null and callable for the
    /// entire invocation: the Linux implementation calls it directly on its
    /// allocation, header, and corrupt-input error paths. Its raw unsigned-byte
    /// argument preserves the frozen `-funsigned-char` C `char *` contract.
    /// The caller owns all pointed-to storage and establishes buffer extents
    /// and callback lifetimes for this call.
    pub fn unlzma(
        buf: *mut c_uchar,
        in_len: c_long,
        fill: Option<unsafe extern "C" fn(*mut c_void, c_ulong) -> c_long>,
        flush: Option<unsafe extern "C" fn(*mut c_void, c_ulong) -> c_long>,
        output: *mut c_uchar,
        posp: *mut c_long,
        error: unsafe extern "C" fn(*mut c_uchar),
    ) -> c_int;
}
