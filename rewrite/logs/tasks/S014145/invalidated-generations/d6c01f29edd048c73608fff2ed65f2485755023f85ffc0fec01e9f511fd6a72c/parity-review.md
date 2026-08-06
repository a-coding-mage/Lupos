# S014145 parity review — slot 1

Reviewed source-only against pinned `vendor/linux` revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding P1 — the candidate reverses the header's required incomplete-type dependency

`include/linux/irqhandler.h:10-12` deliberately only forward-declares
`struct irq_desc` and then declares `irq_flow_handler_t`.  The accompanying
comment says the dedicated header exists to avoid circular include
dependencies.  In contrast, `src/include/linux/irqhandler.rs:12` imports the
completed descriptor from `crate::include::linux::irqdesc::irq_desc`.

That Rust module does not exist in the fresh tree: its owning frozen task is
`S014140` (`include/linux/irqdesc.h -> src/include/linux/irqdesc.rs`), which is
still `TODO` and is not a declared dependency of S014145.  More importantly,
the pinned C include ordering is the opposite: `include/linux/irq.h:16`
includes `irqhandler.h`, and only later at line 589 includes `irqdesc.h`;
`irqdesc.h:85` itself stores an `irq_flow_handler_t` in `struct irq_desc`.

The candidate therefore does not preserve the forward-declaration boundary
which prevents this cycle, and it introduces an undeclared, unavailable
dependency.  Resolve with the project-wide representation for the C
incomplete `struct irq_desc` that preserves the `*mut irq_desc` ABI without
requiring the completed `irqdesc` translation from this header task.

## Finding P2 — upstream SPDX identifier was changed

The pinned source begins `/* SPDX-License-Identifier: GPL-2.0 */`, while the
candidate uses `// SPDX-License-Identifier: GPL-2.0-only`.  The task permits
no branding delta here and requires retention of the upstream SPDX identifier.
Restore the source identifier exactly.

## Verified parity points

- The only selected semantic item for both frozen configurations is
  `irq_flow_handler_t` at `irqhandler.h:12`; the source has no enums, flags,
  storage, or configuration-dependent branches beyond the include guard.
- `Option<unsafe extern "C" fn(desc: *mut irq_desc)>` has the intended nullable
  function-pointer form: it retains C calling convention, `void` return, and
  non-const descriptor-pointer mutability.  The C typedef can represent a
  null function pointer, while normal IRQ flow-handler call sites (for example
  `irqdesc.h:186-189`) invoke a configured non-null handler.  No discrepancy
  was found in that signature itself.
- The provenance source path, Linux revision, `common` architecture scope, and
  task ID match the frozen S014145 row.  ABI and lifetime records correctly
  identify the function-pointer typedef but remain `PENDING_REVIEW`; those
  records must be closed by application, not treated as evidence of completion.

No compiler, formatter, build, test, rust-analyzer, or runtime tooling was
used.  This report changes no candidate source or queue state.
