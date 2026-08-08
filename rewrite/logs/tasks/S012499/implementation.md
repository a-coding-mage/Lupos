## Implementation attempt 1

Status: BLOCKED.

The pinned source `include/asm-generic/audit_change_attr.h` is not a standalone
translation unit. It is an array-initializer fragment included by architecture
audit C files. Its `#ifdef __NR_*` branches are evaluated in the including
translation unit's syscall namespace; the x86 audit consumers include both
native and compat contexts. The frozen manifests provide no Rust consumer
contract or exact mechanism for reproducing those preprocessor presence tests.

Emitting the entries unconditionally would add syscall numbers absent from a
consumer context. Encoding guessed target `cfg` predicates would change the
Linux selection semantics. No destination source was created because either
choice would be an intentional semantic difference.

Evidence: `vendor/linux/include/asm-generic/audit_change_attr.h`; direct
consumers in `vendor/linux/arch/x86/kernel/audit_64.c` and
`vendor/linux/arch/x86/ia32/audit.c`; selected symbol rows in
`rewrite/SYMBOLS.tsv` remain `PENDING_REVIEW` for every conditional branch.
