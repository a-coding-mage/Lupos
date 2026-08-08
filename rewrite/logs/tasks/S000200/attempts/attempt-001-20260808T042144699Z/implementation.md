# S000200 implementation

Translated `arch/arm64/include/asm/vncr_mapping.h` into the path-preserving
`src/arch/arm64/include/asm/vncr_mapping.rs`.

The source contains only object-like macros whose values are byte displacements
within the VNCR page. Every selected `VNCR_*` macro is represented by a public
`usize` constant with the identical hexadecimal value; `usize` expresses its
use as an architecture-local address displacement on the frozen AArch64 target.
The C include guard has no Rust runtime or linkage counterpart.

No types, layouts, functions, storage, synchronization, unsafe operations, or
configuration-dependent executable branches are present in the pinned header.
The source was reviewed directly at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`; no build, formatter, compiler, or
test tool was used.
