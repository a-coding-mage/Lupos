# S014147 application resolution

Applier source review was performed against
`vendor/linux/include/linux/irqreturn.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 and AArch64
configuration identities, the task's candidate and both independent reports,
and the direct IRQ callback/core contexts in `include/linux/interrupt.h`,
`kernel/irq/handle.c`, and `kernel/irq/spurious.c`.  No compiler, formatter,
analyzer, linker, test, or historical Lupos source was used.

## P1 / R1 — resolved by source correction

`IRQ_RETVAL(x)` at `include/linux/irqreturn.h:18` is the C conditional
expression `((x) ? IRQ_HANDLED : IRQ_NONE)`: `x` is evaluated once and then
subject to C scalar truth conversion.  The prior `c_int` parameter incorrectly
narrowed that contract.  `src/include/linux/irqreturn.rs` now defines the
public `IrqRetvalOperand` conversion contract for all Rust representations
needed for C scalar forms (signed/unsigned integer widths, `bool`, `f32`,
`f64`, and raw const/mut pointers).  Its generic `IRQ_RETVAL` function receives
the operand by value, so the caller expression is evaluated once, and returns
`IRQ_HANDLED` exactly when that value is true and `IRQ_NONE` otherwise.  This
preserves the documented boolean conditions (for example `emm_int != 0` at
`drivers/mmc/host/cavium.c:509`) without a pre-truth-test `c_int` conversion.

## R2 — unresolved; task must remain BLOCKED

The candidate aliases `enum irqreturn` and `irqreturn_t` to `c_int`.  The
pinned header supplies only the enumerators `0`, `1`, and `2` and the typedef;
it does not state the enum's ABI representation.  The direct callback
declaration `typedef irqreturn_t (*irq_handler_t)(int, void *)` in
`include/linux/interrupt.h:104`, and the aggregate/result use in
`kernel/irq/handle.c:185-263` and `kernel/irq/spurious.c:135-140`, establish
that the representation must admit the bitwise aggregate value `3`, but do
not establish its size, alignment, signedness, or frozen-target callback ABI.

The relevant authoritative records remain unresolved:

- `rewrite/ABI.tsv`, S014147's four rows for `enum irqreturn` and
  `irqreturn_t` on x86_64 and AArch64;
- `rewrite/LIFETIMES.tsv`, the matching four scalar-value rows.

The available Phase 0 metadata records the pinned Clang 19.1.7 paths/targets
and original compile commands, but provides no enum layout/ABI probe, DWARF,
or other captured result for this type.  The compiler invocation flags are not
independent evidence of the compiler-selected C enum representation.  Under
the frozen-source authority order, the applier cannot replace those
`PENDING_REVIEW` records with a guessed `c_int` ABI.  S014147 is therefore
BLOCKED pending admissible, frozen-target enum ABI evidence and closure of the
matching ABI/lifetime records.
