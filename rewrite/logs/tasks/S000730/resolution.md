# S000730 applier resolution — attempt 1 / P02

## Scope and evidence

This adjudication reopened the complete pinned
`vendor/linux/arch/x86/include/asm/trapnr.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the sealed candidate snapshot,
both independent review reports and attestations, and the frozen task records.
It also reopened direct pinned consumers:

- `vendor/linux/arch/x86/entry/entry_64.S:39,332-347`, which textually
  includes the header and compares `X86_TRAP_BP` in assembler `.if`
  expressions;
- `vendor/linux/arch/x86/boot/compressed/mem_encrypt.S:16,222`, which includes
  the header and uses `$X86_TRAP_VC` as an assembly immediate; and
- `vendor/linux/arch/x86/include/asm/vmx.h:423-430`, which composes each
  `EVENT_TYPE_*` replacement list into the `INTR_TYPE_*` C macro expressions.

The attested candidate snapshot is
`0a1b65da068d7722f44331e4c78c9357524a9b4584cca4046d77f6d71079019d`
(`candidate.diff`); the sealed semantic proposal is
`0e463fa13c7280daf97493bfef6708acfde459be8f73e49252beaded7a511201`.
The destination source was not changed during application. No compiler,
formatter, linker, test, runtime tool, rust-analyzer diagnostic, or historical
Lupos Rust source was used.

## Finding dispositions

### P1 — `EVENT_TYPE_*` and `X86_TRAP_*` macro/token contract

**Disposition: SUSTAINED; unresolved exact translation, blocks the task.**

Pinned `trapnr.h:8-42` supplies object-like C preprocessor replacement lists,
not data objects. The candidate replaces all of them with fixed-typed Rust
`pub const` items. That preserves the displayed numbers but cannot supply
tokens to `entry_64.S` assembler conditionals, the compressed boot assembly
immediate, or `vmx.h` macro composition. The direct consumers above establish
that this is operative selected behavior rather than merely an internal Rust
value catalogue.

The source does not establish a Rust-to-C/assembly macro export mechanism, and
the frozen `ABI.tsv`, `LIFETIMES.tsv`, and task records contain no approved
cross-language header contract. Choosing a generated C header, a macro bridge,
or a Rust-specific replacement interface would be a new, unreviewed design.
The fixed `i32` Rust item also cannot preserve the original replacement-list
use in each C integer context. The candidate therefore cannot be accepted.

### P2 — `_ASM_X86_TRAPNR_H` include guard and selected conditionals

**Disposition: SUSTAINED; unresolved exact translation, blocks the task.**

Pinned `trapnr.h:2-3,44` implements a per-C/assembly-translation-unit
preprocessor guard. `SYMBOLS.tsv` selects `ifndef@2`, the
`_ASM_X86_TRAPNR_H` operative macro, and `endif@44`, all still
`PENDING_REVIEW`. The native consumers above rely on textual inclusion, while
the candidate provides neither a preprocessing guard nor a frozen bridge that
would preserve its native token-stream effect. Rust module idempotence is not
source evidence of equivalence for those C and assembly consumers.

### R1 — Rust item representation and contextual integer conversion

**Disposition: SUSTAINED; same source-evidence blocker as P1 and P2.**

The independent Rust review correctly identifies both language-boundary
defects. In particular, `vmx.h:423-430` shifts the event-type tokens before
later C call boundaries; a Rust `i32` item requires a separately designed
conversion and cannot expand in that C macro expression. Neither the pinned
source nor frozen artifacts establish the exact replacement ABI/interface. No
candidate modification can be made without introducing an unreviewed bridge
and invalidating the sealed candidate and reviews.

## Terminal disposition

`S000730` is **BLOCKED**. Do not prepare or commit a semantic final closure and
do not mark the task `DONE`. The blocker is missing source-proven
Rust-to-C/assembly macro and include-guard integration for the selected native
consumers; it is not a build or test result.
