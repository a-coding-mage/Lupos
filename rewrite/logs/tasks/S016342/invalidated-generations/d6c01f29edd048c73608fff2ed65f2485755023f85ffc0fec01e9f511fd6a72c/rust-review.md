# Rust review — S016342

Reviewer: `rust_reviewer` (slot 2)  
Scope: `vendor/linux/include/uapi/linux/psample.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` versus
`src/include/uapi/linux/psample.rs`.

## Finding R1 — string-literal macros were changed into reference expressions

**Severity: major.**

`PSAMPLE_NL_MCGRP_CONFIG_NAME`, `PSAMPLE_NL_MCGRP_SAMPLE_NAME`, and
`PSAMPLE_GENL_NAME` in the C header (lines 66–68) each expand to a string
literal: an array expression with the terminating NUL.  The candidate exposes
each as `&[u8; N]` (lines 92–94), which is a Rust reference expression rather
than an array-literal value.  This changes the direct aggregate-initializer
form and requires a dereference/copy for an array field.  The selected-header
context demonstrates the distinction: upstream `net/psample/psample.c:111`
initializes the `genl_family.name` character array directly with
`PSAMPLE_GENL_NAME`; the candidate constant cannot directly initialize an
equivalent Rust `[u8; N]` field.

Represent the macro payloads as byte-array values (including the NUL), or
provide a mechanism whose expression behavior preserves both direct array
initialization and pointer coercion at each translated use.  Do not silently
turn this UAPI macro family into borrowed-slice API values.

## Checked items

- The anonymous attribute enum's enumerators are C `int` constant expressions;
  the candidate maps their values `0..17`, including `__PSAMPLE_ATTR_MAX`, to
  signed 32-bit constants.  `PSAMPLE_ATTR_MAX` remains the derived value 16.
- The two tagged C enum domains have `int`-sized values under the frozen
  commands (no `-fshort-enums`), and the candidate's `i32` aliases and ordered
  constants preserve every value.  The aliases do not create Rust discriminant
  validity assumptions, which is appropriate for values that may originate in
  netlink messages.  The fact that C named-enumerator constants are `int`
  expressions is therefore retained operationally here.
- The UAPI comments correctly retain the payload widths/byte-order descriptors;
  the constants themselves are attribute identifiers, not the payload data.
  No configuration conditional exists in the source header.
- No `unsafe`, FFI layout, panic path, test configuration, placeholder, or
  unauthorized branding was added.  SPDX and immutable provenance match the
  pinned source and queued common-architecture task.

## Verdict

Reject pending resolution of R1.  No other Rust-semantics finding was found.
