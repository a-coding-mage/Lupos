# Parity review — S016002 (slot 1)

Scope reviewed: `vendor/linux/include/uapi/asm-generic/errno-base.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its direct UAPI inclusion context
in `include/uapi/asm-generic/errno.h`, the frozen task manifests, and the
candidate at `src/include/uapi/asm-generic/errno-base.rs`.  This was manual
source inspection only; no compiler, formatter, test, analyzer, or historical
Lupos source was used.

## Findings

### P1 — selected C macro interface has no source-proven Rust representation

Linux symbols: `_ASM_GENERIC_ERRNO_BASE_H`, `EPERM` through `ERANGE`.

Evidence: the pinned header lines 2–3 uses `_ASM_GENERIC_ERRNO_BASE_H` as a C
preprocessor include guard, and lines 5–38 define each errno identifier as an
object-like preprocessor macro.  The frozen `SYMBOLS.tsv` selects the guard and
every errno macro for both `x86_64` and `aarch64`, all with
`PENDING_REVIEW`.  `include/uapi/asm-generic/errno.h:5` consumes this header by
C `#include`, then continues to use its macro namespace (for example
`EWOULDBLOCK EAGAIN` at line 29).

The candidate replaces those tokens with Rust module constants and only a
comment claiming that module behavior corresponds to the include guard.  A
Rust `pub const` neither participates in C preprocessing nor establishes the
same token-substitution/redefinition/include-order behavior, and the frozen
ABI/lifetime manifests contain no completed Linux-facing macro-export,
generated-binding, or header-generation contract that could bridge this
difference.  The bare decimal literals do have the expected positive values,
and `i32` matches the ordinary C `int` width on the approved architectures,
but that does not establish the selected macro/guard interface.  Exact parity
therefore cannot be accepted from the pinned source and frozen records.

### P1 — candidate evidence is not a candidate snapshot/diff

Linux symbols: all selected symbols in `errno-base.h`.

Evidence: `rewrite/logs/tasks/S016002/candidate.diff` contains a prose claim
rather than a diff or focused source snapshot.  It does not bind the reviewed
candidate lines to the implementation transition, so an independent reviewer
cannot use the required candidate artifact to establish which source revision
was offered for review.  This prevents completion of the source-review
evidence chain independently of the interface issue above.

## Disposition

`FINDINGS`.  The candidate must not be accepted as parity-complete.  The
applier needs either a frozen, source-proven mechanism preserving the selected
UAPI macro/include contract and a real candidate snapshot, or must block the
task rather than guess.
