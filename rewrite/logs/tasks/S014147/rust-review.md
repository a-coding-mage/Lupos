# Rust review — S014147

Reviewed `src/include/linux/irqreturn.rs` against the complete pinned
`include/linux/irqreturn.h`, the frozen x86_64/aarch64 configuration identity,
and the direct IRQ-core consumers.  This was source inspection only; no
compiler, formatter, test, analyzer, or historical Lupos source was used.

## Findings

### R1 — HIGH: `IRQ_RETVAL` no longer has the C macro's scalar-expression contract

Pinned source defines `IRQ_RETVAL(x)` as `((x) ? IRQ_HANDLED : IRQ_NONE)` at
`vendor/linux/include/linux/irqreturn.h:18`.  The conditional operator accepts
any C scalar expression and performs its normal truth conversion after the
argument has been evaluated once.  The candidate instead exposes a `const fn`
whose parameter is exactly `core::ffi::c_int` (`src/include/linux/irqreturn.rs:29`).
This rejects otherwise-valid translated Boolean and pointer conditions, as
well as scalar values wider than `int`, and would require a caller-side cast
that can truncate before the truth decision.  It is therefore not a faithful
mapping of the operative macro.

This is not merely theoretical header surface: pinned direct uses include
Boolean expressions (`vendor/linux/drivers/mmc/host/cavium.c:509`,
`emm_int != 0`) and integer return/status values, while the selected IRQ core
combines handler results with `retval |= res` at
`vendor/linux/kernel/irq/handle.c:230`.  The applier must select a Rust
representation/call-site translation rule that preserves C scalar truth
semantics without narrowing or double evaluation, and record its exact scope.

### R2 — HIGH: the `c_int` enum ABI assertion is unsupported and leaves the task's ABI records unresolved

The candidate asserts that `enum irqreturn` has the ordinary C `int`
representation on both frozen targets and aliases both `irqreturn` and
`irqreturn_t` to `core::ffi::c_int` (`src/include/linux/irqreturn.rs:8-14`).
The authoritative ABI rows for both architectures remain `PENDING_REVIEW`
for `enum irqreturn` and `irqreturn_t` (`rewrite/ABI.tsv`, S014147 rows), and
the corresponding ownership/lifetime rows are also `PENDING_REVIEW`
(`rewrite/LIFETIMES.tsv`, S014147 rows).  Frozen identity establishes Clang
19.1.7 and the two targets, but it is not ABI evidence for this assertion.

This type is part of an IRQ handler callback ABI:
`typedef irqreturn_t (*irq_handler_t)(int, void *)` in
`vendor/linux/include/linux/interrupt.h:104`; direct core functions also
return and aggregate it (`vendor/linux/kernel/irq/handle.c:185-263`), and
`vendor/linux/kernel/irq/spurious.c:135-140` explicitly converts it to
`unsigned int` before validating aggregate results.  The applier must resolve
and record the exact frozen C enum size, alignment, signedness/representation,
and callback calling-convention compatibility for both x86_64 and aarch64
from admissible Phase 0 evidence before accepting an integer alias.  If that
evidence is unavailable, the task must be BLOCKED rather than asserting it.

## Non-findings

* A Rust enum would be inappropriate: the IRQ core ORs handler values, so the
  valid aggregate `IRQ_HANDLED | IRQ_WAKE_THREAD` must remain representable.
* The constants preserve the source bit values 0, 1, and 2.  They require no
  `unsafe`; the candidate introduces no unsafe block, FFI symbol, or mutable
  state.
* The provenance fields match the queue's Linux path, revision, and `common`
  architecture membership.  No configuration conditional exists in the source
  header beyond its include guard.

## Required resolution

Do not mark S014147 DONE until R1 is corrected and R2's ABI/lifetime records
are closed with pinned-source/Phase 0 evidence.  Each disposition should be
recorded in the task resolution.
