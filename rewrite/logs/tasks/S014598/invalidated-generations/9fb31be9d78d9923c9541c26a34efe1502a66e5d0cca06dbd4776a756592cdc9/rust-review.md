# Rust semantics review — S014598

Result: REJECT

1. **All PCI ID constants have the wrong Rust integer type.**  The pinned
   header defines 2,902 object-like macros as unsuffixed hexadecimal integer
   literal replacement lists (for example `PCI_CLASS_NOT_DEFINED` at
   `vendor/linux/include/linux/pci_ids.h:13` and
   `PCI_DEVICE_ID_BERKOM_A1T` at line 179).  Every value is representable by
   the 32-bit signed `int` used by both frozen Linux targets, so those C
   expansions participate in the usual signed integer promotions and
   conversions as `int`.  The candidate instead fixes every name to `u32`
   (beginning at `src/include/linux/pci_ids.rs:17`).  That changes expression
   typing at every consumer: comparisons and arithmetic with promoted `u16`
   fields are signed in C, while the Rust constants require unsigned operands
   or explicit casts; it also changes the type selected for any context that
   consumes the macro directly.  Preserve the source integer semantics (or a
   context-preserving macro mechanism) and make consumer conversions explicit
   only where the corresponding C conversion occurs; do not make the entire
   header `u32` merely because many PCI ID storage fields are unsigned.

2. **Provenance has two incompatible architecture declarations.**  The
   candidate declares both `//! architectures: x86_64,aarch64` and
   `//! architectures: common` at lines 4-5.  The leased queue/scope row is
   `common`; provenance must contain one exact, unambiguous architecture field
   for the task rather than duplicate keys with different values.

Source-inspection coverage: the 2,902 numeric macro name/value pairs match the
candidate exactly.  The header has no functions, layout-bearing types, FFI,
pointer operations, allocation, callbacks, or unsafe code; no forbidden Rust
tests, placeholders, panics, `unwrap`, or `expect` were present.  No compiler,
formatter, test, or diagnostic tool was invoked.
