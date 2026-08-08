S012500 remains BLOCKED at the implementation gate.

The complete pinned source is `vendor/linux/include/asm-generic/audit_dir_write.h`.
It is a C array-initializer fragment whose `#ifdef __NR_*` conditions are
evaluated by each including translation unit.  The frozen header-closure
evidence identifies both `lib/audit.c` and `arch/x86/ia32/audit.c` consumers,
but the task's frozen Rust context contains no exact caller-side macro,
generated syscall namespace, or availability predicate that can preserve the
native/compat distinction.  A fixed Rust array, unconditional identifiers, or
an invented cfg mapping would therefore change syscall-class membership.

No destination source was created and no compiler or build command was run.
