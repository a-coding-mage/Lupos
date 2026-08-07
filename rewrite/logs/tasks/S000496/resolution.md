# Application resolution — S000496, attempt 1

Manual source adjudication only.  No compiler, formatter, test, linker,
runtime tool, diagnostic service, or historical Lupos Rust source was used.

The pinned source is `arch/x86/include/asm/cpufeatures.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`; this task is frozen to the
`x86_64` configuration.  The task remains **BLOCKED**: the pinned source and
the frozen Phase-0 records do not establish a faithful Rust representation for
the C preprocessor interface described below.  A semantic-closure final/commit
is intentionally not produced: its contract permits only `COMPLETE` or
`NOT_APPLICABLE` decisions, while the affected semantic records remain
unresolved and the task cannot be accepted.

## Finding dispositions

| Findings | Disposition | Pinned evidence and adjudication |
| --- | --- | --- |
| P1 / RUST-002 (`X86_BUG`) | UPHELD — blocker | `cpufeatures.h:524` is the function-like macro `#define X86_BUG(x) (NCAPINTS*32 + (x))`.  It substitutes the caller expression, including its C integer conversions, at each use site.  The candidate's `pub const fn X86_BUG(x: u32) -> u32` imposes a Rust item-call and a fixed argument/result type.  Neither this header, the frozen configuration, nor the task-local source evidence supplies a whole-consumer type/conversion contract or an equivalent Rust macro/module mechanism.  No source-backed edit can prove equivalence, so the finding cannot be resolved. |
| RUST-001 (numeric macro expression types) | UPHELD — blocker | `NCAPINTS`, `NBUGINTS`, and the feature/bug definitions are untyped C preprocessor expressions beginning at `cpufeatures.h:8-9` and `:21`; the candidate gives all of them `u32`.  The pinned header does not constrain their contextual C integer promotions/conversions to `u32`, and its consumer closure is not a Rust type contract.  The all-`u32` representation therefore cannot be accepted as a mechanically equivalent translation. |
| P1 / RUST-003 (`CONFIG_X86_32`) | RESOLVED_CHANGED | The frozen x86_64 configuration contains `CONFIG_X86_64=y` and no `CONFIG_X86_32` definition.  Consequently the source's `#ifdef CONFIG_X86_32` at `cpufeatures.h:535-541` excludes `X86_BUG_ESPFIX` for this task.  The unsupported `#[cfg(feature = "CONFIG_X86_32")]` and `X86_BUG_ESPFIX` item were removed from the destination, leaving the selected x86_64 branch faithful.  This repair does not resolve the macro-interface blockers. |
| P1 / RUST-004 (include guard) | UPHELD — blocker | `_ASM_X86_CPUFEATURES_H` is defined by the `#ifndef`/`#define` sequence at `cpufeatures.h:2-3` and closes at `:578`.  The C source establishes repeat textual-inclusion behavior only.  No pinned source or frozen manifest supplied to this task maps the 2,902 header consumers to a Rust module/import topology or establishes an equivalent include-once contract.  The destination's lack of such a mechanism cannot be accepted as equivalent. |

Because P1/RUST-002, RUST-001, and P1/RUST-004 remain source-evidenced
blockers, this attempt has no valid final semantic record set and must not be
marked `DONE`.
