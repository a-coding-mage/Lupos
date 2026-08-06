# Rust review — S012499

Reviewed `src/include/asm-generic/audit_change_attr.rs` against pinned
`vendor/linux/include/asm-generic/audit_change_attr.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, including the four selected
inclusion contexts: `arch/x86/kernel/audit_64.c`, `arch/x86/ia32/audit.c`,
`lib/audit.c`, and `lib/compat_audit.c`. Review scope was Rust macro-expansion
semantics, target-width representation, source provenance, and artificial
abstractions. No source was edited and no build, test, formatter, or runtime
command was run.

## Result

Accepted. No Rust-semantics findings.

The upstream file is intentionally an unguarded C initializer fragment, not a
type, symbol, or storage declaration. It inserts comma-separated `__NR_*`
values into each consumer's `unsigned`/`unsigned int` array before the
consumer-owned `~0U` sentinel. The candidate faithfully preserves that split
of responsibility: each exported callback macro expands once with only the
ordered `u32` value list, introduces no array/storage/sentinel, and has no
ownership, aliasing, FFI, layout, allocation, panic, synchronization, or
unsafe boundary.

The four fixed expansion contexts resolve the source conditionals correctly:

- x86_64 native has the 18 values selected by `audit_64.c` and the x86_64
  native syscall table;
- x86_64 IA32 has the 21 values selected by `ia32/audit.c` and
  `syscall_32.tbl`;
- AArch64 native has the 14 asm-generic values selected by `lib/audit.c`; and
- AArch64 AArch32 compatibility has the 21 values selected by
  `lib/compat_audit.c` and `arch/arm64/tools/syscall_32.tbl`.

All values fit the C `unsigned int` element type on both frozen 64-bit
architectures and the explicit `u32` spelling preserves the required width
without a signed conversion, truncation, shift, or evaluation-order change.
The callback macros are a direct Rust representation of a context-dependent
initializer fragment; they do not substitute a trait, collection, constant
array, or other production abstraction. Their distinct context names prevent
the native and compatibility syscall-number sets from being conflated.
