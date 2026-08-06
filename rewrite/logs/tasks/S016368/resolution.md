# Resolution — S016368

Reviewed the complete pinned source
`vendor/linux/include/uapi/linux/securebits.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` and both independent reports.

1. **Rust review finding 1 — accepted and fixed.** The provenance SPDX line
   now exactly retains `GPL-2.0 WITH Linux-syscall-note`, matching the UAPI
   source expression.
2. **Rust review finding 2 — accepted and fixed.** `issecure_mask` is now a
   function-like `macro_rules!` expansion of `(1_i32 << ($x))`, not a callable
   `const fn`. This preserves the source macro's signed 32-bit left operand,
   parenthesized single evaluation of its expression input, and the shift
   precondition at each expansion site. All selected header uses invoke that
   macro with the source indices 0 through 11.
3. **Parity review — retained.** Its exhaustive source comparison found no
   additional discrepancy; the corresponding constants and aggregate mask
   expressions remain direct macro expansions of the pinned header.

No build, formatter, compiler, linker, test, runtime, or other validation
command was run; this resolution is source-only.
