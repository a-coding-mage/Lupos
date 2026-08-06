# Resolution — S016371

Source rechecked: `vendor/linux/include/uapi/linux/seg6_genl.h:5-31` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, together with the selected
consumer `vendor/linux/net/ipv6/seg6.c:494-507` and
`vendor/linux/include/net/genetlink.h:78-82`.

## P1 / R1 — `SEG6_GENL_NAME` literal storage, pointer decay, and aggregate initialization

**Accepted and resolved.**  The C macro expands to the `char[5]` string
literal `"SEG6"`, whose elements are `S`, `E`, `G`, `6`, and a terminating
zero.  `src/include/uapi/linux/seg6_genl.rs` now represents its static-storage
form as `pub static SEG6_GENL_NAME: [c_char; 5]`.  A translated
pointer-expression use must take `SEG6_GENL_NAME.as_ptr()`, matching C
array-to-pointer conversion without borrowing a temporary copied `const`.

The selected C use is instead the aggregate initializer
`.name = SEG6_GENL_NAME` for `struct genl_family.name`, which is
`char[GENL_NAMSIZ]`.  Its Rust mapping is
`seg6_genl_name_array::<GENL_NAMSIZ>()`: the helper writes the literal prefix
including its zero terminator and begins from an all-zero aggregate, preserving
the C aggregate initializer's zero-filled remainder.  With the pinned
`GENL_NAMSIZ` target this is exactly the five literal elements followed by the
remaining zero elements.  The future owner of `net/ipv6/seg6.c` must use this
aggregate form for `seg6_genl_family.name`, rather than a pointer or a copied
five-element array.

## Enum constants and derived maxima

**Verified.**  Both anonymous C enums have no named enum object type; all
enumerators remain `c_int` constant expressions with their source ordinals:
`SEG6_ATTR_*` is `0..8`, `SEG6_CMD_*` is `0..5`, and the public maximum macros
remain `__SEG6_ATTR_MAX - 1` (`7`) and `__SEG6_CMD_MAX - 1` (`4`).

## Final semantic-record closure

The header is unconditional for both frozen architectures and has no callable
behavior, allocation, ownership, locking, ABI-bearing aggregate, or
configuration branch beyond its C include guard.  The `PENDING_REVIEW` Phase 0
records for the two anonymous enum declarations, operative macros, and include
guard are closed for this task by the source mappings above; no manifest was
edited.

No compiler, formatter, linker, test, runtime, or diagnostic tool was used.
