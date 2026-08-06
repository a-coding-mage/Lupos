# Parity review — S000426

Reviewed independently against pinned `vendor/linux/arch/x86/events/amd/iommu.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 configuration,
the task inventory, and the sole direct consumer
`vendor/linux/arch/x86/events/amd/iommu.c`.  This review used source inspection
only; no compiler, formatter, analyzer, test, or runtime command was run.

## Findings

1. **Required upstream copyright and author attribution omitted (major).**
   `vendor/linux/arch/x86/events/amd/iommu.h:2-7` carries:

   ```c
   /*
    * Copyright (C) 2013 Advanced Micro Devices, Inc.
    *
    * Author: Steven Kinney <Steven.Kinney@amd.com>
    * Author: Suravee Suthikulpanit <Suraveee.Suthikulpanit@amd.com>
    */
   ```

   The candidate retains only the SPDX identifier and adds provenance, omitting
   the complete copyright/attribution notice.  This violates the required
   retention of relevant upstream copyright notices and fails the requested
   exact attribution preservation.  Restore this notice verbatim (in Rust
   comment form) ahead of the immutable provenance block.

2. **Selected operative header-guard macro is absent (major).**
   The oracle has the `_PERF_EVENT_AMD_IOMMU_H_` guard at
   `iommu.h:9-10,24`.  The frozen inventory explicitly selects `ifndef@9`,
   `endif@24`, and operative macro `_PERF_EVENT_AMD_IOMMU_H_` in
   `rewrite/SYMBOLS.tsv:16537-16539`.  The candidate supplies neither a mapped
   guard nor an explanation of the Rust module mechanism that enforces the
   same single-definition/include behavior.  No module index presently exists
   (per phase policy), so the candidate alone does not establish the required
   guard semantics.  The applier must provide an exact Rust-side mapping whose
   importing behavior prevents the repeated-definition effect guarded against
   by the upstream header, and close the three pending symbol records with
   source evidence.

3. **Typed `i32` constants do not preserve the C macro replacement semantics
   at the direct consumer boundary (major).**
   Every upstream value is an unsuffixed integer-literal macro, e.g.
   `IOMMU_PC_COUNTER_SRC_REG` is `0x08` at `iommu.h:14`; it has ordinary C
   integer-literal expression semantics rather than a fixed declared object
   type.  The sole direct consumer passes these macros directly as the fourth
   argument to `amd_iommu_pc_set_reg`/`amd_iommu_pc_get_reg`; that parameter is
   `u8` by `vendor/linux/include/linux/amd-iommu.h:65-68`, and the direct calls
   are at `vendor/linux/arch/x86/events/amd/iommu.c:248,254,260,266,276,305,318`.
   C therefore performs the normal, lossless conversion of each replacement
   literal at each call site.  The candidate declares all eight names as
   `pub const ...: i32`, which makes the names fixed `i32` Rust values and
   requires downstream explicit casts before they can occupy the translated
   `u8` ABI position.  Its added doc comment acknowledges this changed calling
   contract rather than preserving the macro behavior.  The candidate also
   imposes `i32` on every other expression context, whereas the macros are
   replacement tokens subject to C's contextual conversions/promotions.
   The applier must choose and document a faithful mapping based on the
   complete selected consumer interface; it must not leave a hidden narrowing
   obligation to a later consumer translation.  The values themselves are
   correct: `0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 64, 16`.

## Checked without defect

- The SPDX identifier is exactly `GPL-2.0-only`.
- Immutable provenance identifies the correct Linux path, revision, x86_64
  architecture, and task ID.
- All eight selected value macro names are present, public, and carry the
  exact numeric values from `iommu.h:13-18,21-22`; no branding substitution,
  extra operative behavior, functions, types, statics, or configuration
  branches were introduced.
- `rewrite/SCOPE.tsv:427` classifies this as `RUST_TRANSLATE`, x86_64-only,
  with one header consumer.  `rewrite/metadata/header_closure.tsv:7846` and
  `rewrite/metadata/task_dependencies.tsv:12311` identify that consumer as
  `arch/x86/events/amd/iommu.c` / task S000425.  The frozen configuration has
  `CONFIG_CPU_SUP_AMD=y`, `CONFIG_AMD_IOMMU=y`, and `CONFIG_PERF_EVENTS=y`.
- No task-specific ABI or lifetime record was present in `rewrite/ABI.tsv` or
  `rewrite/LIFETIMES.tsv`; this does not discharge the pending guard/macro
  semantic records in `rewrite/SYMBOLS.tsv`.

Result: **reject pending applier resolution of findings 1–3.**
