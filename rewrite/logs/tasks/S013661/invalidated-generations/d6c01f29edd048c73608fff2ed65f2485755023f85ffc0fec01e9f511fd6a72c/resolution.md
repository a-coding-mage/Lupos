# S013661 applier resolution

Applier: `gpt-5.6-terra`, high (source-only)

## Evidence reopened

- `vendor/linux.SHA` records pinned revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The complete `vendor/linux/include/linux/crc32poly.h` defines three
  object-like, unsuffixed hexadecimal integer-literal macros.
- Frozen scope and header-closure evidence select this header for both
  x86_64 and aarch64 through `lib/decompress_bunzip2.o`.  The selected Rust
  consumer is task `S017216`.
- In the complete `start_bunzip` CRC-table initialization in
  `vendor/linux/lib/decompress_bunzip2.c`, `c` is `unsigned int` and the
  `CRC32_POLY_BE` macro expansion is the right operand of `^` with
  `(c << 1)`.  The C usual arithmetic conversions therefore convert its
  standalone `int` literal to `unsigned int` at that use.
- The candidate and both independent reports were reopened.  No compiler,
  formatter, analyzer, build, test, debugger, or runtime command was used.

## Finding dispositions

| Finding | Disposition |
| --- | --- |
| P1 / F1: fixed `i32` Rust constant does not provide the C use-site unsigned conversion | Accepted.  `0x04c11db7` is an `int` literal in the source header on both frozen targets, while its selected XOR context is `unsigned int`.  The candidate preserves only the standalone literal type, not the macro expansion’s contextual C conversion. |

## Constants and provenance

The candidate values exactly match the three Linux literals.  `CRC32_POLY_LE`
and `CRC32C_POLY_LE` have the required 32-bit `unsigned int` literal type;
`CRC32_POLY_BE` has the standalone 32-bit `int` literal type.  SPDX,
immutable provenance, task ID, Linux revision, architecture set, public names,
and the absence of branding changes are all correct.  The include guard has no
separate Rust runtime behavior.  This header has no ownership, layout, ABI,
locking, allocation, RCU, refcount, or cleanup record to close.

## Blocking decision

No exact header-local Rust mapping has been established for the C object-like
macro’s general expression behavior.  Keeping the candidate as `i32` requires
a caller-specific cast in the selected `u32` XOR; changing it to `u32` would
erase the source macro’s standalone `int` type.  A Rust macro that re-emits an
unsuffixed literal would instead acquire Rust inference/defaulting rules, not
the C macro’s fixed `int` type followed by C usual arithmetic conversions.

Neither a caller-specific cast nor a scalar-type change is justified as a
general mapping within this header task.  The exact contextual behavior must
be decided together with the selected caller’s full C-to-Rust expression
translation or by a scoped semantic decision.  The task is therefore
`BLOCKED`; no source edit is applied and no `PENDING_REVIEW` semantic item is
claimed closed for a `DONE` transition.
