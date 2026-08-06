# Resolution — S000525

Applier re-opened the complete pinned
`vendor/linux/arch/x86/include/asm/extable_fixup_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its x86 extable consumers, the
frozen task records, and both independent reviews.

## P0-1 / Rust finding 1 — SPDX identifier

Accepted.  The source begins with `SPDX-License-Identifier: GPL-2.0`; the
candidate's `GPL-2.0-only` identifier was not an allowed change.  The Rust
provenance line now retains `GPL-2.0` exactly.

## Rust finding 2 — function-like macro operand semantics

Accepted.  `EX_DATA_REG`, `EX_DATA_FLAG`, and `EX_DATA_IMM` were fixed-`i32`
functions, which prematurely converted any caller operand.  They are now
expression macros whose single parenthesized operand is shifted directly by
the pinned fixed counts (8, 12, and 16).  Thus the caller's integer
conversion and width determine the expression result, as in the source macro;
there is no `i32` parameter/return conversion and no wrapping-shift operation.
The in-header composed encodings invoke those macros with the same signed
integer literals as upstream, preserving their `int` table-word encodings.

No other parity or Rust-safety finding remained.  This constants/macro header
has no ownership, lifetime, locking, RCU, refcount, layout, linkage, or
calling-convention record beyond the resolved symbol mappings.  No build,
formatter, compiler, linker, test, or runtime command was run.
