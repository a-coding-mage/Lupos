# Rust review — S016011

Reviewed `src/include/uapi/asm-generic/mman-common.rs` against pinned
`vendor/linux/include/uapi/asm-generic/mman-common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the common x86_64/aarch64
scope.

## Result

Accepted: no Rust-semantics finding.

## Checks

- The candidate defines every non-include-guard object-like macro in the
  source exactly once; no source value or derived value is omitted.  The C
  include guard deliberately has no Rust item analogue.
- Every literal in the header, including `MAP_UNINITIALIZED` and the
  `PROT_GROW*` flags, is representable as a signed C `int` on both frozen
  targets.  The explicit `i32` constants therefore preserve the C literal
  values, signedness, and the relevant integer-operation width without an
  implicit narrowing conversion, overflow, or debug/release difference.
- `PKEY_ACCESS_MASK` remains a computed bitwise OR of the two `i32` source
  operands.  Its value is `0x3`, and it has neither an operator-precedence
  change nor a bit-width/sign-extension hazard.
- This UAPI header contains no structs, unions, enums, pointers, FFI
  declarations, ownership transfers, synchronization, configuration branch,
  allocation, `unsafe`, or panic path.  The candidate adds none.
- The source comments reserving `PROT` and `MAP` bit ranges do not constitute
  exported definitions; retaining only the defined values does not create an
  uninitialised bitfield or expose an invalid Rust enum domain.
- The required immutable provenance identifies the exact source, revision,
  common architecture scope, and task.  No project-authored Rust test is
  present.

No source files were edited and no build, format, test, or runtime command was
run.
