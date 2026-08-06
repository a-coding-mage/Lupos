# S015088 applier resolution

Reviewed the complete pinned source
`vendor/linux/include/linux/sunrpc/gss_err.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate, and both
independent reports for the common x86_64/AArch64 task.

## Finding disposition

1. **P1 / RUST-2 — accepted and fixed.** Restored the complete 1993
   OpenVision Technologies copyright, permission, and warranty-disclaimer
   notice as a Rust comment. The pre-existing Regents of the University of
   Michigan notice remains intact.

2. **RUST-1 — accepted and fixed.** The uncast object-like source macros now
   have Rust `i32` type: the context-service flags, credential choices,
   display-status class values, `GSS_S_COMPLETE`, the three offsets, and the
   five supplementary flags. This matches the C `int` literal and shift
   expressions on both frozen targets. Values explicitly cast to `OM_uint32`
   in the source remain `OM_uint32` (`u32`).

   The seven function-like status macros are represented by generic helpers
   over the sealed `GssStatusCode` integer-category trait. For C categories
   promoted to `int` and then combined with an `OM_uint32` mask (`_Bool`,
   8-/16-bit signed and unsigned integers, and `int`), the helper result is
   `OM_uint32`; 64-bit, pointer-sized, and 128-bit signed and unsigned inputs
   retain their C usual-arithmetic-conversion result category. Each helper
   consumes its input once and performs the original mask/shift expression;
   no input is universally narrowed to `u32`.

## Final semantic records

- `OM_uint32` is C `unsigned int`, represented as `u32` on the frozen
  x86_64 and AArch64 targets.
- This header declares no storage layout, linkage, allocation, ownership,
  locking, RCU, refcount, configuration, or architecture-specific behavior.
- The C include guard has no Rust runtime or configuration analogue. All
  selected operative definitions from upstream lines 37--162 are represented,
  including the `GSS_S_CRED_UNAVAIL` alias.

No compiler, formatter, build, test, runtime, or benchmark command was run.
