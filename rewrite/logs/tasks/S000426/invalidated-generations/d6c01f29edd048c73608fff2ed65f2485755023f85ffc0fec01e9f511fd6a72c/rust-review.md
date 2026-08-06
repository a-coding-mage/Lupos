# Rust semantic review — S000426

Reviewed independently against pinned `vendor/linux/arch/x86/events/amd/iommu.h`, the frozen x86_64 configuration, `SCOPE.tsv`, `SYMBOLS.tsv`, the ABI/lifetime records, and its selected direct consumer `arch/x86/events/amd/iommu.c`.  No compiler, formatter, test, or compiler-backed diagnostic was used.

## Finding R1 — upstream copyright and attribution were dropped (must fix)

`src/arch/x86/events/amd/iommu_h.rs:1` retains the SPDX identifier but omits the relevant notices at upstream lines 3–6: `Copyright (C) 2013 Advanced Micro Devices, Inc.` and the two named authors.  The fresh-source rule requires retaining relevant upstream copyright notices.  Restore those notices as Rust comments without changing the immutable provenance block.

## Semantic audit

- The eight selected macro replacements at upstream lines 13–22 are all unsuffixed C integer constants.  On the frozen x86_64 target their type is C `int`; candidate lines 15–23 use `i32` and retain every value exactly.  No overflow, sign, shift, cast, unsafe, panic, allocation, layout, or FFI operation is introduced by these constants.
- The only selected direct consumer is `arch/x86/events/amd/iommu.c`.  Its used register selectors are passed to `amd_iommu_pc_set_reg`/`amd_iommu_pc_get_reg`, whose pinned declaration (`include/linux/amd-iommu.h:65–68`) takes `u8 fxn`.  C performs the value-preserving conversion for the listed values (0 through 0x28).  The future Rust translation of that consumer must make the corresponding explicit, value-preserving `i32`-to-`u8` narrowing at those call boundaries; the direct consumer task S000425 remains `TODO`, so this candidate cannot itself provide that use-site conversion.  `PC_MAX_SPEC_BNKS` and `PC_MAX_SPEC_CNTRS` have no other pinned-tree consumer.
- The C include guard has no runtime, layout, linkage, or FFI effect.  A single Rust module supplies the analogous one-definition behavior; no Rust guard constant is required.
- The header is unconditionally selected by the frozen x86_64 built-in `arch/x86/events/amd/iommu.o`; it has no feature conditional around the eight replacements.  Candidate architecture provenance is `x86_64`, matching the task.
- `rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` contain no S000426 row: this macro-only header declares no ABI object or ownership/lifetime contract.  The task scope record and all eleven `SYMBOLS.tsv` rows remain `PENDING_REVIEW`; the applier must explicitly close those records in its resolution, including that the guard is compile-time-only and each value is an `int` replacement used under the `u8` call contract.

Apart from R1, no Rust-semantic finding in this header candidate.
