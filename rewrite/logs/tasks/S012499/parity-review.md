# S012499 parity review (slot 1)

Verdict: **APPROVE — no parity findings.**

Reviewed the complete pinned `include/asm-generic/audit_change_attr.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against the fresh candidate and
the frozen Phase 0 records.

## Scope and selected contexts

`rewrite/metadata/header_closure.tsv` records exactly two selected consumers
per architecture.  The candidate supplies one storage-free, caller-supplied
macro expansion for each resulting initializer context:

| Context | Pinned inclusion site | Candidate macro | Result |
| --- | --- | --- | --- |
| x86_64 native | `arch/x86/kernel/audit_64.c:23-26` | `audit_change_attr_x86_64_native!` | 18 ordered entries, then caller-owned sentinel |
| x86_64 IA32 | `arch/x86/ia32/audit.c:11-14` | `audit_change_attr_x86_64_ia32!` | 21 ordered entries, then caller-owned sentinel |
| AArch64 native | `lib/audit.c:22-25` | `audit_change_attr_aarch64_native!` | 14 ordered entries, then caller-owned sentinel |
| AArch64 AArch32 compat | `lib/compat_audit.c:22-25` | `audit_change_attr_aarch64_compat!` | 21 ordered entries, then caller-owned sentinel |

Each upstream inclusion is deliberately a reincludable, declaration-free
initializer fragment.  Each candidate macro likewise declares no array or ABI
symbol, invokes its caller-supplied consumer exactly once, preserves the final
comma, and leaves both `unsigned int`-compatible storage and the following
`~0U` sentinel with the consumer.  Explicit `u32` expressions exactly model
the selected targets' `unsigned int` array elements.

## Conditional expansion and order

All 20 inventory conditionals (`__NR_chmod`, `__NR_chown`, `__NR_fchown`,
`__NR_setxattrat`, `__NR_removexattrat`, `__NR_fchownat`,
`__NR_fchmodat2`, `__NR_chown32`, `__NR_link`, and `__NR_linkat`, for each
architecture) were checked against their selected include contexts.  The four
candidate sequences have the same membership and source order as the pinned
fragment.  In particular:

- native AArch64 omits `chmod`, `chown`/`lchown`, `chown32` family, and `link`,
  while retaining `fchown`, both `*xattrat` calls, `fchownat`, `fchmodat`,
  `fchmodat2`, and `linkat`;
- both 32-bit contexts retain the legacy `chown32` trio and their distinct
  `fchownat`/`fchmodat`/`linkat` numbers;
- x86_64 native retains `chmod`, `chown`, `lchown`, `fchown`, and `link`.

The literal values were cross-checked with the pinned x86 syscall tables,
the AArch64 AArch32 syscall table, and `include/uapi/asm-generic/unistd.h`.
No selected branch, initializer element, ordering, conditional membership,
symbol, ABI object, or branding delta is omitted or changed.

Frozen configuration evidence is consistent with all four contexts:
`CONFIG_IA32_EMULATION=y` for x86_64 and `CONFIG_AUDIT_COMPAT_GENERIC=y` for
AArch64; both configurations enable `CONFIG_AUDIT` and `CONFIG_AUDITSYSCALL`.
The candidate provenance SHA and `architectures: common` agree with
`vendor/linux.SHA`, `PHASE0_IDENTITY.tsv`, and task S012499.

No compiler, formatter, build, test, linker, emulator, debugger, or benchmark
was run.
