# Implementation — S016002

Translated `include/uapi/asm-generic/errno-base.h` to
`src/include/uapi/asm-generic/errno-base.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source contains exactly 34 unconditional base errno macros, `EPERM`
through `ERANGE`, with the consecutive values 1 through 34. Each is preserved
as a `core::ffi::c_int` constant: the upstream definitions are unsuffixed
decimal integer literals and therefore have C `int` type on both frozen LP64
x86_64 and AArch64 configurations. The C include guard has no Rust runtime or
ABI equivalent. There are no functions, storage, configuration branches,
ownership, locking, cleanup, or error paths beyond the exported values.

No branding delta, tests, drivers, module indexes, formatting, compilation, or
runtime actions were added or performed.
