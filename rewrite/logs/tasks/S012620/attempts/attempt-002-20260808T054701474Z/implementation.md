# Implementation — S012620

Translated `vendor/linux/include/crypto/dh.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the selected `aarch64`
header closure.

`dh` retains its C field order: three `const void *` fields followed by three
`unsigned int` sizes. The Rust declaration is `#[repr(C)]` and `Copy, Clone`;
the four declarations retain their C symbol names, pointer constness, `char`,
`unsigned int`, and `int` ABI categories through `core::ffi` types.

The immutable provenance, upstream Intel copyright notice, and Salvatore
Benedetto authorship notice are retained. The C include guard has no Rust
module-level analogue.

Destination SHA-256 at implementation seal:
`eca68d69b1f1c13d6cac030e0141367eab1531a1fb7625a257a98221b1641873`.

No compiler, formatter, linker, test, runtime, or historical-source command
was used.
