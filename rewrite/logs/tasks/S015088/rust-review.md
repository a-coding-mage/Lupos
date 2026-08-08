# Rust source review — S015088 (attempt 2, slot 2)

## Scope and evidence

- Pinned source reviewed: `vendor/linux/include/linux/sunrpc/gss_err.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate reviewed: `src/include/linux/sunrpc/gss_err.rs` and this task's
  current `candidate.diff`.
- Frozen task records reviewed for S015088 in `SCOPE.tsv`, `FILE_MAP.tsv`,
  `SYMBOLS.tsv`, `LIFETIMES.tsv`, and `ABI.tsv`.
- Architectures: `common` (x86_64 and aarch64 selection). The sole declared
  C type is `OM_uint32`, an `unsigned int`; the candidate's `u32` preserves
  its 32-bit unsigned value representation on both selected architectures.

## Review result

**APPROVE — no Rust-semantic findings.**

The candidate preserves every selected object-like and function-like macro.
The status masks, offsets, and all shifted status values remain `u32`; their
maximum shift is 24 and their operands are unsigned, so the translation does
not introduce signed promotion, narrowing, overflow, or debug/release panic
behavior. `GSS_C_INDEFINITE` remains the exact `0xffff_ffff` `OM_uint32`
value. `GSS_S_CRED_UNAVAIL` remains an alias of `GSS_S_FAILURE`.

Each function-like macro expands its expression argument once. The candidate
uses `u32` masks and shifts, matching the header's `OM_uint32` status-code
domain and avoiding duplicated side effects. The field macros preserve the
same right shifts and masks. The header has no storage, pointers, references,
allocation, callbacks, FFI declarations, layouts, atomics, interior
mutability, pinning, `Drop`, or `unsafe` blocks; none are introduced. The
public type, constants, and exported declarative macros introduce no C ABI
layout or calling-convention surface for this header-only source.

No compiler, formatter, test, runtime, or diagnostic tooling was used.
