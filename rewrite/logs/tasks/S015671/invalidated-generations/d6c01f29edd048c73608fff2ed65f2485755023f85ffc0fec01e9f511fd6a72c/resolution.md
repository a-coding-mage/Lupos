# Applier Resolution — S015671

Pinned source reopened: `vendor/linux/include/net/tls_prot.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Review dispositions

- Parity review (slot 1): accepted. Its exhaustive mapping is correct: source
  lines 16–24, 29–32, and 37–66 declare exactly 7, 2, and 28 explicitly valued
  anonymous C enumerators. `src/include/net/tls_prot.rs` retains every name,
  value, and the C `int` constant-expression category as an `i32` constant.
  The include guard at source lines 10–11 and 68 is configuration-independent
  and has no Rust runtime or ABI counterpart.
- Rust review (slot 2): accepted. The source has no enum declarator or object,
  storage, linkage, layout-bearing external ABI, ownership, lifetime, lock,
  RCU, refcount, unsafe, or FFI contract. `pub const` introduces no data
  symbol or storage, so no Rust enum type or other representation is warranted.

No source correction is required. The candidate provenance, SPDX expression,
copyright notice, frozen `common` scope, and all 37 values match the pinned
header. The frozen targets are `x86_64-linux-gnu` and `aarch64-linux-gnu`
(`rewrite/PHASE0_IDENTITY.tsv`); both task architecture rows are closed with
the source facts above.

## Semantic-record closure

- `SYMBOLS.tsv`: both architecture rows for the guard, guard macro, and all
  three anonymous-enum declarations are `COMPLETE`, with source-line and
  frozen-configuration evidence.
- `ABI.tsv`: all three enum declarations on both targets are `NOT_APPLICABLE`
  for linkage, layout, alignment, and calling convention and `NOT_EXPORTED`;
  no object or external ABI exists.
- `LIFETIMES.tsv`: all three enum declarations on both targets are
  `NOT_APPLICABLE` for storage, ownership, lifetime, and locking/RCU/refcount.
- No task rows exist for functions, statics, data objects, exported symbols,
  callbacks, allocation, unsafe/FFI, locking/RCU/refcount, or configuration
  branches other than the include guard: each absent family is N/A.

This is a source-translation completion only; no compile, format, link, test,
or runtime action was performed.
