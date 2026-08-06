# Rust review — S014598

Reviewed independently as slot 2.  I did not inspect the parity-review report.

## Finding R1 — high: every translated macro is forced to `u32`, changing its C integer type and contextual conversion semantics

`include/linux/pci_ids.h` defines all 2,902 translated object-like macros as
unsuffixed integer literals.  The complete-header audit found every value in
the range `0x00000000..0x000d1010` (the maximum is
`PCI_CLASS_WIRELESS_WHCI` at C line 135), so under the pinned targets each
replacement list is an `int`, not an unsigned 32-bit integer.  The candidate
instead declares every name as `pub const ...: u32`, for example
`PCI_CLASS_WIRELESS_WHCI` at `src/include/linux/pci_ids.rs:112` and
`PCI_DEVICE_ID_INTEL_82437VX` at line 2750.

This is not merely storage widening: C macros substitute an `int` expression
and are subsequently converted/promoted in their use context.  A single Rust
`u32` item has fixed unsigned type and cannot stand in for an `int` in signed
arithmetic/comparison or a context that expects a narrower PCI ID without an
explicit, source-derived conversion.  The source demonstrates the latter
contract: `PCI_DEVICE(vend, dev)` in `include/linux/pci.h:1052-1054` inserts
these values into the vendor/device fields, documented immediately below as
16-bit PCI IDs (`include/linux/pci.h:1057-1069`).  The candidate therefore
changes type checking, signedness, promotions, and possible truncation at
every translated macro use.

The applier must preserve the source expression type/conversion behavior at
each Rust use (and record the chosen Rust representations in the appropriate
ABI/type guidance) rather than imposing `u32` uniformly on this header.

## Verified checks without additional findings

- The C header has only its include guard (`include/linux/pci_ids.h:10-11`,
  `:3270`); it has no configuration conditionals.  The candidate introduces
  no `cfg` branch.
- A complete name/value comparison after stripping C comments found 2,902 C
  object macros and 2,902 Rust public constants: no missing or extra names,
  and no numeric-value mismatches.  This includes the seven `PCIE_*` names.
- All C replacements are plain unsuffixed integer literals; none requires a
  suffix, cast, arithmetic expression, alias expansion, pointer provenance,
  or overflow behavior beyond the type issue above.
- The candidate contains no functions, FFI declarations, `unsafe`, layout
  types, or executable code, so no separate ownership/aliasing/drop/ABI
  finding applies here.  `pub const` does not by itself emit a C-linkage
  symbol, matching the macro-only nature of this source.
