# Rust review — S013505

Reviewed candidate: `src/include/linux/bcma/bcma_regs.rs` against pinned
`vendor/linux/include/linux/bcma/bcma_regs.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Findings

1. **High — C constant types are not preserved.**  The candidate declares all
   75 macros as `u32`.  In the C header, only the 13 `BCMA_SOC_*` macros with
   an explicit `U` suffix (lines 75, 76, 78-80, 83-86, and 89, 93-95) have
   type `unsigned int`.  The other 62 object-like macros, including
   `BCMA_SOC_PCI_MEM_SZ` whose source expression is `(64 * 1024 * 1024)`, are
   `int`: every literal involved is representable as an `int` under the frozen
   Linux targets.  Their fixed Rust `u32` types alter integer promotion and
   signedness at each use site, including arithmetic such as the original
   `BCMA_SOC_PCI_MEM_SZ - 1` in
   `drivers/bcma/driver_pci_host.c:449,459,468`.  Preserve the C expression
   types instead of assigning all macros a uniform `u32` type; adapt typed
   consumers explicitly where required.

2. **Medium — SPDX identifier was changed.**  The pinned header begins
   `/* SPDX-License-Identifier: GPL-2.0 */`, while candidate line 1 states
   `// SPDX-License-Identifier: GPL-2.0-only`.  The rewrite rules require
   retaining the upstream SPDX identifier; restore the exact source identifier.

## Checked and found no additional issue

- All 75 `BCMA_` names are present exactly once and their numeric values match
  the pinned header.
- No functions, aggregate layouts, linkage declarations, conditional branches,
  ownership rules, or unsafe code exist in the source header/candidate.
- Provenance source path, revision, architecture (`common`), and task ID match
  the frozen queue and `vendor/linux.SHA`.
- No Rust test configuration, placeholder, or panic was introduced.

No source was edited and no compiler, formatter, build, test, or runtime tool
was run.
