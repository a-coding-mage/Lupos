# Application resolution — S014788

## Pinned-source adjudication

I reopened the complete pinned header `vendor/linux/include/linux/rational.h`
at revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, its direct defining
implementation `vendor/linux/lib/math/rational.c`, frozen x86_64 and aarch64
configurations, Kbuild selection, the candidate, and both independent reports.
No compiler, formatter, linker, test, runtime, or diagnostic tool was run.

The header's complete operative content for both configurations is the one
unconditional C declaration:

`void rational_best_approximation(unsigned long, unsigned long, unsigned long,
unsigned long, unsigned long *, unsigned long *);`

The accepted declaration in `src/include/linux/rational.rs` preserves the
same external spelling and parameter order through `unsafe extern "C"`, uses
`core::ffi::c_ulong` for all four C `unsigned long` value arguments, and uses
`*mut c_ulong` for both output arguments.  The frozen targets are LP64
(`CONFIG_64BIT=y`, with `CONFIG_X86_64=y` or `CONFIG_ARM64=y`), so `c_ulong`
is the target C ABI type.  The raw mutable pointers retain C provenance,
nullability, validity, alignment, writable-storage, and aliasing semantics:
they neither fabricate Rust references nor impose ownership, lifetime, or
non-aliasing requirements.  The C definition dereferences both output pointers
unconditionally, in order, and has no `restrict` qualifier; pointers that
alias remain permitted where the underlying C writes are valid.

The header introduces no storage, layout, allocation, ownership transfer,
locking, RCU/refcount, configuration branch, inline arithmetic, or cleanup
behavior.  `lib/math/rational.c` is selected through `CONFIG_RATIONAL=y` for
both frozen configurations and owns the future function definition and
`EXPORT_SYMBOL(rational_best_approximation)` export; this header task must not
duplicate either.

## Review dispositions

1. Parity review: accepted.  Rechecked; no omission or behavior difference
   found.
2. Rust review: accepted.  Rechecked; no FFI, linkage, width/signedness,
   pointer-provenance, aliasing, lifetime, or layout defect found.
3. Pending semantic records: closed for this task by the source facts above.
   The six `SYMBOLS.tsv` records are include-guard boundary records only:
   the guard is unconditional and has no Rust runtime/ABI counterpart.  The
   sole declaration is external C ABI, has no Rust-owned storage or lifetime,
   and has the raw-pointer contract stated above.  No separate S014788
   lifetime or ABI row exists because this header declares no object or layout;
   the relevant ABI facts are resolved here and the exported definition is
   owned by S017291.

No candidate source edit was necessary.  The candidate is accepted as-is.
