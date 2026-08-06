# Rust review — S016053

Reviewed `src/include/uapi/linux/arm_sdei.rs` against the complete pinned
`include/uapi/linux/arm_sdei.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Findings

### R1 — Public version macro literal types were changed (must fix)

Linux uses unsuffixed C integer literals for the version shift and mask macros.
On the frozen AArch64 ABI, `SDEI_VERSION_{MAJOR,MINOR,VENDOR}_SHIFT` is `int`;
`SDEI_VERSION_{MAJOR,MINOR}_MASK` is `int`; and
`SDEI_VERSION_VENDOR_MASK` is `unsigned int`.  The candidate publishes all
three shifts as `u32` and all three masks as `u64`.  This changes the exposed
macro value types and contradicts `implementation.md`, which says remaining
unsuffixed decimal values are represented as `i32`.

Preserve the C-literal-width/signedness constants at the public boundary, and
perform any casts required by the fixed-width Rust extraction helpers locally.
Evidence: `vendor/linux/include/uapi/linux/arm_sdei.h:28-37`;
`src/include/uapi/linux/arm_sdei.rs:37-57`.

### R2 — `SDEI_1_0_FN` is not semantically equivalent as a general macro (must fix or explicitly constrain with evidence)

The C function-like macro has no `u32` parameter restriction.  Its result type
and arithmetic follow the usual C conversions between the `unsigned int` base
literal and the caller's expression: for example, an `unsigned long` argument
causes 64-bit addition on AArch64, while a `u32`/`int` argument uses 32-bit
unsigned arithmetic.  The candidate's `const fn SDEI_1_0_FN(n: u32) -> u32`
rejects the former and always wraps at 32 bits.  The statement that it preserves
the macro's wrapping rule is therefore only true for a `u32`-converted
argument, not for the macro as defined.

Either retain the source macro's applicable conversion behavior in the Rust
interface, or record and enforce a frozen-scope restriction proving that this
public macro is only ever instantiated with its in-header `int` literals.  The
derived `SDEI_1_0_FN_SDEI_*` constants themselves are correctly `u32` for those
literal invocations.
Evidence: `vendor/linux/include/uapi/linux/arm_sdei.h:6-26`;
`src/include/uapi/linux/arm_sdei.rs:10-35`.

## Checked without findings

- The pinned header declares no structs, unions, enums, typedefs, function
  declarations, or FFI symbols; consequently no `#[repr(C)]` representation is
  required for this task.
- `SDEI_1_0_FN_BASE`, `SDEI_1_0_MASK`, and the derived function-number
  constants have the correct `u32` type/value for their in-header C expansions.
- The negative return-value macros and the remaining small decimal macros are
  correctly represented as `i32` values.
- Each replacement helper evaluates its input exactly once.  For the actual
  `u64 ver` use in `drivers/firmware/arm_sdei.c:959-979`, the version extraction
  calculations and `u64` result values are otherwise equivalent.

No source, queue, build, formatter, or test action was performed in this
review.
