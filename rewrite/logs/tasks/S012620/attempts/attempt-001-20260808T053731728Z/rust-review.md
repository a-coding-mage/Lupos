# Rust source review — S012620 attempt 1, slot 2

Review status: FINDINGS

Reviewed only the pinned `include/crypto/dh.h`, the current
`src/include/crypto/dh.rs`, its candidate summary, and the task-owned frozen
manifest/proposal context.  No compiler, formatter, test, historical Rust
source, or runtime evidence was used.

## Finding RUST-COPY-SEMANTICS

Semantic record key: `SC1-e50364836c9e6d0ef20e3178eaec1d9df48f60c898608d8cbbdc45f7f2d52d12`

`struct dh` in C is an ordinary aggregate with no ownership or destructor;
C permits value copies that duplicate all three non-owning pointer values and
the three lengths.  The Rust `dh` also has no `Drop`, but does not implement
`Copy` (and consequently not the corresponding trivial `Clone`).  A Rust
move therefore consumes the source binding, changing the header type's
ordinary value-copy semantics and making a mechanically faithful C aggregate
unavailable to safe Rust callers.  Derive `Copy, Clone` on the `#[repr(C)]`
type; this has no layout or destructor effect and preserves the C shallow-copy
contract.  The pointer lifetime remains caller-owned as documented by
`include/crypto/dh.h:70-77`.

## Checks without findings

- `#[repr(C)]` and declaration order preserve the AArch64 aggregate layout:
  three 8-byte pointers followed by three `unsigned int`/`u32` fields, with
  natural 8-byte aggregate alignment and tail padding.  No packing, union,
  bitfield, or endian conversion is present in the upstream declaration.
- `*const c_void` exactly represents the three `const void *` fields without
  manufacturing references, provenance, aliasing, pinning, or lifetime
  guarantees.  The decode-buffer aliasing contract remains expressed as raw
  pointers, so the binding does not overstate it.
- All four helper declarations retain their C symbol spelling, `extern "C"`
  calling convention, pointer constness/mutability, and `unsigned int`/`int`
  widths (`u32`/`c_int`).  `char *` and `const char *` are only opaque buffer
  pointers at this interface; no Rust character-value interpretation is
  introduced.
- There are no callbacks, interrupt/RCU/refcount protocols, allocation paths,
  bounds operations, arithmetic casts, `Drop` implementations, or explicit
  `unsafe {}` blocks in this header translation.  The `unsafe extern "C"`
  declaration correctly leaves helper invocation unsafe rather than creating
  safe references or safe buffer access.

The reported finding must be resolved before the task can be accepted.
