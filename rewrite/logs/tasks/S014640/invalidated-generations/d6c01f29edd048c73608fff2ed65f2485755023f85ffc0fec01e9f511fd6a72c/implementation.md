# Implementation — S014640

Translated the complete pinned `include/linux/pid_types.h` header to
`src/include/linux/pid_types.rs` from Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header is unconditional in the frozen common x86_64/aarch64 configuration
union; accordingly, its immutable provenance architecture category is `common`.
It declares exactly one five-member C enum and one external mutable
global through an incomplete `struct pid_namespace` declaration.  `pid_type`
is a public `#[repr(C)]` enum with explicit C `int` discriminants zero through
four, preserving the declaration order and the `PIDTYPE_MAX` array-bound
sentinel.  `pid_namespace` remains opaque in this module, exactly matching the
source header's forward declaration rather than importing or recreating the
full layout from `pid_namespace.h`; the external `init_pid_ns` symbol retains
its C linkage and mutable-object contract.

Relevant pinned context inspected: `include/linux/pid.h`, `include/linux/sched.h`,
`include/linux/pid_namespace.h`, and the PID namespace definition/use sites in
`kernel/pid.c` and `kernel/pid_namespace.c`.  Consumers use these ordered enum
values for task-PID selection and PID-type-indexed storage, and use the global
namespace declaration by address.  There are no header-local configuration
branches, functions, allocation paths, locking operations, or ownership
transfers.

Scope, required branch, lease, source/destination paths, frozen Linux SHA, and
queue fingerprint `af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`
were verified before editing.  No compiler, formatter, linker, test, runtime
tool, historical Lupos source, or non-leased source file was used or changed.
