# Parity review — S015088, attempt 2, slot 1

Status: FINDINGS

Reviewed only the pinned `include/linux/sunrpc/gss_err.h`, the current
`src/include/linux/sunrpc/gss_err.rs`, the task candidate snapshot, and the
frozen task records for both selected architectures. No compiler, formatter,
test, runtime, or historical-source evidence was used.

## Finding P001 — function-like status macros narrow the accepted operand type

Linux symbols: `GSS_CALLING_ERROR`, `GSS_ROUTINE_ERROR`,
`GSS_SUPPLEMENTARY_INFO`, `GSS_ERROR`, `GSS_CALLING_ERROR_FIELD`,
`GSS_ROUTINE_ERROR_FIELD`, and `GSS_SUPPLEMENTARY_INFO_FIELD`.

Pinned local evidence: `include/linux/sunrpc/gss_err.h:92-99,154-159` defines
each operation with an untyped parameter `x` and a mask of type `OM_uint32`.
Consequently, C applies its usual arithmetic conversions to the one evaluated
operand; for example an `int` operand is converted to `unsigned int`, while a
wider unsigned operand retains its wider result type. The candidate's macros
at `src/include/linux/sunrpc/gss_err.rs:63-90,123-141` embed `u32` mask
literals. Rust's `&` and `>>` therefore require the supplied expression to be
compatible with `u32`; they neither admit the C-convertible signed and wider
integer operands nor reproduce the corresponding result-width/conversion
behavior. The occurrence count remains one, so single evaluation is preserved,
but the public macro operand and result contract is narrowed.

Resolution required: retain the pinned C conversion/result behavior for every
selected call context, or establish from the permitted selected callers that
each macro argument is invariably `OM_uint32` and record that exact constraint
before treating the `u32`-specific definitions as equivalent.

## Checked mappings

`OM_uint32` is correctly represented as 32-bit unsigned storage for the two
selected architectures. The translated object-like flags, offsets, masks,
major-status values, supplementary bits, and `GSS_S_CRED_UNAVAIL` alias have
the same numeric bit patterns as the pinned definitions. The seven
function-like macros retain one occurrence of their argument, but P001 blocks
approval because their operand conversion behavior differs.

No unauthorized branding, allocation, locking, lifetime, error-path, or
linkage change is present in this header beyond the macro-visibility/conversion
issue described above.
