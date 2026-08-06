# Implementation — S016368

- Linux source: `include/uapi/linux/securebits.h`
- Destination: `src/include/uapi/linux/securebits.rs`
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `x86_64,aarch64`

The complete selected header is represented: `issecure_mask`, the default,
all six secure-setting bit indices and their immutable-lock indices, every
individual `SECBIT_*` mask, and the three aggregate masks.  The C literal in
`issecure_mask` has type `int`; the Rust constants and helper therefore use
`i32`.  Every supplied index is in the source header's defined range 0 through
11, so the signed left shifts used by the selected macros are valid and retain
the original values.

There are no included local definitions, types, ownership, locking, ABI layout,
or configuration branches. The C include guard has no Rust counterpart.
