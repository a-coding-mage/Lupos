# Applier resolution — S016070 / P02 attempt 1

Outcome: **BLOCKED**.  The sealed candidate is not changed.  The required
source-level C macro/preprocessor behavior has no faithful Rust-context
representation established by the frozen task records or the directly pinned
source context.

## P1 / RUST-1 — extractor macro integer domains

Disposition: **accepted; unresolved source-contract blocker**.

`vendor/linux/include/uapi/linux/bpf_common.h:6,17,22,31,49` defines
`BPF_CLASS`, `BPF_SIZE`, `BPF_MODE`, `BPF_OP`, and `BPF_SRC` as function-like
C macro expansions, not fixed-width functions.  The directly included UAPI
consumer `vendor/linux/include/uapi/linux/filter.h:24-28` supplies
`struct sock_filter.code` as `__u16`; in C that macro operand undergoes the
ordinary integer promotions before the mask expression.  The candidate's
`fn(u32) -> u32` items require a caller cast and replace the macro's contextual
operand/result type behavior.  The frozen `SYMBOLS.tsv` records for these
operative macros remain `PENDING_REVIEW`; neither they nor the pinned context
defines an equivalent Rust macro/typed-integer contract for all selected uses.
The candidate cannot be corrected without a newly specified and independently
reviewed translation mechanism.

Affected findings: parity P1; Rust RUST-1.

## P2 / RUST-2 — BPF_MAXINSNS conditional definition

Disposition: **accepted; unresolved source-contract blocker**.

`vendor/linux/include/uapi/linux/bpf_common.h:53-55` has `#ifndef
BPF_MAXINSNS` around the definition at line 54.  It therefore preserves an
includer's prior macro value.  The candidate's unconditional Rust item always
defines `BPF_MAXINSNS` as `4096` and cannot express that caller-controlled
preprocessing branch.  The frozen mappings do not supply a Rust preprocessor
or configuration-context mechanism that could preserve this public UAPI
override contract.  Source evidence therefore does not permit a faithful
application-stage correction.

Affected findings: parity P2; Rust RUST-2.

## P3 — object-like opcode/field macros

Disposition: **accepted; subsumed by the same unresolved macro-context
contract**.

The values at `bpf_common.h:7-51` are object-like C macros containing
unsuffixed integer literals.  Replacing them with `u32` Rust items changes the
source expression type and removes macro use in preprocessing/contextual C
expressions.  No frozen record establishes a semantics-preserving Rust
representation for that contract.  This independently confirms that a silent
constant-type edit would not resolve the task.

Affected finding: parity P3.

No compiler, formatter, linker, test, runtime command, analyzer, or historical
Lupos source was used.  Any later correction requires a requeued candidate and
two fresh independent reviews.
