# Parity review — S015124 / attempt 1 / P02 / slot 1

Result: **FINDINGS**

Reviewed only the pinned `vendor/linux/include/linux/sys.h`, the frozen x86_64
task/manifests and selected caller context, `src/include/linux/sys.rs`, its
candidate diff, and this task's sealed semantic proposal.  No compiler,
formatter, linker, test, rust-analyzer diagnostic, or historical Lupos source
was used.

## Finding PARITY-001 — active `_LINUX_SYS_H` guard has no candidate mapping

Linux symbol: operative macro `_LINUX_SYS_H`.

The pinned header's outer `#ifndef _LINUX_SYS_H` at line 2, `#define
_LINUX_SYS_H` at line 3, and closing `#endif` at line 30 are active
preprocessor behavior: the first inclusion defines the guard macro and later
inclusions skip the header.  `SYMBOLS.tsv` rows 288635 and 288639 inventory
that conditional and operative macro, and the sealed proposal sets their
completion fields to `COMPLETE`.  The selected x86_64 consumers are
`arch/x86/entry/syscall_32.c` and `arch/x86/entry/syscall_64.c`; each includes
`<linux/sys.h>` in its prologue.  Their recorded frozen compile commands contain
no `-Dnotdef` option.

The candidate contains only provenance/doc comments.  It defines neither an
equivalent item nor a module/import mechanism that carries the Linux guard's
duplicate-inclusion and defined-macro contract.  Its assertion that the
`notdef` aliases are inactive does not account for the separately active outer
guard.  The candidate diff accurately represents that empty file, so this is a
translation omission rather than a snapshot discrepancy.

Required disposition: do not close the semantic records as `COMPLETE` on the
strength of an empty Rust module.  The applier must establish and implement an
exact source-level mapping for this active guard contract, or mark the task
`BLOCKED` if the Rust module system cannot provide that contract without an
unauthorized mechanism.

Semantic-closure keys: `SC1-d72c2e862bcfd0ca1ce27b31c4e06d061c4c81e4e3f11d9472e5186a8ae8d142`, `SC1-a1c9d66ef28da1a5f9f5b544112d7f9db48859d900f0f0e2c731301b86ba7ce1`, `SC1-9a823fcef2b787ad39a9542fdd330293f483e6ece527b57dca2bb26d161cda7b`, and `SC1-25e8f1d710a23d2e5d312218be6e2522e94c0b50b01ef1018483837e7faa6b53`.

## Exhaustiveness notes

- The nine aliases `_sys_waitpid`, `_sys_olduname`, `_sys_uname`, `_sys_stat`,
  `_sys_fstat`, `_sys_lstat`, `_sys_signal`, `_sys_sgetmask`, and
  `_sys_ssetmask` occur solely inside the pinned header's `#ifdef notdef` block
  (lines 14–24).  With no `notdef` definition in either selected command, they
  are inactive for the frozen x86_64 build; their `NOT_APPLICABLE` proposal
  records are not findings.
- The header contains no functions, types, statics, ABI layouts, linkage,
  allocation, locking, lifetime, error, ordering, or cleanup paths beyond the
  preprocessor contracts above.
- Candidate provenance names the pinned Linux path, revision, architecture, and
  task correctly; its SPDX identifier matches the pinned header.  No branding
  delta was found, and the allowlist contains no entry for this header.
