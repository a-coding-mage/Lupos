# S013505 parity review (slot 1)

Reviewed `vendor/linux/include/linux/bcma/bcma_regs.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/bcma/bcma_regs.rs` for the frozen `common` scope.

## Inventory checked

- All 75 object-like `BCMA_*` macros in source lines 7--102 have one public
  Rust constant with the identical name; there are no extra `BCMA_*` Rust
  constants.
- Each 75 constant evaluates to the source numeric value.  This includes both
  reversed BCM4328 clock-status aliases, all PCI/PCIe offsets and masks, and
  all SiliconBackplane address-map values.
- `BCMA_SOC_PCI_MEM_SZ` remains the arithmetic expression `64 * 1024 * 1024`;
  omitting the C macro's outer parentheses is harmless because Rust const use
  is an atomic path, not textual substitution.
- The source include guard has no runtime or ABI content and is appropriately
  represented by Rust module inclusion.  The source contains no functions,
  layout declarations, storage, configuration conditional, linkage, locking,
  or lifetime operation to port.
- Candidate provenance has the right source path, revision, architecture, and
  task ID.  No branding delta or implementation placeholder was found.

## Findings

1. **Major — the candidate changes the underlying C integer semantics for 62
   unsuffixed macros.**  C lines 7--77 and 99--102 use unsuffixed literals (or
   the all-`int` expression `(64 * 1024 * 1024)`), so on both frozen targets
   their macro expressions have signed 32-bit `int` type.  The candidate
   publishes each as `u32`.  Only the thirteen literals with a `U` suffix on
   source lines 75--76, 78--80, and 83--95 are directly `unsigned int`/`u32`.
   This changes the signedness of public constants and removes the source
   expression's C usual-arithmetic-conversion behavior.  It is material for
   expressions such as signed arithmetic, comparison, shifting, and arguments
   whose C prototypes trigger conversion.  Relevant pinned consumers also use
   several values as `u16` register offsets (`bcma_read*`/`bcma_write*`) and
   use address-map values in `resource_size_t` arithmetic
   (`drivers/bcma/driver_pci_host.c:416--474`).

   The applier must derive and record the intended Rust representation at each
   selected use, retaining signed versus unsigned width and explicit conversion
   behavior rather than treating every macro as a `u32` register value.  The
   task's pending symbol/ABI semantic records must be closed with that source
   evidence before `DONE`.

2. **Minor — SPDX identifier was not retained literally.**  The source starts
   `SPDX-License-Identifier: GPL-2.0`; candidate line 1 changes it to
   `GPL-2.0-only`.  The project rule requires retaining SPDX identifiers.
   Restore the upstream identifier unless a recorded branding/license
   allowlist explicitly authorizes this change (none was found for this task).

## Conclusion

Rejected pending resolution of findings 1 and 2.  Numeric name/value coverage
is otherwise complete.  No build, test, formatter, compiler, or runtime
command was run.
