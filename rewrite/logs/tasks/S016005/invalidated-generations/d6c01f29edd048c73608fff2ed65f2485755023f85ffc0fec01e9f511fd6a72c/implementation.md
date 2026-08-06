# Implementation — S016005

Source oracle: `vendor/linux/include/uapi/asm-generic/hugetlb_encode.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The frozen Phase 0 inventory classifies this common UAPI header as
`RUST_TRANSLATE`, with no task dependencies. Its canonical architecture label
is `common`; `SYMBOLS.tsv` records the include guard condition and all 15
encoding macro definitions for both x86_64 and AArch64. `LIFETIMES.tsv` and
`ABI.tsv` contain no task-specific records: this header declares neither
storage nor ABI-bearing types.

The Rust file maps each operative C macro to a public `u32` constant. The C
size encoding operands carry the `U` suffix, so `u32` preserves their unsigned
32-bit values and shift behavior on both approved architectures. The include
guard has no Rust analogue; Rust module inclusion supplies that role. The
constants retain the original shift expression so derived values remain tied to
`HUGETLB_FLAG_ENCODE_SHIFT`.

No compiler, formatter, test, linker, or runtime command was run.
