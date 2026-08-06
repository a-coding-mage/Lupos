# S012503 parity review (slot 1)

## Result

PASS — no parity findings.

## Source evidence reviewed

- `vendor/linux/include/asm-generic/audit_write.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`;
- its included initializer fragment,
  `vendor/linux/include/asm-generic/audit_dir_write.h`;
- every frozen direct inclusion site recorded by
  `rewrite/metadata/header_closure.tsv`:
  `arch/x86/kernel/audit_64.c`, `arch/x86/ia32/audit.c`, `lib/audit.c`, and
  `lib/compat_audit.c`;
- the corresponding x86 syscall tables, AArch64 generic syscall definitions,
  and AArch64 AArch32 compatibility syscall table; and
- the S012503 scope, task, symbol, ABI, and lifetime records.

## Review

The upstream header is intentionally unguarded and declares neither storage
nor a symbol.  At each inclusion it emits the complete directory-write
fragment first, then `acct`, conditionally selected write syscalls, with the
surrounding C declaration owning both the `unsigned int` array and the final
`~0U` terminator.  The candidate preserves that scope: each caller-supplied
macro emits only `u32` elements, includes the preceding
`audit_dir_write.h`-equivalent prefix, and leaves array creation and the
sentinel to its consumer.

The four fixed expansions exactly match the frozen consumers and retain source
order:

- x86_64 native: 15 directory-write entries followed by
  `163, 167, 179, 76, 77, 49, 285`;
- x86_64 IA32: 15 directory-write entries followed by
  `51, 87, 131, 92, 193, 93, 194, 361, 324`;
- AArch64 native: 7 directory-write entries followed by
  `89, 224, 60, 45, 46, 200, 47`; and
- AArch64 AArch32 compatibility: 15 directory-write entries followed by
  `51, 87, 131, 92, 193, 93, 194, 282, 352`.

This accounts for every conditional in the upstream fragment: native
contexts omit the 64-suffixed truncate variants, while IA32 and AArch32
compatibility retain them.  `u32` is the exact selected `unsigned int`
element width, and the explicit suffixes preserve that element type without
creating an array, sentinel, linkage symbol, or runtime behavior.

The Rust source retains the upstream `GPL-2.0` SPDX identifier and has exact
source path, revision, common architecture membership, and S012503 task
provenance.  It adds no branding difference, test, placeholder, or operative
logic outside this initializer-fragment translation.

No compiler, formatter, linker, test, emulator, debugger, or benchmark was
run.
