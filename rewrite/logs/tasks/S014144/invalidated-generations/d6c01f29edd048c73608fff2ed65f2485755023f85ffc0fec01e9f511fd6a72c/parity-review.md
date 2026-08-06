# Parity review — S014144

Reviewer: parity reviewer (P02, slot 1)  
Scope: `include/linux/irqflags_types.h` → `src/include/linux/irqflags_types.rs`  
Method: manual pinned-source and frozen-configuration review only; no compiler, formatter, test, analyzer, or runtime command was used.

## Evidence reviewed

- The queue row is leased to P02 and is in `REVIEWING`; `rewrite_queue.py verify` reports the frozen queue fingerprint `af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.
- `vendor/linux.SHA` is `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching the candidate's Linux revision and source/task/architecture provenance fields.
- `rewrite/SCOPE.tsv` classifies this common header as `RUST_TRANSLATE`; `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` inventory only its include guards, `CONFIG_TRACE_IRQFLAGS` conditional, and `struct irqtrace_events` for both approved architectures.
- The complete upstream header has no declarations outside `#ifdef CONFIG_TRACE_IRQFLAGS` (lines 5–20). It declares only `struct irqtrace_events`; it has no variable/data definition, `extern`, linkage/export declaration, section annotation, or other macro-generated storage.
- Neither frozen configuration defines `CONFIG_TRACE_IRQFLAGS`; each defines only `CONFIG_TRACE_IRQFLAGS_SUPPORT=y` and `CONFIG_TRACE_IRQFLAGS_NMI_SUPPORT=y`. `lib/Kconfig.debug` makes `TRACE_IRQFLAGS` a separate bool depending on SUPPORT, while the support symbols themselves do not select it. Thus the upstream type is absent from both selected preprocessing results.
- Direct inclusion is through `include/linux/irqflags.h` and `include/linux/sched.h`. The two `task_struct` fields using this type are each inside their own `#ifdef CONFIG_TRACE_IRQFLAGS`; all observed `lockdep.c` uses access those fields and are unavailable without that configuration. Therefore no selected caller needs a Rust declaration, layout, alignment, linkage, or data object.
- Had the guard been selected, the upstream type would require the exact ordered C fields at lines 8–18 (four `unsigned int` event counters and four pointer-width `unsigned long` instruction-pointer fields); that counterfactual layout is not part of either frozen configuration's emitted interface.

## Finding

1. **Low — required immutable provenance SPDX token differs from the project template.**
   `src/include/linux/irqflags_types.rs:1` says `GPL-2.0`, whereas the mandatory fresh-source provenance template in `AGENTS.md` §5 specifies `// SPDX-License-Identifier: GPL-2.0-only`. The candidate otherwise has the required source, revision, architecture, and stable task provenance. The applier should resolve the mandated form while retaining the upstream notice as required by the same section.

## Parity conclusion

Apart from the provenance-format finding above, the candidate faithfully represents the frozen configuration union: it deliberately emits no Rust declaration because upstream emits none when `CONFIG_TRACE_IRQFLAGS` is absent on both x86_64 and AArch64. No missing data, symbol/linkage, section, guard-controlled selected branch, or active type-layout behavior was found.

The applier must also close the task's existing `PENDING_REVIEW` ABI/lifetime records with this configuration-elision evidence before `DONE`.
