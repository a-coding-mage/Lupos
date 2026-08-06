# Rust review — S014144

Reviewer: Rust reviewer (Terra, high)

Scope reviewed independently:

- Pinned source: `vendor/linux/include/linux/irqflags_types.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/linux/irqflags_types.rs`.
- Frozen x86_64 and AArch64 configurations, scope/symbol/ABI/lifetime records,
  Kconfig selection context, and header-consumer context.

## Finding R1 — required immutable SPDX provenance is not exact (reject)

The candidate begins with `// SPDX-License-Identifier: GPL-2.0`.  The required
fresh-source provenance template specifies the immutable first line as
`// SPDX-License-Identifier: GPL-2.0-only`.  Correct the candidate’s first
line exactly before acceptance.

Evidence: the project provenance rule and
`vendor/linux/include/linux/irqflags_types.h:1`.

## Conditional and Rust-semantics audit

`struct irqtrace_events` is entirely enclosed by
`#ifdef CONFIG_TRACE_IRQFLAGS` in the pinned header.  That symbol is unset in
both frozen configurations: neither frozen config defines it; the relevant
selectors are also disabled (`CONFIG_PROVE_LOCKING` is unset on both, and
`CONFIG_IRQSOFF_TRACER` is unset where present).  The architecture capability
symbols `CONFIG_TRACE_IRQFLAGS_SUPPORT=y` and
`CONFIG_TRACE_IRQFLAGS_NMI_SUPPORT=y` do not themselves select
`CONFIG_TRACE_IRQFLAGS`; this agrees with `lib/Kconfig.debug`.

Therefore an otherwise item-free Rust module has the correct selected-union
semantics: it contributes no type, field, layout, FFI declaration, linkage, or
visibility.  It creates no zero-sized Rust stand-in for the inactive C struct,
which is correct.  The consumers that name this type do so within their own
`CONFIG_TRACE_IRQFLAGS` regions (for example, the `task_struct` fields in
`include/linux/sched.h`), so no active selected declaration requires a Rust
item from this header.  No ownership, aliasing, unsafe, `Send`/`Sync`, drop,
representation, alignment, or calling-convention issue arises while the only
upstream declaration is inactive.

## Pending-record disposition required at apply

For both architecture rows, the Phase 0 `SYMBOLS.tsv`, `ABI.tsv`, and
`LIFETIMES.tsv` entries for `struct irqtrace_events` remain
`PENDING_REVIEW`.  The applier must close them before `DONE` with the
configuration-derived result: inactive in both frozen builds; no emitted Rust
item or C ABI/layout/linkage contract; no storage, ownership, lifetime,
locking, RCU, or refcount contract.  These are not grounds to synthesize a
placeholder declaration.

No compiler, formatter, test, linker, rust-analyzer diagnostic, or runtime
tool was used.

Verdict: **REJECT pending R1 correction.**
