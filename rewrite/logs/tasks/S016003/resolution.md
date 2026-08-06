# Applier resolution — S016003

Task `S016003` translates `include/uapi/asm-generic/errno.h` to
`src/include/uapi/asm-generic/errno.rs` for the frozen common scope (`x86_64`
and `aarch64`) at pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Independent source recheck

I reopened all 127 lines of the pinned header, its direct provider
`include/uapi/asm-generic/errno-base.h`, the candidate, frozen task records,
and both independent review reports.  The source contributes exactly 102
errno definitions: 98 decimal `int` literals from `EDEADLK = 35` through
`EFTYPE = 134` (with the intentional absence of 58), plus four aliases:
`EWOULDBLOCK = EAGAIN`, `EDEADLOCK = EDEADLK`, `EFSBADCRC = EBADMSG`, and
`EFSCORRUPTED = EUCLEAN`.  The candidate has exactly those 102 public
`core::ffi::c_int` constants and preserves each alias operand.  The direct C
include is represented by `pub use super::errno_base::*`, making the completed
base errno provider visible at the same module interface.

The unsuffixed decimal source literals are C `int` values on both frozen LP64
architectures; `core::ffi::c_int` is the corresponding target C `int` type.
The include guard at lines 2--3 and 127 has no Rust declaration, storage,
runtime, linkage, layout, ownership, locking, RCU/refcount, cleanup, callback,
or driver-ABI analogue.  No source correction is required.

## Review dispositions

1. Parity review: no findings.  Accepted after independently confirming every
   direct name/value and the provider re-export.
2. Rust review: no findings.  Accepted: the `c_int` constants and direct alias
   expressions have no unsafe, layout, FFI-boundary, ownership, synchronization,
   panic, or `Drop` behavior.

## Semantic-record closure

- All 210 task-local `SYMBOLS.tsv` rows are complete: two include-guard
  conditionals, the guard macro, and 102 errno definitions for each frozen
  architecture.  Each now records the unconditional source treatment, the
  Rust `c_int` constant (or no-Rust-item guard handling), exact value or alias,
  and the pinned source-lines-2-127 adjudication.
- The S016003 `SCOPE.tsv` semantic-status is `COMPLETE`.  `ABI.tsv`,
  `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, and `BLOCKERS.tsv` have no S016003 rows;
  they are not applicable to this constant-only header.  No semantic condition
  remains pending for this task.
- The destination preserves the upstream UAPI SPDX expression and immutable
  provenance, makes no branding change, and contains no placeholder, Rust test,
  or unsafe code.

All five required task evidence files exist. No compiler, formatter, linker,
test, emulator, debugger, runtime command, or benchmark was run.
