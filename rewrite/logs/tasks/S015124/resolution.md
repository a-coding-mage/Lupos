# Resolution — S015124

## Outcome

Accepted without source modification.  `src/include/linux/sys.rs` faithfully
maps the empty active surface of pinned `include/linux/sys.h` for x86_64.

## Independent source adjudication

- The source is at pinned revision `425f94c2954b1fe80ebdbf9b29854e89750355df`,
  matching `vendor/linux.SHA` and the candidate provenance.
- The only declarations in the header are the nine legacy aliases on lines
  15–23, wholly nested beneath `#ifdef notdef` (lines 14–24).  A source and
  frozen-context search found no lowercase `notdef` definition, undefinition,
  or `-Dnotdef`/`-Unotdef` command option.  The lowercase token occurs in the
  pinned source only at this guard; unrelated uppercase `NOTDEF` occurrences
  do not affect C preprocessor identifier case.
- The frozen `x86_64` header-closure record identifies exactly two Rust
  consumers, `arch/x86/entry/syscall_32.c` and `arch/x86/entry/syscall_64.c`,
  and records their x86 Kbuild command context.  Both directly include this
  header; neither consumer nor its frozen command context defines `notdef`.
  Thus all nine aliases are inactive for every selected consumer.
- `_LINUX_SYS_H` is a C include guard only.  With the `notdef` region removed
  by preprocessing, the source header contributes no type, value, linkage,
  layout, ownership, synchronization, ABI, or runtime contract.  The empty
  Rust declaration surface is therefore the required mapping, not a stub.

## Review dispositions and semantic-record closure

- Parity review: accepted.  Its inactive-branch and selected-consumer evidence
  agrees with the independent source inspection above.
- Rust review: accepted.  An empty module adds no ownership, unsafe, FFI,
  layout, panic, or drop behavior.
- `SYMBOLS.tsv` pending conditional records `ifndef@2`, `ifdef@14`,
  `endif@24`, and `endif@30` are resolved for this task: the include guard is
  preprocessor-only, and `ifdef@14` is false in every frozen selected context.
- The pending `_LINUX_SYS_H` record is resolved as a non-exported,
  C-preprocessor-only guard.  The pending `_sys_waitpid`, `_sys_olduname`,
  `_sys_uname`, `_sys_stat`, `_sys_fstat`, `_sys_lstat`, `_sys_signal`,
  `_sys_sgetmask`, and `_sys_ssetmask` records are each resolved as inactive
  aliases with no selected Rust representation or ABI requirement.
- There are no S015124 rows in `LIFETIMES.tsv`, `ABI.tsv`, or
  `DRIVER_ABI.tsv`; no additional task semantic record remains to close.

No compiler, formatter, linker, test, or runtime command was used.
