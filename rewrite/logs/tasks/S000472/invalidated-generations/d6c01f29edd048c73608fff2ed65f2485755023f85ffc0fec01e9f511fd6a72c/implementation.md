# S000472 implementation

Translated `arch/x86/include/asm/audit.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen `x86_64`
configuration.

The header declares `ia32_classify_syscall(unsigned int)` and five mutable
external incomplete arrays of C `unsigned`.  The function maps to the C ABI
signature `u32 -> i32`.  Rust has no external incomplete-array object type;
each table is therefore declared as a mutable external `u32` at its first
element, preserving the symbol address and C array-decay contract.  The two
frozen consumers are `arch/x86/ia32/audit.c` (definitions) and
`arch/x86/kernel/audit_64.c` (pointer arguments to `audit_register_class`).

No build, formatter, compiler, test, or diagnostics were run.
