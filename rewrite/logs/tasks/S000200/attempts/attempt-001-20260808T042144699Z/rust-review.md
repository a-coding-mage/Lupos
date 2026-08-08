# Rust source review — S000200, attempt 1, slot 2

Result: FINDINGS

Reviewed the pinned `arch/arm64/include/asm/vncr_mapping.h` against
`src/arch/arm64/include/asm/vncr_mapping.rs` and the leased task records. This
review used source inspection only; no compiler, formatter, test, or runtime
tool was invoked.

## RUST-S000200-01 — all VNCR offsets have changed from C `int` expressions to `usize`

Affected semantic record: `SC1-faad25137ec9a2422cab4334862abaa8de532d501aa8661957f82b8c0fe65ffe`
(`VNCR_VTTBR_EL2`, representative of every VNCR offset macro on source lines
10–113).

The source replacement lists are unsuffixed hexadecimal integer constants.
Each value is representable as a C `int`, so under the frozen AArch64 C ABI an
expansion is a signed 32-bit integer expression. The candidate instead gives
every exported offset an explicit `usize` type. That silently changes the
constant-expression type, signedness, and arithmetic/conversion behavior to a
64-bit unsigned, pointer-sized value. In particular, C integer promotions and
signed arithmetic at a use site do not have the same overflow, comparison,
shift, or mixed-operand behavior as Rust `usize`; callers also cannot retain a
source-compatible 32-bit expression without adding a conversion that the
candidate has made mandatory.

The header itself establishes only that these values are byte displacements; it
does not authorize replacing the C literal type with a pointer-sized unsigned
type. Preserve the source integer-expression semantics and perform any
necessary pointer-offset conversion at the specific use site, with its bounds
and provenance contract established there. This applies uniformly to all 104
`VNCR_*` offsets, not merely the representative macro above.

No ownership, borrowing, pointer dereference, unsafe block, FFI declaration,
layout, allocation, callback, or `Drop` behavior is introduced by this
constant-only candidate. Those areas yield no additional findings.
