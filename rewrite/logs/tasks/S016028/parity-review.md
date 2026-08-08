# Parity review — S016028 / attempt 1 / P02

Reviewed independently against pinned `include/uapi/asm-generic/termbits-common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate, candidate
snapshot, and the task's frozen scope/symbol/ABI/lifetime records. No compiler,
formatter, test, analyzer, or historical Lupos source was used.

## Findings

1. **P1 — `TCIOFLUSH` is omitted.**  The selected Linux operative macro at
   `include/uapi/asm-generic/termbits-common.h:64` is `#define TCIOFLUSH 2`.
   It is present in the frozen symbol inventory for both x86_64 and AArch64,
   but the candidate ends with `TCOFLUSH` and provides no `TCIOFLUSH` item.
   This is an externally visible UAPI omission.

2. **P1 — `__ASM_GENERIC_TERMBITS_COMMON_H` include-guard macro and the C
   preprocessor interface are not preserved.**  Lines 2--3 and 66 of the
   pinned header supply the selected `#ifndef`/`#define` guard.  The immediate
   UAPI consumer `include/uapi/asm-generic/termbits.h:4` includes this header
   and relies on that C-header interface.  Replacing every selected macro with
   a Rust `pub const` neither defines the guard nor makes the UAPI macro names
   available to C/preprocessor consumers.  The frozen symbol records classify
   the guard and every value as operative macros on both approved
   architectures.  No source-proven compatibility boundary or generated C
   header exists in the candidate to preserve those semantics.

3. **P1 — typed Rust constants change the macro-expression contract for the
   selected flag and selector macros.**  Linux exposes untyped replacement
   tokens (for example `IGNBRK` at line 9, `CRTSCTS` at line 51, and
   `TCIOFLUSH` at line 64) which participate in C integer promotions and in
   expressions with the `unsigned int` `tcflag_t` declared by the immediate
   generic termbits consumer.  The candidate chooses `i32` for nearly all
   macros and `u32` only for `CRTSCTS`; it therefore cannot be used in the same
   expressions without Rust-side casts and has no evidence for an exact
   context-sensitive mapping.  This is not merely a numeric-value issue: it
   changes the source-level UAPI contract and leaves the required macro/ABI
   mapping unresolved.

## Closure-record state

`semantic-closure-proposal.tsv` for this attempt contains only its header and
no `SC1-*` records.  Consequently no valid semantic-closure key exists for the
findings above; inventing one would make the evidence invalid.  The task needs
a regenerated complete proposal before its findings can be attested through
semantic closure.

## Result

**FINDINGS.**  The candidate is not parity-complete and cannot be accepted.
