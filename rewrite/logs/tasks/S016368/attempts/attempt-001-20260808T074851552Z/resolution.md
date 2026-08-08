# Application resolution — S016368 / P01 / attempt 1

## Result

**REQUEUE REQUIRED — do not enter `APPLYING` or `DONE`.**  The queue remains
unchanged by this application record.  The pipeline coordinator must perform
the project-controlled requeue to implementation, preserving the leased task
identity and frozen scope, then obtain a current candidate snapshot and two
new independent reviews before a subsequent application stage.

The findings establish concrete source corrections and an evidence-binding
failure; they do not establish that the pinned Linux behavior is unknowable.
Accordingly, this is **not `BLOCKED`**.

## Evidence reopened

- Pinned source: `vendor/linux/include/uapi/linux/securebits.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, complete lines 1--83.
- Current candidate: `src/include/uapi/linux/securebits.rs`, complete lines
  1--71.
- Submitted candidate snapshot:
  `rewrite/logs/tasks/S016368/candidate.diff`, complete lines 1--60.
- Review reports: `parity-review.md` and `rust-review.md` in this task
  directory.
- Narrow local context: `vendor/linux/include/linux/securebits.h:5--7` and
  the selected direct uses at `vendor/linux/security/commoncap.c:994` and
  `1394--1396`; frozen `SYMBOLS.tsv` records `issecure_mask` as an operative
  macro for both x86_64 and aarch64.  No task-specific ABI or lifetime row is
  present in `ABI.tsv` or `LIFETIMES.tsv`.

## Finding dispositions

### SC1-001 (parity review) — accepted; requires reimplementation and review

`securebits.h:9` defines the function-like macro exactly as
`issecure_mask(X) (1 << (X))`: a caller-context expression with one evaluation
of `X` and a C `int` left operand.  The current candidate instead exports
`pub const fn issecure_mask(x: u32) -> i32` at lines 13--15 and requires every
in-file use to cast its `i32` constant to `u32` (for example lines 22--23).
That narrows the macro operand contract and changes its source-level
caller-expression mechanism.  The generic wrapper in
`include/linux/securebits.h:7` supplies `X` directly, so the defect is not
disproved by the fixed bit-index uses in this header.

Required requeue work: replace the function mapping with a macro-form mapping
whose expansion preserves the pinned `1 << (X)` mechanism, including one
operand evaluation and the signed 32-bit left operand, and recheck the frozen
selected wrapper and direct consumers.  This is a source-directed correction;
it must be independently reviewed rather than designed or applied in this
application-only pass.

### SC1 (Rust review) — accepted; same corrective requeue as SC1-001

The Rust review independently identifies the same semantic narrowing.  The
pinned macro and local wrapper evidence above confirm it.  The current
`fn(u32) -> i32` cannot be retained as the mapping of this operative macro.
The reimplementation and fresh reviews required for SC1-001 fully cover this
finding; no separate alternative design is approved here.

### SC2 (Rust review) — accepted; evidence snapshot invalidated

The reviewed current candidate has the six-line secure-setting comment at
lines 7--12.  The submitted `candidate.diff` goes directly from the blank line
after provenance to the function at its lines 10--12 and contains none of
those current-source lines.  Thus the task snapshot is not an exact record of
the candidate at review/application time.  Although this particular drift is
comment-only, the Phase 1 evidence requirement is exact candidate binding, not
an inference that omitted lines are harmless.

Required requeue work: regenerate `candidate.diff` from the post-correction
candidate, then run both reviews anew against that snapshot and candidate.
The existing reports must not be used to close a corrected or otherwise
changed candidate.

## Application boundary

No candidate source, queue row, frozen manifest, or other task log was
modified by this application pass.  No compiler, formatter, linker, runtime,
test, benchmark, rust-analyzer diagnostic, or historical Lupos Rust source was
used.
