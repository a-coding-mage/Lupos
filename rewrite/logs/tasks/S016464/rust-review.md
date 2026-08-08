# Rust source review — S016464, attempt 1

Verdict: APPROVE

Reviewed `vendor/linux/include/uapi/linux/virtio_ids.h`, the fresh candidate
`src/include/uapi/linux/virtio_ids.rs`, its candidate diff, and the frozen
task/scope/symbol/Phase-0 records.

- The candidate retains all 47 selected public names and their source values:
  40 `VIRTIO_ID_*` constants and 7 `VIRTIO_TRANS_ID_*` constants.  The gaps in
  the numeric device-ID sequence (14, 15, and 42–44) are preserved rather than
  filled.
- Each C replacement token is an unsuffixed decimal or hexadecimal integer
  literal that fits the 32-bit signed `int` used by both frozen targets.  The
  explicit Rust `i32` constants therefore preserve the source integer width,
  signedness, value, and ordinary arithmetic/shift promotion inputs; no
  truncating cast, wrapping operation, pointer operation, allocation, panic
  path, or `unsafe` block was introduced.
- These object-like macros have no side effects or argument-evaluation
  behavior.  Their Rust constant substitutions preserve their evaluated value;
  C-preprocessor-only expansion is not an operative runtime or ABI mechanism
  in the path-preserving Rust module.
- The source contains no storage, ownership, aliasing, pinning, callbacks,
  synchronization, FFI layout, or Drop behavior.  The candidate introduces
  none.  The BSD-3-Clause SPDX identifier, source provenance, architecture
  scope, task ID, names, and transitional-ID values match the reviewed source.

No source-review findings.
