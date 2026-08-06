# Parity review — S014147

Reviewed independently against `vendor/linux/include/linux/irqreturn.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 and AArch64
configurations, Phase 0 task/symbol/ABI/lifetime records, and direct IRQ-core
and consumer contexts. No compiler, formatter, test, or analyzer was run.

## Finding P1 — `IRQ_RETVAL` was narrowed from a C truth-test macro to an `int`-only function

`include/linux/irqreturn.h:18` defines `IRQ_RETVAL(x)` as
`((x) ? IRQ_HANDLED : IRQ_NONE)`.  It evaluates `x` exactly once using C's
truth-value conversion; it is not an `int`-typed function interface.  The
candidate at `src/include/linux/irqreturn.rs:29` instead exports
`pub const fn IRQ_RETVAL(x: core::ffi::c_int)`.  This rejects a faithful Rust
translation of selected source expressions whose C result is a boolean.  For
example, the pinned source contains `IRQ_RETVAL(emm_int != 0)` in
`drivers/mmc/host/cavium.c:509`, `IRQ_RETVAL(nr_serviced > 0)` in
`drivers/net/ethernet/8390/lib8390.c:507`, and
`IRQ_RETVAL((status & mask) != 0)` in `drivers/gpio/gpio-nomadik.c:301`.
C supplies an `int` for those comparisons, but a direct Rust expression is
`bool` and cannot be passed to the candidate function.  The same macro is
also intentionally generic over other scalar truth-tested expressions.

This is material because Phase 0 selects the macro for both architectures
(`rewrite/SYMBOLS.tsv` rows 197764 and 197770), and the header is selected by
both frozen configurations.  Preserve its single-evaluation truth-test
semantics at every selected callsite/type rather than exposing an API narrowed
to `c_int`; document any necessary Rust representation strategy and ensure it
does not silently change accepted operand forms.

## Verified parity points

- The three values retain their pinned C `int` values: `IRQ_NONE = 0`,
  `IRQ_HANDLED = 1`, and `IRQ_WAKE_THREAD = 2`.
- Mapping the return storage to an integer admits the valid aggregate value
  `IRQ_HANDLED | IRQ_WAKE_THREAD` (`3`).  This aggregate is required by
  `kernel/irq/handle.c:185-239` (`retval |= res`) and is inspected with both
  bitwise and equality operations in `kernel/irq/spurious.c:135-351`.
- The candidate retains the typedef spelling `irqreturn_t`, no configuration
  branch is omitted from the header, and provenance matches the frozen Linux
  revision.

## Review disposition

Rejected pending resolution of P1.  The applier must also close the Phase 0
`PENDING_REVIEW` ABI/lifetime records for `enum irqreturn` and `irqreturn_t`
with source-backed representation rationale for both frozen targets.
