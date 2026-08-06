# S015088 Rust review

Reviewed `src/include/linux/sunrpc/gss_err.rs` against the complete pinned
`vendor/linux/include/linux/sunrpc/gss_err.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for the common x86_64/AArch64
task scope. No build, test, formatter, or source modification was performed.

## Findings

1. **RUST-1 — high — function signatures narrow the C macro contract and
   change signedness/type behavior.**
   `GSS_CALLING_ERROR`, `GSS_ROUTINE_ERROR`, `GSS_SUPPLEMENTARY_INFO`,
   `GSS_ERROR`, and the three `*_FIELD` helpers are C function-like macros
   (upstream lines 92–99 and 154–159), not `OM_uint32` functions. Their
   argument is evaluated once, but it is otherwise accepted in its caller
   expression type; C's usual arithmetic conversions with the `OM_uint32`
   masks determine the result type. The candidate instead exposes six
   `const fn`s requiring `x: OM_uint32` and returning `OM_uint32`
   (Rust lines 45–63 and 95–108). This rejects signed and wider caller
   expressions that the C macros accept and erases the C conversion/result
   type behavior. For example, a C caller may pass a signed `int` (converted
   for the bitwise operation) or a `u64` expression; the Rust API cannot
   represent either call without a caller-side semantic change.

   The same signedness narrowing occurs for the source macros that have no
   `OM_uint32` cast: context-service flags (upstream lines 42–50), credential
   choices (55–57), display-status kind (62–63), `GSS_S_COMPLETE` (75), the
   three offsets (80–82), and supplementary flags (146–150) are C `int`
   expressions. The candidate declares all of them as `OM_uint32` (Rust
   lines 16–31, 39–41, and 88–92). Their current numeric values are equal,
   but their exposed signedness and expression typing are not. The applier
   must establish the exact Rust-facing representation required by all
   selected consumers rather than treating the status word type as the type
   of every C macro.

2. **RUST-2 — medium — required upstream provenance notice is incomplete.**
   The Rust header retains the University of Michigan notice but drops the
   separate OpenVision Technologies 1993 copyright, permission terms, and
   warranty disclaimer present in the pinned source at lines 12–34. The
   project protocol requires relevant upstream copyright notices to be
   retained. Restore that notice (or record authoritative evidence that it is
   inapplicable) before closing the file.

## Checked without findings

- `OM_uint32` remains a 32-bit unsigned type; all values explicitly cast to
  `OM_uint32` in C retain their `u32` bit patterns, including masks,
  `GSS_C_INDEFINITE`, calling/routine errors, and `GSS_S_CRED_UNAVAIL`.
- Calling, routine, and supplementary bit positions and error categories are
  numerically preserved for `OM_uint32` inputs.
- The source header has no Kconfig or architecture conditional branches;
  omission of its C include guard does not introduce a configuration delta.
- No layout, FFI linkage, ownership, allocation, synchronization, unsafe
  code, or drop behavior is present in this header.
