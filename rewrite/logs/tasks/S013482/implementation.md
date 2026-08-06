# S013482 implementation

Oracle: `vendor/linux/include/linux/audit_arch.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected common header is translated to `src/include/linux/audit_arch.rs`.
It preserves the seven ordered `auditsc_class_t` values and `AUDITSC_NVALS`.
The `audit_classify_compat_syscall` declaration retains its C ABI, `int` ABI
and syscall unsigned-int width.  The five compat class incomplete-array
declarations retain their exact external symbol names and expose each array's
first `unsigned int` element through its C ABI symbol.

Context inspected: `lib/compat_audit.c`, `arch/x86/ia32/audit.c`,
`arch/x86/kernel/audit_64.c`, `kernel/auditsc.c`, frozen x86_64/aarch64
configs, and the task’s Phase 0 scope/symbol/ABI/lifetime records.
