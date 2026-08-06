# S000426 applier resolution

Applied by source inspection only against pinned
`vendor/linux/arch/x86/events/amd/iommu.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its sole selected consumer
`vendor/linux/arch/x86/events/amd/iommu.c`, and the frozen x86_64 Phase 0
metadata.  No compiler, formatter, analyzer, test, runtime command, or
historical Lupos Rust source was used.

## Review finding dispositions

1. **Upstream copyright and authors — fixed.**  The candidate now retains the
   complete upstream notice and both author lines verbatim in Rust line-comment
   form.  The SPDX identifier and immutable provenance remain unchanged.

2. **`_PERF_EVENT_AMD_IOMMU_H_` include guard — resolved.**  The selected
   `#ifndef`, `#define`, and `#endif` at oracle lines 9, 10, and 24 control only
   preprocessor textual inclusion.  `rewrite/FILE_MAP.tsv` gives this header
   exactly one destination, `src/arch/x86/events/amd/iommu_h.rs`; Rust imports
   that canonical module rather than textually including it, so a module has a
   single definition in its namespace.  The candidate documents that direct
   mapping.  The C guard introduces no runtime state, layout, ABI, linkage, or
   configuration behavior and therefore has no Rust value/item counterpart.

3. **Contextual macro conversion — fixed.**  The former `i32` constants were
   replaced with eight expression macros, retaining each original macro name
   as a crate-visible re-export.  This preserves literal expansion at each
   Rust use rather than introducing a fixed Rust object type.  The complete
   pinned-tree search finds these uses only in `arch/x86/events/amd/iommu.c`:

   - `IOMMU_PC_COUNTER_REG`: lines 305 and 318;
   - `IOMMU_PC_COUNTER_SRC_REG`: lines 248 and 276;
   - `IOMMU_PC_DEVID_MATCH_REG`: line 254;
   - `IOMMU_PC_PASID_MATCH_REG`: line 260;
   - `IOMMU_PC_DOMID_MATCH_REG`: line 266.

   Each is passed as `fxn` to `amd_iommu_pc_set_reg` or
   `amd_iommu_pc_get_reg`, whose pinned declaration takes `u8`
   (`include/linux/amd-iommu.h:65-68`).  The respective values `0x00`, `0x08`,
   `0x10`, `0x18`, and `0x20` are all in range, so Rust contextual inference at
   each future macro call produces the same `u8` value as C's `int` literal
   followed by parameter conversion.  `IOMMU_PC_COUNTER_REPORT_REG`,
   `PC_MAX_SPEC_BNKS`, and `PC_MAX_SPEC_CNTRS` have no selected pinned-tree
   consumer; their macros retain the exact literal tokens for any later
   selected expression context.  The future S000425 translation must invoke
   these expression macros at its direct equivalents of the listed C calls;
   this header candidate neither omits nor replaces that consumer logic.

## Pending-record closure

The eleven S000426 rows in `rewrite/SYMBOLS.tsv:16537-16547` are closed by the
source facts above: the three guard rows are compile-time-only module mapping;
the eight operative macro rows map one-for-one to expression macros with exact
literal values.  `rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` have no S000426
rows because this header declares no ABI object, ownership, lifetime,
synchronization, RCU, refcount, allocation, or cleanup contract.  The frozen
selection is x86_64-only and unconditional for the selected header consumer
(`CONFIG_CPU_SUP_AMD=y`, `CONFIG_AMD_IOMMU=y`, `CONFIG_PERF_EVENTS=y`).

No unresolved source, ABI, lifetime, or selected-branch question remains for
S000426.
