# Parity review — S016368

Reviewed `src/include/uapi/linux/securebits.rs` against the complete pinned
`vendor/linux/include/uapi/linux/securebits.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result

No parity findings.

## Exhaustive checks

- Provenance identifies the exact Linux source, revision, `common` queue
  architecture class, and task `S016368`.
- The Rust `i32` helper preserves the source macro's signed C `int` result for
  every defined in-header invocation (indices 0 through 11).  Each invocation
  evaluates to the same bit value as `1 << X`; no selected use exercises an
  undefined C shift count.
- `SECUREBITS_DEFAULT`, all twelve secure-setting/locked indices, and all
  twelve individual `SECBIT_*` masks are present with their source values and
  names unchanged.
- Aggregate expressions retain their source members and operations:
  `SECURE_ALL_BITS` is bits 0, 2, 4, 6, 8, and 10; `SECURE_ALL_LOCKS` is that
  result shifted left one; and `SECURE_ALL_UNPRIVILEGED` is bits 8 and 10.
  Their signed-`int` values are respectively `0x555`, `0xaaa`, and `0x500`.
- The source has no conditional configuration branch, data layout, linkage,
  side effect, allocation, locking, or cleanup behavior.  The C include guard
  has no Rust runtime counterpart.

No source, build, formatting, test, or runtime command was performed.
