# S012501 implementation

Implemented `src/include/asm-generic/audit_read.rs` as four callback macros
that preserve the upstream initializer-fragment semantics for every frozen
consumer context: x86_64 native, x86_64 IA32, AArch64 native, and AArch64
AArch32 compat.

The macros preserve source order, duplicate-free values, `unsigned` width
(`u32`), and every active `#ifdef __NR_*` result from the pinned syscall
headers. The AArch64 native expansion omits `__NR_readlink`, whose guard is
false there. Consumers retain ownership of their terminating `~0U` sentinel,
matching the C inclusion sites.

Evidence read: `include/asm-generic/audit_read.h`; x86 native/IA32 consumers
and syscall tables; AArch64 native and compat consumers and syscall tables;
the frozen architecture configurations; Phase 0 identity and task manifests.
