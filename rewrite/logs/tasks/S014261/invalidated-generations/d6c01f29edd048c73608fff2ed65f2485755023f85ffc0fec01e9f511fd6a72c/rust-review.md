# Rust review — S014261

Scope reviewed: `src/include/linux/lsm/smack.rs` against the complete pinned
`vendor/linux/include/linux/lsm/smack.h`, with the frozen x86_64 and AArch64
configuration union and the containing `struct lsm_prop` declaration in
`include/linux/security.h` as context. This was a manual source review; no
compiler, formatter, linker, test, or analyzer was run.

## Result

No Rust-specific finding.

## Reviewed Rust and ABI properties

- Both frozen configurations state `# CONFIG_SECURITY_SMACK is not set`.
  Consequently the sole C member, `struct smack_known *skp`, is absent for the
  entire approved architecture union. The forward declaration has no remaining
  Rust representation requirement.
- `#[repr(C)] pub struct lsm_prop_smack {}` accurately represents the selected
  zero-member C aggregate, including its use as the `smack` field of
  `struct lsm_prop`; it neither creates a pointer/reference to the disabled
  `smack_known` type nor changes ownership, aliasing, or drop behavior.
- The candidate introduces no `unsafe`, references, raw pointers, allocation,
  `Drop`, auto-trait override, or executable behavior. There is therefore no
  additional Rust lifetime, provenance, synchronization, panic, or cleanup
  concern in the selected configuration union.
- The type is public and retains the exact Linux-facing type name needed by
  translated consumers. The omitted C include guard is preprocessor-only and
  has no Rust runtime or ABI analogue.

Disposition: accept from the Rust ownership/FFI/layout review perspective.
