# S016215 parity review (slot 1)

Reviewed the complete pinned source
`vendor/linux/include/uapi/linux/kernel-page-flags.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/kernel-page-flags.rs` and the frozen common
x86_64/AArch64 scope, symbol, ABI, lifetime, and header-closure records.

## Result: PASS — no parity findings

- The candidate retains the exact UAPI SPDX identifier, pinned source path,
  revision, `common` architecture scope, and task ID.  It introduces no
  branding delta, test code, configuration-dependent surface, or declaration
  outside the scoped header.
- The complete source has exactly 27 exported `KPF_*` object-like macros and
  no types, functions, storage, linkage, ownership, locking, cleanup, or
  macro expressions.  The candidate exports exactly the same 27 public names,
  in source order: `KPF_LOCKED` through `KPF_BUDDY` are 0--10,
  `KPF_MMAP` through `KPF_NOPAGE` are 11--20, and `KPF_KSM` through
  `KPF_PGTABLE` are 21--26.  A direct name/value comparison found no missing,
  extra, reordered, or changed constant.
- Every source literal is an unsuffixed decimal integer literal, hence has C
  `int` type on both frozen targets.  The candidate uses `i32` for every
  constant, preserving this signed integer-constant surface; this is also the
  form consumed by the selected `fs/proc/page.c` uses such as
  `1 << KPF_PGTABLE` and `kpf_copy_bit(..., KPF_LOCKED, ...)`.
- `KPF_ERROR` retains its ``Now unused`` status.  The 2.6.31 grouping comment
  is retained as documentation only and does not alter the exported UAPI
  values.  The C multiple-inclusion guard has no Rust module counterpart.
- Mechanical records select this UAPI header for both frozen architectures as
  a built-in header dependency of `fs/proc/page.o`; the kernel-facing wrapper
  `include/linux/kernel-page-flags.h` includes it directly.  This confirms the
  candidate covers the intended UAPI provider rather than the separate,
  non-UAPI kernel-only flags at values 32--42.

`rewrite/SYMBOLS.tsv` retains 60 Phase-0 `PENDING_REVIEW` entries for this
task (the three guard records plus 27 macros for each architecture).  Before
`DONE`, the applier must close them with the facts above: all KPF macros are
unconditional signed C `int` literals with the stated values; the include
guard has no Rust value surface; and this passive constants-only header has no
ABI layout or ownership/lifetime record to resolve.  This is a record-closure
requirement, not a candidate source defect.

No compiler, formatter, build, test, emulator, debugger, benchmark, source,
manifest, or queue edit was performed by this reviewer beyond this required
review artifact.
