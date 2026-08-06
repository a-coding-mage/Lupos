// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/decompress/generic.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013677

use core::ffi::{c_char, c_int, c_long, c_uchar, c_ulong, c_void};

/// C `decompress_fn`: a decompressor entry point.
///
/// `inbuf` is the input buffer and `len` is its pre-read byte count.  With a
/// nonzero `len`, `inbuf` contains all required input and `fill` is absent.
/// With a zero `len`, `inbuf` may be null; otherwise it identifies an input
/// buffer whose minimum size is determined by the selected decompressor.
/// `fill` may be called repeatedly to refill that buffer.
///
/// When `flush` is absent, `outbuf` identifies storage large enough for all
/// output.  When `flush` is present, `outbuf` is null, the decompressor
/// allocates its output buffer, and calls `flush` as its stream requires.
/// If `posp` is non-null, the decompressor stores the number of input bytes
/// read through it.  The callback pointers are nullable exactly as in C.
pub type decompress_fn = Option<
    unsafe extern "C" fn(
        inbuf: *mut c_uchar,
        len: c_long,
        fill: Option<unsafe extern "C" fn(*mut c_void, c_ulong) -> c_long>,
        flush: Option<unsafe extern "C" fn(*mut c_void, c_ulong) -> c_long>,
        outbuf: *mut c_uchar,
        posp: *mut c_long,
        error: Option<unsafe extern "C" fn(*mut c_char)>,
    ) -> c_int,
>;

unsafe extern "C" {
    /// Detects the decompression method represented by the input magic.
    ///
    /// `inbuf` must designate `len` readable bytes.  When non-null, `name`
    /// receives the matching compression-name pointer, or null when fewer
    /// than two bytes are available; the returned decompressor is nullable.
    pub fn decompress_method(
        inbuf: *const c_uchar,
        len: c_long,
        name: *mut *const c_char,
    ) -> decompress_fn;
}
