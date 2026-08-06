# Parity review — S012503 (attempt 2)

Reviewed `src/include/asm-generic/audit_write.rs` against pinned
`vendor/linux/include/asm-generic/audit_write.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64/AArch64
configuration union, the header-closure evidence, and the four selected C
inclusion contexts:

- AArch64 native: `lib/audit.c` (`write_class`)
- AArch64 compat: `lib/compat_audit.c` (`compat_write_class`)
- x86_64 native: `arch/x86/kernel/audit_64.c` (`write_class`)
- x86_64 IA32: `arch/x86/ia32/audit.c` (`ia32_write_class`)

## Findings

1. **P1 — The translated fragment duplicates, instead of composes, its
   selected dependency.** Upstream line 2 reincludes the complete
   `asm-generic/audit_dir_write.h` initializer fragment before adding the
   write-specific entries.  The task's frozen dependency is S012500, but none
   of the four candidate macros invokes the matching `audit_dir_write_*`
   fragment.  Instead, candidate lines 15–17, 31–33, 47–48, and 63–65 repeat
   all of that dependency's architecture-specific entries.  This changes
   source ownership and severs the reincludable-fragment relationship: a
   correction to S012500 cannot be reflected by S012503, and the write-header
   task owns values belonging to the directory-write header.  Preserve the
   upstream inclusion/composition boundary while retaining the selected
   caller-owned `unsigned int` array and its following `~0U` sentinel.

2. **P1 — SPDX identifier was changed.** The pinned header begins
   `SPDX-License-Identifier: GPL-2.0` (upstream line 1); the candidate uses
   `GPL-2.0-only` (candidate line 1).  The rewrite rules require retention of
   SPDX identifiers, so the candidate must preserve the upstream identifier.

## Checks with no finding

- The candidate remains a re-invocable, caller-owned fragment rather than
  creating a global array; each selected C context owns the receiving
  `unsigned`/`unsigned int` array and appends `~0U` itself.
- For the frozen four contexts, the candidate's entry order and `u32` width
  agree with the C expansion.  This includes AArch64 native's emitted duplicate
  `truncate`/`truncate64` and `ftruncate`/`ftruncate64` values, and the selected
  x86_64 and compat conditional entries.
- No branding difference or sentinel insertion/removal was found beyond the
  two findings above.

## Result

**Reject pending correction of both P1 findings.** This was source-only
inspection; no compiler, formatter, linker, test, or runtime command was run.
