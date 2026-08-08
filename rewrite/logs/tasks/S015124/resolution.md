# Resolution — S015124 / attempt 1 / P02

Pinned source reopened: `vendor/linux/include/linux/sys.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## PARITY-001 — active `_LINUX_SYS_H` guard has no candidate mapping

Disposition: **SUSTAINED; BLOCKED**.

The outer `#ifndef _LINUX_SYS_H` / `#define _LINUX_SYS_H` / closing `#endif`
at pinned header lines 2, 3, and 30 is an active C preprocessor contract.  It
defines a macro after first inclusion and suppresses later inclusion of the
header.  The candidate at `src/include/linux/sys.rs` is comment-only and has
no Rust item, module declaration, import boundary, or other frozen mapping
for that contract.

The inactive `#ifdef notdef` region at lines 14--24 is not the blocker: the
frozen x86_64 header closure identifies only `arch/x86/entry/syscall_32.c` and
`arch/x86/entry/syscall_64.c` as selected consumers, the complete pinned
consumer prologues include this header without defining `notdef`, and
`_LINUX_SYS_H` has no other occurrence in the pinned Linux tree.  Thus the
nine obsolete aliases do not supply declarations or ABI surface in the frozen
selected C translations.

The frozen mapping evidence records the source-to-destination file mapping
(`rewrite/SCOPE.tsv` row S015124: `include/linux/sys.h` to
`src/include/linux/sys.rs`) and the two C header consumers
(`rewrite/metadata/header_closure.tsv` and
`rewrite/metadata/task_dependencies.tsv`).  It does not establish a Rust
module/import graph or another Rust representation of the active include
guard.  Per the frozen workflow, shared module indexes are generated only
after all file tasks are DONE; this application stage cannot invent or edit
one, and the sealed candidate may not be silently changed.  Source evidence
therefore cannot establish exact preservation of the active preprocessor
contract within this file task.

The slot-2 approval was considered but is not sufficient to close this
unmapped active contract.  Slot-1's report is likewise not relied upon as
acceptance evidence because its `implementation.md` checksum-touch disclosure
does not provide review output; the disposition follows this independent
source review.

No candidate source was edited, no semantic-closure records were committed,
and no compiler, formatter, linker, test, runtime, or diagnostic tool was
used.  The task must remain BLOCKED until a frozen Rust module mapping and
source-level guard contract can be established without changing the frozen
scope.
