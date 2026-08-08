# Parity review — S000758 (slot 1)

Reviewed `vendor/linux/arch/x86/include/asm/vmxfeatures.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/arch/x86/include/asm/vmxfeatures.rs` and the task-local candidate diff.

## Scope and local evidence

- `rewrite/SCOPE.tsv` classifies this x86_64 header as `RUST_TRANSLATE`; its
  frozen symbol inventory contains all 65 operative object-like definitions:
  `NVMXINTS` and the 64 `VMX_FEATURE_*` bit-index definitions.
- The pinned header has no selected Kconfig-controlled definition branch.
  Its include guard only prevents repeated textual inclusion.
- Direct consumers confirm the required constant-expression roles:
  `arch/x86/include/asm/processor.h` uses `NVMXINTS` as the
  `vmx_capability` array extent; `arch/x86/kernel/cpu/feat_ctl.c` uses it in
  `BUILD_BUG_ON` and obtains control-word bit positions via
  `VMX_FEATURE_* & 0x1f`; `arch/x86/include/asm/vmx.h` does the same through
  `VMCS_CONTROL_BIT` and `VMFUNC_CONTROL_BIT`.

## Result

APPROVE — no parity findings.

All 65 selected names are present exactly once in the candidate.  `NVMXINTS`
remains `5`, and every `VMX_FEATURE_*` replacement expression preserves its
pinned `(word * 32 + bit)` value and therefore its bit index, including the
scattered word-0 EPT/VPID/APIC/VM-function entries and tertiary
`VMX_FEATURE_IPI_VIRT`.  The explicit `i32` constants preserve the signed
32-bit C `int` type of the unsuffixed literal expressions; all defined values
(0 through 100) are representable without overflow.  Rust module inclusion
supplies the one-definition behavior corresponding to the C include guard.

No symbols, branches, linkage/layout obligations, allocation or lifetime
mechanisms, error paths, ordering/locking behavior, or non-allowlisted
branding changes exist in this definition-only header.  The candidate retains
the required provenance and an SPDX identifier; no project-authored Rust test,
stub, or placeholder is present.
