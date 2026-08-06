# S014598 application resolution

Pinned source: `vendor/linux/include/linux/pci_ids.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## P1 — accepted and corrected

The source has 2,902 value-bearing object-like definitions.  Every replacement
list is an unsuffixed hexadecimal integer literal, and the largest is
`PCI_CLASS_WIRELESS_WHCI` (`0x0d1010`).  That is representable as a signed C
`int` on both frozen targets; none selects an unsigned type.  The prior uniform
`u32` declarations therefore changed the type of every replacement expression.

The destination now declares every corresponding item as `i32`, retaining the
original spelling, literal token, source order, and comment text.  `i32` is the
Rust representation of the source expression's C `int` type.  Narrowing and
other contextual conversions remain obligations of each translated consumer at
the same source operation where C performs them; this macro-only header does
not introduce a replacement conversion or a blanket unsigned interpretation.

## R1 — accepted and corrected

The independent source audit confirms P1's type evidence.  The correction
removes all `u32` declarations from this destination and introduces no casts,
wrappers, aliases, arithmetic, or changed literal value.  A post-application
name/value/order comparison of the source definitions and Rust declarations
matched all 2,902 pairs exactly.

## Record closure

For this task's operative macro records, the final semantic mapping is:
unsuffixed C hexadecimal literal in the signed-`int` range -> `pub const` with
the same literal and Rust `i32` type.  There are no aliases, conditional
branches, function-like macros, ABI/linkage items, ownership/lifetime rules,
locking, or unsafe operations in this header.  The conventional include guard
is represented by the Rust module boundary.  The required source provenance is
present and the source comments have been retained; no branding change was
made.

No compiler, formatter, linker, test, emulator, debugger, benchmark, or
runtime command was run.
