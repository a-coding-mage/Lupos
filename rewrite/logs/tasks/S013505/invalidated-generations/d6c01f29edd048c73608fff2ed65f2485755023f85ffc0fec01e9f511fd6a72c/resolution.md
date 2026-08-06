# S013505 resolution

Applied against `vendor/linux/include/linux/bcma/bcma_regs.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Parity review finding 1 / Rust review finding 1 — resolved

The Rust declarations now retain the C expression type on both frozen targets:

- The 62 macros whose defining expression has no `U` suffix are `i32`,
  including every register offset, mask, shift, PCI configuration value, the
  four `BCMA_SOC_FLASH*` values, and
  `BCMA_SOC_PCI_MEM_SZ = 64 * 1024 * 1024`.  All literals in those definitions
  are representable as C `int`, and the three operands of the size expression
  are C `int`; the pinned header therefore evaluates each of these macros as
  signed 32-bit `int` on x86_64 and AArch64.
- The 13 macros whose defining literal has the `U` suffix are `u32`:
  `BCMA_SOC_SDRAM_BASE`, `BCMA_SOC_PCI_MEM`, `BCMA_SOC_PCI_CFG`,
  `BCMA_SOC_SDRAM_SWAPPED`, `BCMA_SOC_SDRAM_R2`, `BCMA_SOC_PCI_DMA`,
  `BCMA_SOC_PCI_DMA2`, `BCMA_SOC_PCI_DMA_SZ`,
  `BCMA_SOC_PCIE_DMA_L32`, `BCMA_SOC_PCIE_DMA_H32`,
  `BCMA_SOC_PCI1_MEM`, `BCMA_SOC_PCI1_CFG`, and
  `BCMA_SOC_PCIE1_DMA_H32`.

This preserves the signed/unsigned source categories and values.  Typed
consumer translations must express their source-context conversions explicitly
at the use, including the pinned `resource_size_t` assignments and arithmetic
in `drivers/bcma/driver_pci_host.c:416-474`; this macro-only header does not
silently impose a different common unsigned type.

Read-only inventory comparison confirms 75 public `BCMA_*` constants, with
exactly 62 `i32` and 13 `u32` declarations, matching the 75 pinned object-like
macros and their suffix-derived categories.  All reviewed names and numeric
values are retained.

## Parity review finding 2 / Rust review finding 2 — resolved

The file's SPDX line is restored to the pinned upstream identifier
`GPL-2.0`; no license or branding change remains.

No build, compiler, formatter, test, runtime, or other forbidden Phase 1 tool
was run.
