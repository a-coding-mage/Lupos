Task S016011 implementation evidence

Source: vendor/linux/include/uapi/asm-generic/mman-common.h
Destination: src/include/uapi/asm-generic/mman-common.rs
Linux revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
Architectures: common (selected by x86_64 and aarch64)

The complete 94-line pinned header was read. It contains only the include guard,
comments, and 52 operative object-like macros. The fresh Rust file maps each
operative macro to a public `u32` constant, preserving all literal values and
the `PKEY_ACCESS_MASK` bitwise expression. The include guard has no runtime or
ABI effect and is represented by the Rust module boundary. No unsafe code,
callers, callees, locking, allocation, or lifetime behavior is present.

Frozen records checked before mutation: task row S016011, scope and file map,
symbols, ABI/lifetime manifests, Linux SHA, and the supplied identity/queue/
scope/symbol/ABI/lifetime hashes. No compiler, formatter, test, runtime, or
historical Lupos source was used.

Semantic closure: COMPLETE for all selected symbols and conditions.
