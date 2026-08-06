# S016344 parity review — PASS

Reviewed `src/include/uapi/linux/psp.rs` exhaustively against the pinned
`vendor/linux/include/uapi/linux/psp.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result

PASS — no actionable parity findings.

## Evidence

- The complete named `enum psp_version` is present as `psp_version: c_int`;
  its four enumerators have the exact implicit C values 0 through 3.
- All six anonymous enum namespaces retain every explicit/implicit value:
  association-device-info (1, 2, sentinel 3, max 2), device (1 through 8,
  max 7), association (1 through 6, max 5), keys (1 through 3, max 2),
  statistics (1 through 12, max 11), and commands (1 through 13, max 12).
  Each private `__PSP_*_MAX` sentinel and public `PSP_*_MAX = sentinel - 1`
  relationship is preserved.  In total, all 55 C enumerator declarations are
  represented with their exact names and values.
- The frozen UAPI macros are complete: `PSP_FAMILY_VERSION` is `c_int` value
  1, and the three string macros contain the exact ASCII bytes and trailing
  NULs: `psp`, `mgmt`, and `use`.  Their immutable `c_char` arrays preserve
  C string storage and pointer-use semantics for translated consumers.
- The source header has no structs, functions, configuration-dependent
  branches, mutable state, or additional operative macros.  Its C include
  guard has no Rust-module analogue and is correctly not materialized.
- SPDX is exactly `((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`;
  source-path, pinned revision, architecture (`common`), and rewrite task
  provenance are exact.  There are no branding changes, tests, unsafe blocks,
  placeholders, or extra symbols.

This was a source-only review.  No compilation, formatting, test, runtime, or
source-file action was performed.
