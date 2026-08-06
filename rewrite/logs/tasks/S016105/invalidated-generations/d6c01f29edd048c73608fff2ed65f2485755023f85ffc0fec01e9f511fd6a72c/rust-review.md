# Rust review — S016105

Scope reviewed: `include/uapi/linux/dpll.h` against
`src/include/uapi/linux/dpll.rs` for the frozen revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` on the `common` architecture
scope.  This was a source-only review; no compiler, formatter, test, or
runtime command was run.

## Findings

### R1 — High: enumerators were changed from C `int` constant expressions to distinct Rust wrapper values

Evidence: every enumerator in the C declarations at
`vendor/linux/include/uapi/linux/dpll.h:20-306` is an untyped C enumeration
constant (and therefore has type `int` in C), whereas the candidate declares
fourteen `#[repr(transparent)] pub struct dpll_* (pub c_int)` types at
`src/include/uapi/linux/dpll.rs:12-34` and gives each corresponding constant
that wrapper type (for example `DPLL_A_ID: dpll_a` at line 119).

This does not preserve the source interface or C integer-promotion behavior:
C consumers may use an enumerator as an `int` constant expression and rely on
the normal integral conversions in assignments, masks, arithmetic, and
netlink-attribute APIs.  The Rust candidate instead requires explicit field
extraction/conversion and supplies no arithmetic, bitwise, or conversion
operations.  In addition, `repr(transparent)` over `c_int` asserts a signed
underlying representation for the separately named enum object types without
closing the corresponding ABI manifest records, which remain `PENDING_REVIEW`
for both targets.

The applier must derive and document the required representation of the enum
tags separately from the enumerator constant-expression interface, then expose
the latter with the same integer semantics.  Do not treat the wrapper value
type as a drop-in replacement merely because its current numerical values fit
in 32 bits.

### R2 — High: C string-literal macros were changed into Rust references rather than preserving C array/decay semantics

Evidence: `DPLL_FAMILY_NAME` and `DPLL_MCGRP_MONITOR` are C string-literal
macros at `vendor/linux/include/uapi/linux/dpll.h:10,308`; each expands to a
`char[N]` string literal (including the NUL), supports array contexts such as
`sizeof`, and decays to a C character pointer in ordinary expressions.  The
candidate exports `&[u8; 5]` and `&[u8; 8]` at
`src/include/uapi/linux/dpll.rs:37,187`.

A Rust shared reference is neither a C `char[N]` expression nor a `char *`
value suitable for C-compatible FFI use, and `[u8; N]` does not establish the
required C-character signedness or pointer conversion.  The NUL bytes and
lengths are correct, but the macro expression/ABI behavior is not.  The
applier must provide a representation and use-site contract that preserves
both fixed-array contents and C-pointer decay behavior where the translated
callers require it.

## Checked without finding

- All 126 public `DPLL_*`/`__DPLL_*` identifiers from the header are present
  in the candidate; a sorted source-to-candidate set comparison was empty.
- Explicit values, implicit increments, private `__*_MAX` sentinels, and
  public `*_MAX = __*_MAX - 1` aliases match the pinned source numerically.
- The header has no configuration-dependent branches beyond its include guard;
  the candidate introduces no cfg split.  No branding delta is allowlisted or
  present.
- SPDX expression and immutable provenance source, revision, architecture,
  and task ID match the required source/task identity.

Disposition: reject pending applier resolution of R1 and R2.
