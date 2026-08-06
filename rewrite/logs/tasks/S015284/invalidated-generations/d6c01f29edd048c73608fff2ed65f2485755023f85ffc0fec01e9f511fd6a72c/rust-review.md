# Rust review — S015284

Role: `rust_reviewer`  
Model: `gpt-5.6-terra`  
Reasoning effort: `high`

## Scope inspected

- Pinned source: `vendor/linux/include/linux/uts.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/linux/uts.rs`.
- Frozen x86_64 and AArch64 configurations; both set
  `CONFIG_DEFAULT_HOSTNAME="(none)"`.
- Direct selected consumers recorded by the header closure:
  `init/version.c`, `kernel/utsname.c`, and `kernel/utsname_sysctl.c`.
- S015284 symbol inventory and Phase 0 metadata.

No compiler, formatter, linker, test, or compiler-backed diagnostic was run.

## Findings

### R1 — blocking: `&str` constants do not preserve the C macro string ABI

`UTS_SYSNAME`, `UTS_NODENAME`, and `UTS_DOMAINNAME` are C preprocessor macros
which expand to string literals.  At the selected direct initialization use in
`init/version-timestamp.c`, each expansion supplies a C character array with a
trailing NUL and can undergo the C array-to-pointer conversion required by the
destination `char[]` initialization.  The candidate instead exports Rust
`&str` values.  A Rust `&str` is a UTF-8 fat reference (data pointer plus
length), has no trailing NUL, and cannot stand in for a C string-literal token
or a `char *`/`char[]` initializer without an explicit, separately specified
conversion.  This changes both representation and the valid downstream use
contexts.

Required resolution: provide a source-level representation and consumer-facing
interface that preserves each frozen macro expansion's C bytes, including its
NUL terminator, and its intended C-array/pointer initialization semantics.
Do not expose `&str` as the operative equivalent.

### R2 — blocking: candidate drops each macro's conditional override contract

Lines 8–18 of `include/linux/uts.h` independently guard all three definitions
with `#ifndef`.  Therefore a definition supplied before this header remains
effective.  The candidate unconditionally defines all three Rust constants,
with no conditional/configuration mechanism.  No `-DUTS_*` override was found
in the captured frozen architecture metadata, but absence in these particular
commands does not erase the selected header's independently inventoried
conditional branches or its Linux header contract.  In particular, an
equivalent Rust mapping must make the Phase 0 disposition for these branches
explicit rather than silently replacing them with unconditional items.

Required resolution: account for all six selected `ifndef`/`endif` branches and
the three operative macro definitions in the task's semantic records; preserve
the defined-before-include behavior where it is part of the supported frozen
build interface, or record specific pinned-source evidence that the mechanism
is unreachable for every selected consumer.

### R3 — required completion: S015284's semantic inventory remains pending

All S015284 `SYMBOLS.tsv` entries for its guards and operative macros are
`PENDING_REVIEW`; no S015284 ABI or lifetime entry was present.  The candidate
does not identify the macro expansion representation, linkage (none), or the
absence of a runtime ownership/lifetime object.  The applier must close these
task-specific pending semantic facts before a `DONE` transition, per the Phase
1 gate.

## Other observations

The candidate provenance identifies the correct Linux path, exact frozen
revision, `common` architecture membership, and task ID.  It does not introduce
an FFI item, struct, array, raw pointer, unsafe block, linkage declaration, or
`repr`-sensitive layout; consequently the central Rust issue is the chosen
macro/string representation rather than a missing `repr` annotation.  The
visible payload text (`Linux` and `(none)`) agrees with the pinned header and
the two frozen configurations, but payload equality alone is insufficient for
the representation findings above.

## Verdict

Changes required.  R1 and R2 must be resolved from pinned-source evidence;
R3 must be closed in the task semantic records before final application.
