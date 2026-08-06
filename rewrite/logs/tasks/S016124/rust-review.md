# Rust review — S016124

Reviewed `src/include/uapi/linux/falloc.rs` against pinned
`vendor/linux/include/uapi/linux/falloc.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for x86_64 and AArch64.

## Result

Accepted. No Rust-semantics findings.

All nine source definitions are unsuffixed hexadecimal integer constants whose
values fit C `int` on both frozen architectures.  The corresponding Rust
constants are public `i32` values with exactly the same bits and names:
`0x00`, `0x01`, `0x02`, `0x04`, `0x08`, `0x10`, `0x20`, `0x40`, and `0x80`.
`i32` has the required 32-bit signed representation for the Linux fallocate
`int mode` flag interface.  The candidate neither performs arithmetic nor
introduces shifts, casts, truncation, overflow behavior, or altered flag
composition semantics.

This header has no types, storage, functions, FFI/linkage declarations,
conditional configuration, ownership, aliasing, synchronization, allocation,
or unsafe operation.  Rust module inclusion replaces the C textual include
guard; there is consequently no Rust equivalent required for `_UAPI_FALLOC_H_`.
The candidate adds no layout or API behavior beyond the original UAPI names and
values.
