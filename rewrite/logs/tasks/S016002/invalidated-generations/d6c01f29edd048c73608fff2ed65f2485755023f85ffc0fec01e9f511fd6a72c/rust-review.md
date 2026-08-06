# Rust review — S016002

Scope reviewed: `src/include/uapi/asm-generic/errno-base.rs` against pinned
`vendor/linux/include/uapi/asm-generic/errno-base.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result

No Rust-semantics, ownership, FFI, layout, integer-conversion, or UAPI-value
finding.

## Evidence checked

- The candidate declares exactly the 34 base errno names from `EPERM` through
  `ERANGE`; each preserves its unconditional upstream value, consecutively
  `1` through `34`.
- Every upstream replacement token is an unsuffixed decimal integer literal
  representable as C `int`.  On both frozen x86_64 and AArch64 targets,
  `core::ffi::c_int` represents that signed C `int` domain, so the explicit
  Rust type preserves the intended signed errno value and does not truncate or
  change a value's sign.
- The file contains only immutable `pub const` declarations: no storage,
  `unsafe`, raw-pointer/reference construction, `extern` ABI boundary,
  layout-bearing type, allocation, panic path, synchronization, or `Drop`
  timing exists to introduce a Rust-specific lifetime or aliasing change.
- `EAGAIN` remains value `11`; the `EWOULDBLOCK` alias is correctly outside
  this source file, in the separate upstream `asm-generic/errno.h` layer.
- The required provenance identifies the exact source, revision, common
  architecture scope, and task.  No test configuration or rejection marker is
  present.

## Finding disposition

None. The candidate is acceptable from the independent Rust review perspective.
