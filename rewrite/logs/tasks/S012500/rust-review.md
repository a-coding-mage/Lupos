# Rust review — S012500

Reviewed `src/include/asm-generic/audit_dir_write.rs` against pinned
`vendor/linux/include/asm-generic/audit_dir_write.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, including all four selected
inclusion contexts: `arch/x86/kernel/audit_64.c`, `arch/x86/ia32/audit.c`,
`lib/audit.c`, and `lib/compat_audit.c`.  This was a source-only audit; no
source was edited and no build, test, formatter, or runtime command was run.

## Result

Accepted. No Rust-semantics findings.

The upstream header is deliberately unguarded and has no C declaration,
storage, type, linkage, or ABI object.  It is a reincludable, conditional,
comma-separated initializer fragment that each caller places in its own
`unsigned`/`unsigned int` array before its own `~0U` sentinel.  The candidate
does not fabricate an array, static, trait, collection, sentinel, or runtime
helper.  Each exported declarative macro invokes the caller-provided macro
once with only the selected ordered `u32` values, retaining the source's
caller-owned storage and terminator boundary.

The fixed expansions accurately cover the frozen caller/macro contexts:

- x86_64 native (`audit_64.c`): 15 values, including the legacy direct-number
  calls and the `*at` group through `renameat2`;
- x86_64 IA32 (`ia32/audit.c`): 15 values with the IA32 syscall numbers;
- AArch64 native (`lib/audit.c`): 7 values, where only the asm-generic
  `mkdirat` conditional group and `renameat2` apply; and
- AArch64 AArch32 compatibility (`lib/compat_audit.c`): 15 values with the
  compat syscall numbers.

Every numeric literal fits the caller's C `unsigned int` element type on both
frozen architectures.  Explicit `u32` spelling prevents signed conversion or
target-width ambiguity; there are no casts, pointer/reference operations,
allocation, `Drop`, synchronization, FFI, panic path, or `unsafe` boundary to
audit.  The macro parameter is an `ident` used strictly in macro-invocation
position, so it cannot introduce a hidden evaluation, borrow, ownership, or
hygiene change.  The separate names keep native and compatibility syscall
sets distinct.  The candidate retains the required immutable provenance and
contains no test configuration, placeholder, artificial static, or trait-based
replacement.
