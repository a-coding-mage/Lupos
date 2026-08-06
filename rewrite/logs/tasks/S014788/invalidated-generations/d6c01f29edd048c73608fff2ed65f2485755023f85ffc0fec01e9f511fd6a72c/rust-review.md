# Rust review — S014788

Verdict: ACCEPT; no source-level Rust ownership, FFI, layout, or linkage defect found.

## Evidence examined

- Pinned `include/linux/rational.h` declares `rational_best_approximation` with
  four `unsigned long` values and two mutable `unsigned long *` output
  pointers, and no conditional declaration or additional ABI attribute.
- The candidate declares the same symbol in an `unsafe extern "C"` block with
  four `core::ffi::c_ulong` values and two `*mut c_ulong` pointers.  This keeps
  the C external symbol spelling and C calling convention, and makes invocation
  unsafe without creating a Rust reference or uniqueness/lifetime guarantee.
- Both frozen configurations set `CONFIG_64BIT=y`; x86_64 also sets
  `CONFIG_X86_64=y` and aarch64 sets `CONFIG_ARM64=y`.  Their pinned C headers
  define the relevant Linux `unsigned long` family in terms of `unsigned long`.
  `c_ulong` is therefore the target C type for both approved LP64 targets.
- The pinned implementation in `lib/math/rational.c` writes both output
  pointers unconditionally and contains no `restrict` qualifier.  The raw
  mutable-pointer parameters correctly retain Linux requirements for valid,
  writable, suitably aligned storage through the call while permitting the
  pointers to alias; null or dangling pointers remain invalid exactly as in C.
- Selected direct users are C driver objects plus the future
  `lib/math/rational.c` translation.  The implementation exports
  `rational_best_approximation`; the candidate’s declaration does not invent a
  wrapper, ownership transfer, or competing Rust symbol.

## Record closure required before DONE

`rewrite/SYMBOLS.tsv` still has the six S014788 header-guard records marked
`PENDING_REVIEW` (three per architecture). `rewrite/LIFETIMES.tsv` and
`rewrite/ABI.tsv` have no S014788 row. The applier must close the required
symbol, ABI, and lifetime/provenance records from the above pinned-source
evidence before marking the task `DONE`; this is a workflow-record obligation,
not a defect in `src/include/linux/rational.rs`.

No compiler, formatter, linker, test, or runtime tool was used.
