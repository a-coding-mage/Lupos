# Rust review — S016252

Reviewed `src/include/uapi/linux/mptcp_pm.rs` against the complete pinned
`include/uapi/linux/mptcp_pm.h` for both frozen targets.  No build, formatter,
or test command was run.

## Findings

### R1 — `MPTCP_PM_NAME` does not preserve the C macro's pointer expression semantics (must fix)

`MPTCP_PM_NAME` in the UAPI is the function-like replacement expression
`"mptcp_pm"` (source line 10): in ordinary C expression contexts it has an
array-of-`char` string-literal type and decays to `const char *`, including the
terminating NUL.  The candidate instead exposes a Rust `pub static
MPTCP_PM_NAME: [c_char; 9]`.  Naming that static is an array place/value, not a
`*const c_char`; no implicit array-to-pointer decay exists in Rust.  Thus a
future faithful equivalent of the pinned `struct genl_family` initializer in
`net/mptcp/pm_netlink.c:631` cannot use `MPTCP_PM_NAME` as the C source does;
it must add a conversion at every use.  The candidate comment claiming pointer
expression support is consequently false.

Keep the exact NUL-terminated bytes, but expose the operative macro through a
pointer-form representation (with backing storage whose lifetime is static),
or a deliberately documented macro-equivalent interface that supplies
`*const c_char` directly.  Do not export a mutable string.  The applier must
also record the chosen pointer/lifetime contract in the task ABI/lifetime
resolution.

### R2 — named C enum tags collapse into `c_int` aliases (must fix)

The header declares two distinct named enum types, `enum mptcp_event_type`
(line 44) and `enum mptcp_event_attr` (line 110).  The candidate maps each to
`pub type ... = c_int`, so both names disappear from Rust's type system and
values of either family can be passed interchangeably.  Although the frozen
x86_64/AArch64 ABI uses an `int`-sized enum representation for these values,
the tag is an operative C API type, not merely a documentation label.

Represent each named enum with a distinct ABI-preserving transparent/newtype
integer representation that accepts every valid C integer bit pattern, and
give its constants that type.  Do not use a closed Rust `enum`, because
netlink values may be unknown/out of range and C permits such integer values.
The anonymous enum members correctly remain integer constants.

## Checked without additional findings

- All explicit enum values, implicit increments, gaps, source-order sentinel
  values, and each public `...MAX = sentinel - 1` expression are present with
  `c_int` arithmetic.
- The string byte sequence is `mptcp_pm\\0` and has nine `c_char` elements;
  its signed element type is harmless for these ASCII/NUL bytes on both frozen
  architectures, but R1 still requires pointer-form access.
- This generated UAPI header has no selected configuration branches.  The
  candidate adds no architecture-specific `cfg` behavior.
- No structs, unions, bitfields, function ABI, ownership transfer, mutable
  statics, unsafe blocks, `Drop`, tests, stubs, or unauthorized branding are
  involved.  Provenance and SPDX identifier match the pinned header.

## Verdict

Reject pending R1 and R2.  These are source/ABI corrections for the applier;
this reviewer made no source edits.
