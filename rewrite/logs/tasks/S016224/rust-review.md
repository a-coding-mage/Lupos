# Rust review — S016224

Reviewer: `rust_reviewer` (gpt-5.6-terra, high)  
Pipeline: `P02`  
Scope reviewed: `src/include/uapi/linux/limits.rs` against pinned
`vendor/linux/include/uapi/linux/limits.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result

No Rust source defect found in the candidate.

## Evidence and audit

* The file preserves the upstream `GPL-2.0 WITH Linux-syscall-note` SPDX
  identifier and has the required immutable source/revision/architecture/task
  provenance.  It contains no conditional compilation, runtime state,
  allocation, unsafe code, FFI item, layout-sensitive type, test code, or
  panic path.
* All thirteen selected object-like macros are present as public Rust constants
  with the exact upstream spelling and values: `NR_OPEN=1024`,
  `NGROUPS_MAX=65536`, `ARG_MAX=131072`, `LINK_MAX=127`,
  `MAX_CANON=255`, `MAX_INPUT=255`, `NAME_MAX=255`, `PATH_MAX=4096`,
  `PIPE_BUF=4096`, `XATTR_NAME_MAX=255`, `XATTR_SIZE_MAX=65536`,
  `XATTR_LIST_MAX=65536`, and `RTSIG_MAX=32`.
* Every upstream replacement token is an unsuffixed decimal integer literal.
  On both frozen architectures its C type is `int`; every value is
  representable in a signed 32-bit `int`.  The candidate's explicit `i32`
  type therefore preserves the literal value, signedness, width, and the
  integer-promotion starting type without overflow or truncation.  There are
  no arithmetic, shift, cast, or evaluation-order expressions in this header.
* These are object-like macros only; their expansion has no preprocessor
  side effects and does not depend on a parameter or caller expression.  A
  `pub const` supplies the same immutable typed value to Rust consumers;
  Rust call sites needing C usual-arithmetic conversion with another integer
  type must express that conversion at their own translated expression, rather
  than changing these `i32` definitions.  No current translated Rust direct
  consumer or module declaration was found.
* The upstream include guard has no Rust conditional branch or runtime/ABI
  payload.  Rust module loading supplies the corresponding once-per-module
  definition behavior.  The source has no configuration predicates, and Phase
  0 selects the same header for both x86_64 and AArch64.
* Direct UAPI consumers confirm the relevant contextual uses are bounds:
  `NAME_MAX + 1` in autofs packet arrays and hashlimit structures, and
  `PATH_MAX` in `xt_cgroup_info_v1`; all fit in `i32` before their translated
  call sites supply the required Rust array-length representation.  `fs.h`
  explicitly undefines `NR_OPEN`, which is a C preprocessor-local visibility
  action and does not alter this source module's definition.

## Required application-stage record closure

`rewrite/SYMBOLS.tsv` still labels the header guard and all thirteen selected
macros `PENDING_REVIEW` for each architecture.  No `S016224` entry exists in
the ABI, lifetime, driver-ABI, or blocker manifests.  This is not a candidate
source mismatch, but the applier must replace/close the task's pending semantic
records with the above determinations before a `DONE` transition, as required
by the Phase 1 protocol.

## Finding disposition

No source change requested.  Candidate is accepted from the Rust ownership,
type, literal, visibility, and macro-semantics review perspective, subject to
the required manifest-record closure by the applier.
