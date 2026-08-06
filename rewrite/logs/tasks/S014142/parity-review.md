# Parity review — S014142

## Result: REJECT

### P1 — Critical: the newtype makes upstream unscoped enumerators unusable at every typed token boundary

`include/linux/irqdomain_defs.h:12-29` declares an ordinary C enum.  Its
enumerators are unscoped integer constant expressions: the token definitions
are usable directly wherever an integer expression is required, and C applies
the ordinary assignment/argument conversion when the destination is an
`enum irq_domain_bus_token` object.

The candidate instead makes `irq_domain_bus_token` a distinct transparent Rust
newtype (`src/include/linux/irqdomain_defs.rs:19-22`) while exposing every
`DOMAIN_BUS_*` name as `c_int` (`:25-40`).  Rust performs no corresponding
implicit `c_int -> irq_domain_bus_token` conversion.  Consequently, a direct
translation of the established upstream use sites cannot use these constants:

- `include/linux/irqdomain.h:180` and `:344` store the enum in public
  `irq_domain`/`irq_domain_info` fields;
- `include/linux/irqdomain.h:371`, `:384`, and the `irq_domain_ops` callbacks
  at `:101-103` take the enum type;
- `include/linux/irqdomain.h:406-408`, `kernel/irq/ipi-mux.c:186`, and
  `arch/x86/kernel/hpet.c:565` pass `DOMAIN_BUS_*` directly to those typed
  parameters; and
- `include/linux/irqchip/irq-msi-lib.h:13,18` uses the same identifiers as
  integer shift operands.

The wrapper therefore cannot simultaneously retain the C enumerators' required
integer-expression use and support the header's field/argument uses.  This is
not remedied by the comment asserting that the wrapper retains the C enum tag:
the public Rust API has introduced an incompatible source-level conversion
boundary absent from the pinned implementation.  Resolve with a representation
and exported constants that permit both categories of original use without
per-use semantic adaptation (or record the exact, complete inter-file
translation rule and apply it consistently before accepting this header).

## Checked and no separate finding

- The candidate includes all sixteen enumerators in pinned declaration order,
  with values `0` through `15`.
- A transparent integer carrier does not itself exclude non-enumerator bit
  patterns, unlike a Rust discriminant enum; that aspect is appropriate for
  values arriving from C objects/parameters.
- The added `Copy`, `Clone`, `Eq`, and `PartialEq` impls do not affect the
  carrier's object layout, but do not resolve P1's public API incompatibility.
- The candidate uses the required provenance fields and carries no mutable
  completion claim.  The upstream file has only `SPDX-License-Identifier:
  GPL-2.0`; its immutable rewrite header uses the project-required
  `GPL-2.0-only` form.  No upstream copyright notice is present to retain.
- The source header has no configuration-controlled semantic branch beyond its
  C include guard; both frozen architectures select the same definition.

No compilation, formatting, tests, or diagnostics were run.
