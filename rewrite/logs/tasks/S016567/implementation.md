# Implementation — S016567

Translated `vendor/linux/include/xen/interface/features.h` to
`src/include/xen/interface/features.rs` for the frozen `aarch64` scope.

- Preserved every active Xen feature-index macro as a public `u32` constant
  with the identical macro identifier and integer value: indices 0 through 11,
  13 through 17, and `XENFEAT_NR_SUBMAPS = 1`.
- Preserved the source's intentional absence of `XENFEAT_grant_map_identity`:
  its apparent definition is inside a block comment and therefore is not an
  active C macro.
- The C include guard has no Rust source equivalent. This header contains no
  functions, storage, layout declarations, conditionals selected by the frozen
  ARM64 configuration, or runtime behavior.
- No ABI/linkage claims beyond the exact constant names and values are needed.

Source and frozen scope evidence: `rewrite/SYMBOLS.tsv` rows 365342–365363,
`rewrite/SCOPE.tsv` row S016567, and
`rewrite/metadata/header_closure.tsv`.
