# S016150 parity review (slot 1)

## Scope and evidence

Reviewed the complete pinned source `vendor/linux/include/uapi/linux/hsr_netlink.h`
at revision `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/hsr_netlink.rs`.  The frozen aarch64 configuration
selects HSR as a module (`CONFIG_HSR=m`).  Mechanical scope records this header
as `RUST_TRANSLATE` for aarch64, included by three HSR consumers; the relevant
generic-netlink consumer is `vendor/linux/net/hsr/hsr_netlink.c`.

## Result: PASS — no parity findings

The candidate reproduces every operative UAPI constant in the two anonymous C
enumerations, in source order and at the exact C `int` values:

- attributes: `HSR_A_UNSPEC` through `__HSR_A_MAX` are `0` through `11`, and
  `HSR_A_MAX` remains the derived expression `__HSR_A_MAX - 1` (`10`);
- commands: `HSR_C_UNSPEC` through `__HSR_C_MAX` are `0` through `7`, and
  `HSR_C_MAX` remains the derived expression `__HSR_C_MAX - 1` (`6`).

The source declares anonymous enums only, so it exposes integer enumerator
constant expressions and no named enum type, object, layout, linkage, or
calling-convention surface.  The Rust `pub const ...: core::ffi::c_int`
representations preserve that signed C-`int` constant-expression surface on
the selected aarch64 target.  The `*_MAX` expressions are not prematurely
folded or altered.  The reviewed HSR generic-netlink consumer uses these
identifiers as command/attribute integer IDs and its array bounds; the values
and derived maxima agree exactly.

There are no configuration conditionals in the source other than the C
multiple-inclusion guard; it has no independent Rust runtime/UAPI value
surface.  No UAPI name, macro-derived value, ordering, architecture selection,
branding, SPDX identifier, copyright attribution, or required provenance field
is missing or changed.

The two Phase-0 anonymous-enum ABI/lifetime records currently marked
`PENDING_REVIEW` can be closed as: no storage or ownership/lifetime exists;
each is an anonymous enumeration whose enumerators are signed C `int` integer
constant expressions.  No source change is requested.
