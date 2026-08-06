# S014598 parity review (slot 1)

Reviewed candidate: `src/include/linux/pci_ids.rs` against pinned
`vendor/linux/include/linux/pci_ids.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding P1 — all object-like macros were given an incompatible fixed `u32` type

- **Candidate evidence:** every translated item is declared `pub const ...: u32`,
  including `PCI_CLASS_COMMUNICATION_MODEM` at
  `src/include/linux/pci_ids.rs:60` and `PCI_CLASS_OTHERS` at
  `src/include/linux/pci_ids.rs:129`.
- **Upstream evidence:** the corresponding source definitions are unsuffixed C
  integer literals at `vendor/linux/include/linux/pci_ids.h:75` and `:158`; this
  is true of all 2,902 value-bearing object-like definitions.  These macros are
  contextually converted by C, rather than having a universal `u32` type.  For
  example, `vendor/linux/drivers/pci/endpoint/functions/pci-epf-mhi.c:92-98`
  initializes `struct pci_epf_header` fields with `PCI_BASE_CLASS_COMMUNICATION`
  and `PCI_CLASS_COMMUNICATION_MODEM & 0xff`, and the receiving
  `baseclass_code` and `subclass_code` fields are `u8` in
  `vendor/linux/include/linux/pci-epf.h:45-53`.
- **Impact:** Rust `u32` constants make the equivalent expressions `u32`; they
  cannot be used as `u8`/`u16` initializers or arguments without an explicit
  cast.  That changes the header's context-polymorphic integer-literal contract
  and omits the required conversions at every such use site.  Preserve the
  C-literal typing/consumer conversion semantics instead of imposing `u32` on
  every identifier.

## Exhaustive checks with no additional finding

- The 3,270-line source has only its conventional include guard; it has no
  configuration or architecture conditional branch and no function-like macro.
- After stripping comments, the 2,902 source object-like macro name/value pairs
  exactly match the 2,902 candidate constants, in source order.  No numeric
  value, spelling, alias, duplicate, omission, or reordering difference was
  found.
- The candidate provenance identifies the correct source path, pinned revision,
  `common` architecture class, and task ID.  The Rust module boundary is an
  appropriate replacement for the C include guard.
- No branding delta, ABI/linkage item, stateful behavior, allocation, locking,
  cleanup path, or architecture-specific behavior exists in this header.

No source, queue, manifest, build, formatter, test, or runtime action was
performed during this review.
