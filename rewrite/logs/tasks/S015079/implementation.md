# S015079 implementation record

Status: BLOCKED — no faithful Rust mapping exists for this header's selected
compile-time C preprocessor contract.

## Verified task context

- Required branch: `feat/bun-like-rewrite-test` (verified before this record).
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Queue row: `S015079`, P01, `IN_PROGRESS`, destination
  `src/include/linux/stringify.rs`, architectures `common`.
- Scope row: `RUST_TRANSLATE`, selected by frozen x86_64 and AArch64 header
  closure evidence (`rewrite/metadata/header_closure.tsv`).
- Destination was absent before this work; no source candidate was created.

## Pinned-source behavior

`include/linux/stringify.h` contains only these operative macros:

1. `__stringify_1(x...)` applies C `#` stringification to its unexpanded
   variadic token argument.
2. `__stringify(x...)` invokes `__stringify_1(x)`, causing C macro argument
   expansion before the inner macro applies `#`. The header's stated example is
   `-DFOO=bar`, where `__stringify(FOO)` becomes the C string literal `"bar"`.
3. `FILE_LINE` adjacent-concatenates the C predefined `__FILE__`, a `":"`
   literal, and the expansion/stringification of C predefined `__LINE__`.

The frozen selected consumers use these expansions as C tokens and literals,
including in compile-time initializers, inline assembly text, attributes,
module aliases, and literal concatenations. Representative selected consumers
include `include/linux/linkage.h`, `include/linux/objtool.h`,
`include/linux/timer.h`, `arch/x86/include/asm/asm.h`, and
`arch/arm64/kernel/probes/kprobes.c`.

## Blocking reason

Rust source has no C-preprocessor phase. A Rust `macro_rules!` macro can call
`stringify!`, but it cannot reproduce C's two-stage rule: macro arguments are
matched as Rust token trees and are not C-pre-expanded before `stringify!`.
It also cannot define the C macro identifiers for C or assembly consumers,
perform C adjacent-string-literal concatenation, or provide the C expansion
site values of `__FILE__` and `__LINE__`.

Consequently, exporting Rust macros/functions/constants/arrays would either
stringify the unexpanded invocation tokens or move the operation into Rust,
changing both expansion context and the C literals consumed by the frozen
source. That would violate the pinned behavior. No faithful destination file
can be written within this one-to-one Rust-header task; a build-boundary design
that preserves this C preprocessor header for C/assembly translation units (or
an explicitly approved equivalent source-generation mechanism) is required.

No compiler, formatter, analyzer, build, test, or historical source was used.
