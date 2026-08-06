# Parity review — S016334 (slot 1)

Reviewed `src/include/uapi/linux/posix_acl.rs` against the complete pinned
`vendor/linux/include/uapi/linux/posix_acl.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen common x86_64 and
AArch64 scope.

## Result

No parity findings.

## Coverage and evidence

- The Rust provenance identifies the exact Linux source path, pinned revision,
  `common` architecture membership, and task ID `S016334`.  Its SPDX identifier
  and both upstream copyright notices match the source header.
- The header has no includes, declarations, storage, functions, ABI layouts, or
  configuration-selected code.  Its only operative payload is twelve C integer
  macro expressions.  The candidate contains exactly the same twelve public
  names: `ACL_UNDEFINED_ID`; `ACL_TYPE_ACCESS`, `ACL_TYPE_DEFAULT`;
  `ACL_USER_OBJ`, `ACL_USER`, `ACL_GROUP_OBJ`, `ACL_GROUP`, `ACL_MASK`,
  `ACL_OTHER`; and `ACL_READ`, `ACL_WRITE`, `ACL_EXECUTE`.
- All replacement values are exact: `-1`, `0x8000`, `0x4000`, `0x01`, `0x02`,
  `0x04`, `0x08`, `0x10`, `0x20`, `0x04`, `0x02`, and `0x01`, respectively.
  Each original unsuffixed literal is a C `int` expression under both frozen
  targets; representing it as `i32` preserves its 32-bit signed value category
  and every listed value.
- The C include guard is correctly absent as a Rust item: it prevents repeated
  C preprocessor inclusion only and has no UAPI value or ABI payload.  No
  identifier was renamed, branded, omitted, or assigned new behavior.
- The task's frozen `SYMBOLS.tsv` rows enumerate exactly this guard and these
  twelve operative macros for both architectures.  There are no S016334 rows
  in `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, or the branding allowlist
  requiring additional source content.
- The candidate adds no Rust tests, conditional behavior, types, storage,
  functions, unsafe code, stubs, or substitute mechanism.
