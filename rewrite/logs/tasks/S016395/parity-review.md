# Parity review — S016395, attempt 2, slot 1

Reviewed `vendor/linux/include/uapi/linux/sunrpc_netlink.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against the current candidate
`src/include/uapi/linux/sunrpc_netlink.rs`.

## Result

PASS — no parity findings.

## Checks

- The candidate retains the required SPDX expression and immutable provenance:
  source path, pinned revision, `common` architecture set, and task `S016395`.
- `sunrpc_cache_type` and each of the seven anonymous enums are represented by
  `c_int`.  Every declared enumerator has its original integer value, and every
  `*_MAX` value is still expressed as its corresponding `__*MAX - 1` relation.
- `rewrite/ABI.tsv` has a `COMPLETE` row for each of those eight enum groups on
  both x86_64 and AArch64.  Each row establishes the frozen no-`-fshort-enums`
  ABI as a signed C `int`, size 4 and alignment 4; this agrees with the
  candidate's `c_int` constants/type alias.
- `SUNRPC_FAMILY_NAME`, `SUNRPC_MCGRP_NONE`, and `SUNRPC_MCGRP_EXPORTD` remain
  byte-array representations of their C string-literal macros, with exact
  spellings, trailing NULs, and lengths 7, 5, and 8.  The version macro remains
  the integer value 1.
- The candidate adds no pointer-conversion/decode helper, wrapper, substitute
  API, changed identifier, or branding delta.  Pointer decay remains a
  consuming-use concern, as it is in the original macros.
- No source behavior exists beyond these UAPI declarations; all selected enum
  groups and operative macro values from the pinned header are present.

No source was edited and no compile, formatter, test, or runtime command was
run.
