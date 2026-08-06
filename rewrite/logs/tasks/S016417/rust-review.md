# Rust review — S016417

Reviewed `src/include/uapi/linux/thermal.rs` against pinned
`include/uapi/linux/thermal.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the common x86_64/aarch64
scope.

## Result

Accepted: no Rust-semantics finding.

## Checks

- Each of the six ordinary C enum tags is a `#[repr(transparent)]` newtype
  around `core::ffi::c_int`.  On both frozen Linux targets this preserves the
  required signed 32-bit C `int` layout and call ABI.  Keeping the field public
  also permits the explicit integer operations required when translating C
  enum-to-integer conversion, while accepting every incoming `c_int` value
  rather than treating an unrecognised UAPI/netlink value as invalid.
- Every enumerator and each derived `*_MAX` value retains its C value.  The
  derived values are evaluated from their sentinel constants without a
  narrowing cast, overflow, panic path, or release/debug variation.
- The three numeric object-like macros retain C `int` (`c_int`) values;
  `0x1`, `0x2`, `0x02`, and `20` are exact.
- The three string-literal macros are immutable, NUL-terminated `[c_char; N]`
  statics with exact ASCII bytes and lengths 8, 9, and 6.  A caller can obtain
  the C-style pointer through `.as_ptr()`; no owned string, UTF-8 assumption,
  lifetime shortening, or mutable alias is introduced.
- The source header has no Kconfig or architecture conditional definition.
  The candidate adds no `cfg` divergence and supplies the required immutable
  provenance for `common`.
- This declarative UAPI translation has no unsafe blocks, FFI declarations,
  allocation, `Drop`, synchronization, panic/unwrap path, or project-authored
  Rust test.

No source edits were made and no build, format, test, or runtime command was
run.
