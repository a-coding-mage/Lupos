# Applier resolution — S012500

Reviewed the complete pinned `include/asm-generic/audit_dir_write.h` at Linux
revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, its four selected
inclusion sites, the candidate
`src/include/asm-generic/audit_dir_write.rs`, and both independent review
reports.

## Review dispositions

| Report | Finding | Disposition |
| --- | --- | --- |
| Parity review P1 | Candidate used `GPL-2.0-only` instead of upstream `GPL-2.0`. | Fixed. The provenance SPDX line is now the exact upstream `GPL-2.0` expression. |
| Rust review | No findings. | Accepted. The caller-supplied declarative macros introduce no storage, FFI, ownership, layout, synchronization, panic, or unsafe boundary. |

## Independent applier checks

- The pinned SHA, destination provenance, and task mapping agree on
  `425f94c2954b1fe80ebdbf9b29854e89750355df`,
  `include/asm-generic/audit_dir_write.h`, `S012500`, and the `common`
  architecture scope.
- The upstream header is deliberately unguarded and reincludable: it is a
  conditional comma-separated initializer fragment, not a declaration,
  storage object, or standalone array. Each candidate macro calls its
  caller-supplied macro once, so the translated inclusion site retains
  ownership of its `u32` array and following `~0U` terminator.
- Reopened selected inclusion sites are `arch/x86/kernel/audit_64.c:8-10`,
  `arch/x86/ia32/audit.c:6-8`, `lib/audit.c:7-9`, and
  `lib/compat_audit.c:7-9`. The candidate has exactly their four frozen
  expansions: x86_64 native (15 entries), x86_64 IA32 (15), AArch64 native
  (7), and AArch32 compatibility (15). Each retains upstream source order
  and explicit `u32` width.
- For x86_64, every `#ifdef` in the fragment is selected for both native and
  IA32 inclusion contexts. For AArch64, the eight legacy direct-syscall
  conditionals (`rename` through `mknod`) are absent from the generic native
  context and present in the AArch32-compatible context; `mkdirat` opens the
  complete `*at` group in both contexts, including nested `renameat`; and
  `renameat2` is present in both. The task-local `SYMBOLS.tsv` records are
  closed with this evidence.
- This macro-only header has no task row in `ABI.tsv`, `LIFETIMES.tsv`,
  `DRIVER_ABI.tsv`, or `BLOCKERS.tsv`. The source contains no tests,
  placeholder, artificial ABI object, unauthorized branding, or unsafe code.

The candidate is accepted as the complete fresh translation of the selected
contextual initializer fragment. No build, formatting, test, linker, or runtime
command was run.
