# Implementation — S013801

Translated `include/linux/dqblk_v1.h` to
`src/include/linux/dqblk_v1.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete source header has an include guard and four unconditional
object-like macros only: `V1_INIT_ALLOC`, `V1_INIT_REWRITE`, `V1_DEL_ALLOC`,
and `V1_DEL_REWRITE`.  Their replacement literals have C `int` type, so the
fresh translation exposes zero-argument expression macros with exact `i32`
literals.  This preserves macro-like expression expansion rather than
introducing addressable storage or a convenience quota representation.

Both frozen configurations select the header through the recorded common
header closure.  `include/linux/quota.h` consumes all four values in its
`DQUOT_*` maximum expressions.  No structures, unions, layouts, endianness,
packing, linkage, allocation, lifetime, locking, error path, or
configuration-controlled branch exists in the source header.

No compiler, formatter, linker, test, runtime command, historical Rust
source, or non-leased source file was used or changed.
