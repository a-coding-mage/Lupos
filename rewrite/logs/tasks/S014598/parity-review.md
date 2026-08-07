# Parity review — S014598 (slot 1)

## Result: FINDING

Reviewed only the pinned `vendor/linux/include/linux/pci_ids.h`, the current
`src/include/linux/pci_ids.rs`, and frozen local task guidance.  No compiler,
formatter, test, linker, rust-analyzer diagnostic, or historical Lupos source
was used.

### 1. Incorrect and duplicate architecture provenance

- **Candidate:** `src/include/linux/pci_ids.rs:4-5` declares both
  `//! architectures: x86_64,aarch64` and `//! architectures: common`.
- **Frozen local evidence:** `rewrite/SCOPE.tsv` row `S014598` assigns
  `include/linux/pci_ids.h -> src/include/linux/pci_ids.rs` the architecture
  membership `common`.  The selected-symbol inventory has the same complete
  macro set for each approved architecture and the pinned source has no
  architecture conditional other than its include guard
  (`vendor/linux/include/linux/pci_ids.h:10-11,3270`).
- **Required resolution:** retain one immutable architecture provenance field
  matching the leased task (`common`); remove the extra x86_64/aarch64 field.

## Exhaustive source comparison evidence

- The pinned header contains 2,902 non-guard macro definitions; the candidate
  contains 2,902 public constants.  Names, source order, and literal values
  match one-for-one, including the `PCIE_*` entries and definitions carrying
  trailing comments.  All candidate constants are explicitly `u32`; the
  source values are non-negative PCI IDs/class codes and no source definition
  has an expression or configuration-dependent replacement.
- The complete sequence of source comments and definitions is retained after
  removal of the C-only include guard.  The guard is the sole selected
  conditional and has no Rust equivalent.
- The candidate preserves the Linux names and values (for example the aliases
  `PCI_VENDOR_ID_NCR`/`PCI_VENDOR_ID_LSI_LOGIC` and duplicate-valued vendor
  IDs); it adds no constants, aliases, functions, layouts, linkage, state,
  allocation, locking, error, or lifetime mechanism.
- `vendor/linux.SHA` and candidate revision provenance both identify
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.  SPDX is present.  The branding
  allowlist has no approved rename, and none is present.  No Rust test,
  conditional test configuration, stub, placeholder, panic, or unauthorized
  branding appears in the candidate.
