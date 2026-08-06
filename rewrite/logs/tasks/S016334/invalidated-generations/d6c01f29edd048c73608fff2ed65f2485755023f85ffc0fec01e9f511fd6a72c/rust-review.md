# Rust review — S016334 (slot 2)

## Scope and evidence

Reviewed only `src/include/uapi/linux/posix_acl.rs` against the complete pinned
`vendor/linux/include/uapi/linux/posix_acl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus the S016334 scope and symbol
records for both frozen architectures.

## Result

Accepted. No Rust-specific findings.

The pinned header exports only eleven object-like macros.  Each replacement
list is a C `int` constant expression on x86_64 and AArch64: `(-1)`, `0x8000`,
`0x4000`, six tag values through `0x20`, and three permission values through
`0x04`.  The candidate publishes the same public names and values as `i32`,
which preserves that source expression category and the signed
`ACL_UNDEFINED_ID == -1` sentinel.  Downstream Rust translations must make an
explicit cast at a narrower or unsigned use site, corresponding to C's
contextual integer conversion; this header itself declares no target type or
conversion behavior to encode.

There are no structs, unions, FFI declarations, storage, functions, unsafe
blocks, ownership state, conditional configuration branches, tests, or module
side effects in the source header.  The candidate adds none.  Its SPDX
identifier, copyright notices, immutable provenance, Linux revision, common
architecture scope, and task ID match the pinned source and queue record.

No build, formatter, test, or runtime command was run.
