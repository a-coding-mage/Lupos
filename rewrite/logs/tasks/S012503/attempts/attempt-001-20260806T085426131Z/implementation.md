# S012503 implementation

Source: `vendor/linux/include/asm-generic/audit_write.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The leased header is an unguarded, reincludable C preprocessor initializer
fragment.  It first includes `asm-generic/audit_dir_write.h`, then appends
`acct`, optional `swapon`, `quotactl`, optional truncate/ftruncate variants,
optional `bind`, and optional `fallocate` syscall numbers.  It declares no
storage or symbols by itself.

`src/include/asm-generic/audit_write.rs` provides four exported,
caller-supplied macros for the exact frozen inclusion contexts: x86_64 native,
x86_64 IA32, AArch64 native, and AArch64 AArch32 compatibility.  Each macro
flattens the completed included directory-write fragment followed by this
header's selected entries in source order, using explicit `u32` elements.  The
caller retains the array and its following `~0U` sentinel, precisely as the C
inclusion sites do.

The source contexts examined were `lib/audit.c`, `lib/compat_audit.c`,
`arch/x86/kernel/audit_64.c`, `arch/x86/ia32/audit.c`, the corresponding
syscall tables, and the frozen header-closure records.  No compiler,
formatter, build, test, linker, emulator, debugger, or benchmark was run.
