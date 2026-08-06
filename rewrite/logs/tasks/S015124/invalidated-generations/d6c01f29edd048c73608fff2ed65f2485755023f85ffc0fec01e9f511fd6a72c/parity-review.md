# Parity review — S015124

Reviewed `vendor/linux/include/linux/sys.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/sys.rs` for the frozen `x86_64` task scope.

## Result

PASS — no parity findings.

## Source and selection evidence

- The pinned header contains only its include guard and comments in its active
  preprocessor path.  Its nine legacy aliases (`_sys_waitpid`,
  `_sys_olduname`, `_sys_uname`, `_sys_stat`, `_sys_fstat`, `_sys_lstat`,
  `_sys_signal`, `_sys_sgetmask`, and `_sys_ssetmask`) are wholly enclosed by
  `#ifdef notdef` at lines 14–24.
- `notdef` has no definition in the pinned x86 header/source search scope and
  is not a frozen configuration symbol.  The recorded header-closure compile
  command for `arch/x86/entry/syscall_32.o` contains no `-Dnotdef` input.
  Therefore none of the nine aliases enters the selected translation surface.
- `rewrite/SCOPE.tsv` classifies this exact header as `RUST_TRANSLATE` for
  `x86_64`; `rewrite/metadata/header_closure.tsv` records two selected
  consumers, with `arch/x86/entry/syscall_32.c` as the first recorded consumer.
  `rewrite/SYMBOLS.tsv` records the include guard, the `notdef` conditional,
  and the nine macros, all with `NOT_APPLICABLE` ABI status.  The candidate's
  absence of Rust declarations is consequently the faithful mapping of the
  active path, rather than a stub or omission.
- The candidate has the required source, revision, architecture, and task
  provenance; retains the upstream `GPL-2.0` SPDX identifier; introduces no
  symbols, fake behavior, tests, or placeholder constructs.

No source change is requested.
