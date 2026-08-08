# S016567 implementation

Translated `include/xen/interface/features.h` from the pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/xen/interface/features.rs` for `aarch64`.

The active upstream object-like macros are represented as public constants with
the same identifiers and `int`-typed literal values (`core::ffi::c_int`).  The
C header guard is represented by Rust module inclusion and therefore has no
Rust item.  `XENFEAT_grant_map_identity` remains absent: its apparent `#define`
is inside an upstream block comment and is not a C macro.

No ABI objects, ownership records, synchronization, allocation, or runtime
behavior are defined by this header. No compiler, formatter, linker, test, or
runtime tooling was used.
