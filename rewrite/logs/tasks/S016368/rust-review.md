# Rust review — S016368 (slot 2)

Reviewed `src/include/uapi/linux/securebits.rs` against the complete pinned
`vendor/linux/include/uapi/linux/securebits.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for the common x86_64/AArch64
scope.  This was source-only review: no build, formatter, test, or runtime
command was run, and no source was edited.

## Findings

1. **High — the upstream UAPI SPDX identifier was changed.**  The pinned
   source's first line is `SPDX-License-Identifier: GPL-2.0 WITH
   Linux-syscall-note`; candidate line 1 instead says `GPL-2.0-only`.  This
   drops the syscall-note exception from a UAPI header, contrary to the
   required retention of upstream SPDX identifiers.  Restore the source
   expression exactly.

2. **High — `issecure_mask` is a C function-like macro, but the candidate
   substitutes a narrow callable Rust API.**  Upstream line 9 expands
   `(1 << (X))` at each caller; the left operand is C `int`, while the
   argument is not declared as an `int` parameter and can be any integral
   expression accepted by the C shift rules.  Candidate lines 13–14 instead
   publish `pub const fn issecure_mask(x: i32) -> i32`.  That rejects caller
   expressions of other integral types (for example an unsigned securebits
   value), introduces a callable/function-pointer-capable Rust item where the
   source has no function or ABI, and gives out-of-range shift arguments the
   Rust checked-shift failure behavior rather than the C macro's shift
   preconditions/undefined behavior.  The current derived constants use only
   indices 0 through 11 and therefore retain their numeric values, but that
   does not preserve the selected operative macro's general contract.  Model
   the macro expansion without a new narrowed safe function, or establish and
   record an exact constrained Rust-facing contract for every selected caller
   before acceptance.

## Checks without additional findings

- Every object-like source macro other than the include guard is represented
  once with its source spelling.  `SECUREBITS_DEFAULT`, all six setting/lock
  indices, all twelve `SECBIT_*` masks, `SECURE_ALL_BITS`,
  `SECURE_ALL_LOCKS`, and `SECURE_ALL_UNPRIVILEGED` have their correct values
  for the header's supplied indices.  The source's unsuffixed literals and
  the resulting masks are C `int` values on both frozen targets; the
  candidate's `i32` object constants preserve that width and signedness.
- The aggregate values are within signed 32-bit range (`0x555`, `0xaaa`, and
  `0x500`), so their displayed constant expressions introduce no overflow,
  truncation, or signed-shift issue for the supplied values.
- The header has no structs, unions, extern declarations, storage, mutable
  state, ownership/aliasing contract, synchronization, architecture/Kconfig
  branch, or `unsafe` boundary.  Omitting the C textual include guard itself
  is appropriate for a Rust module; no layout or C-linkage item needs a Rust
  ABI annotation.
- Immutable provenance path, revision, common architecture scope, and task ID
  match the frozen queue and `vendor/linux.SHA`.  No Rust test configuration,
  placeholder, panic macro, allocation, or explicit unsafe code was added.

## Disposition

Changes are required before source acceptance.  The applier must resolve both
findings using pinned-source and task-scoped ABI/consumer evidence.
