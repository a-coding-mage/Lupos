# S000496 applier resolution

Reviewed the pinned `arch/x86/include/asm/cpufeatures.h` in Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 configuration,
the candidate, and both independent reports.

## P1 — SPDX identifier

**Disposition: fixed.** The candidate's first line now preserves the exact
upstream identifier, `SPDX-License-Identifier: GPL-2.0`.  The previous
`GPL-2.0-only` spelling was removed.  Evidence: pinned header line 1 and the
destination provenance line 1.

## Independent final reconciliation

- The candidate provides all 425 object-like `X86_FEATURE_*` definitions and
  all 42 x86_64-selected `X86_BUG_*` definitions with their upstream
  word-times-32-plus-bit expressions.  `NCAPINTS` is 22, `NBUGINTS` is 2, and
  `X86_BUG(x)` computes `NCAPINTS * 32 + x`.
- `X86_BUG_ESPFIX` is correctly absent: it is the only object definition under
  `#ifdef CONFIG_X86_32` (pinned header lines 535–541), while the frozen
  x86_64 configuration selects `CONFIG_X86_64=y` and does not select
  `CONFIG_X86_32`.
- The remaining conditional directives are C include-guard machinery and have
  no Rust runtime declaration.  This constants-only header introduces no
  ABI layout, linkage, ownership, locking, unsafe, allocation, or cleanup
  behavior.  The `u32` indices cover the complete selected range and match
  the downstream unsigned feature-index interfaces recorded by the Rust
  review.
- The task-level semantic records are resolved by this reconciliation: feature
  and bug indices are pure `u32` values, the sole configuration branch is
  excluded for x86_64, and no lifetime, ABI, locking, RCU, refcount, or driver
  contract exists in this header.

The parity review's only finding is resolved.  The Rust review reported no
findings.  No build, formatter, compiler, test, or runtime command was run.
