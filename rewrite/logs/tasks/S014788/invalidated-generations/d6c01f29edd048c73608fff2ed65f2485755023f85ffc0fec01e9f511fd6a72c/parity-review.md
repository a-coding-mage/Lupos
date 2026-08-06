# Parity review — S014788

Reviewed `src/include/linux/rational.rs` independently against the complete
pinned `vendor/linux/include/linux/rational.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the direct definition in
`vendor/linux/lib/math/rational.c`, frozen x86_64/AArch64 configurations,
scope/metadata records, and in-tree users. No implementation rationale,
candidate snapshot, other review, resolution, archive, or historical source
was consulted. No compiler, formatter, linker, test, or runtime command ran.

## Evidence checked

- The source header has exactly one unconditional declaration:
  `void rational_best_approximation(unsigned long, unsigned long, unsigned long,
  unsigned long, unsigned long *, unsigned long *);` It has no selected
  configuration branch, type definition, static state, macro behavior, or
  inline implementation beyond its C include guard.
- Both frozen configurations set `CONFIG_RATIONAL=y`; `lib/math/Makefile`
  therefore selects `rational.o` built into `vmlinux.a`. The header-closure
  metadata records the header as `RUST_TRANSLATE` for both x86_64 and aarch64.
- The direct definition uses the identical six-argument signature and exports
  the C symbol through `EXPORT_SYMBOL(rational_best_approximation)`. The
  users pass addresses of `unsigned long` objects for both output arguments;
  the implementation unconditionally stores the resulting numerator and
  denominator through those pointers.
- The candidate provenance identifies the correct source, revision, common
  architecture scope, and task. It retains the source SPDX and copyright
  notice and introduces neither a test nor a configuration-dependent behavior.
- `core::ffi::c_ulong` represents the target C `unsigned long`; x86_64 and
  aarch64 are both selected LP64 targets. `unsafe extern "C"` preserves the C
  calling convention and external symbol name, while `*mut c_ulong` preserves
  the raw, writable output-pointer contract without adding a non-null,
  aliasing, ownership, or lifetime guarantee absent from C.

## Result

No parity findings. The candidate completely and accurately translates the
header's sole externally linked declaration, including parameter order,
width/signedness, raw output pointers, C ABI, and unconditional availability.
