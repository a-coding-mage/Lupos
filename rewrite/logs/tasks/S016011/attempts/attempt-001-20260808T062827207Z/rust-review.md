# Rust source review — S016011, attempt 1, slot 2

Review scope: `vendor/linux/include/uapi/asm-generic/mman-common.h` and
`src/include/uapi/asm-generic/mman-common.rs`, with the task's frozen semantic
proposal and direct `asm-generic/mman.h` inclusion context.  This was a manual
source review only; no compiler, formatter, test, runtime, or rust-analyzer
diagnostic was invoked or used.

## FINDINGS

### RUST-S016011-001 — explicit `u32` constants do not preserve the C macro integer type and promotions

Every value macro in the Linux header is an unsuffixed integer constant
expression whose value fits the signed C `int` type on both selected targets.
`PKEY_ACCESS_MASK` is likewise the bitwise-or of two such `int` expressions.
The candidate makes each exported Rust constant `u32` (including the operands
and result of `PKEY_ACCESS_MASK`).  That is a material source-interface change:
the constants no longer compose with signed `int`/`i32` flag arguments without
an explicit conversion, and operations such as complement, comparison, and
mixed-width bitwise expressions take unsigned Rust semantics rather than the
C integer-promotion/usual-arithmetic-conversion path selected by each caller.

The UAPI header provides these macros for generic expression substitution, not
a fixed unsigned-32-bit storage object.  Preserve their signed `int` semantics
(for example, an `i32` representation for these in-range values, with any
context-required conversion made at the translated call site) rather than
assigning a universal `u32` type.  This applies to all value macros in lines
10–18, 21–33, 39, 41–83, and 86–91 of the pinned header, on both selected
architectures.

No ownership, borrowing, pointer provenance, aliasing, pinning, interior
mutability, `Send`/`Sync`, drop, callback/interrupt/RCU/refcount, FFI-layout,
unsafe, allocation, panic, or bounds behavior is introduced by this
constant-only candidate beyond the integer-type finding above.
