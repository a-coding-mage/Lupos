# Rust review — S000803

Result: **REJECT**

## Finding R1 — UAPI license identifier changed (blocking)

`vendor/linux/arch/x86/include/uapi/asm/unistd.h:1` is licensed as
`GPL-2.0 WITH Linux-syscall-note`; the candidate instead starts with
`GPL-2.0-only`.  This is a UAPI header, and the changed SPDX expression drops
the upstream syscall-note exception.  It violates the immutable provenance
rule requiring retention of the source SPDX identifier.

Required resolution: retain the exact upstream SPDX expression in the
destination provenance header.

## Finding R2 — Public UAPI conditional surface has no mapping (blocking)

The candidate exports only `__X32_SYSCALL_BIT`.  It has no representation of
the non-`__KERNEL__` selection in upstream lines 15–23:

- `__i386__` selects generated `asm/unistd_32.h`;
- `__ILP32__` selects generated `asm/unistd_x32.h`;
- otherwise it selects generated `asm/unistd_64.h`.

These conditionals are explicitly inventoried for S000803 in
`rewrite/SYMBOLS.tsv` (lines 35978–35983).  The generated 32- and 64-bit
headers are Phase-0 build metadata (`S012431` and `S012432` in
`rewrite/SCOPE.tsv`), but the candidate neither establishes the Rust-facing
mapping to that metadata nor provides the corresponding selected syscall-number
public surface.  Treating the current kernel compile command's `__KERNEL__`
definition as a reason to discard a branch of this UAPI header changes its
public behavior and leaves the recorded conditional mappings unresolved.

Required resolution: establish a source-backed Rust representation for all
three UAPI target selections, including an auditable linkage to the generated
syscall-number metadata; if the frozen task mapping cannot represent those
generated public surfaces without adding a new scoped destination, block the
task rather than omit them.

## Checked constant semantics

`0x40000000` has C type `int` in the upstream macro and is exactly represented
by the candidate's `pub const __X32_SYSCALL_BIT: i32`.  For `i32` operands,
Rust `!__X32_SYSCALL_BIT` has the same 32-bit two's-complement mask as C
`~__X32_SYSCALL_BIT`.  C's usual arithmetic conversions still need to be made
explicit at Rust consumers whose operand is wider (for example the upstream
`unsigned long orig_ax` paths); a typed Rust constant cannot itself reproduce
C macro promotion.  This is not a defect in the value or `i32` choice, but the
candidate documentation must not imply automatic cross-width expression
equivalence.

No compiler, formatter, linker, test, runtime command, or diagnostics were
used in this review.
