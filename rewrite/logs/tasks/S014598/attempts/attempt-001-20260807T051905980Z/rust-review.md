# Rust source review — S014598

Result: **FINDINGS — reject current candidate.**

Reviewed task `S014598`, attempt `1`, pipeline `P01`, while its queue row was
`REVIEWING`.  The pinned source is
`vendor/linux/include/linux/pci_ids.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`; the frozen queue fingerprint is
`943e5f2626a4c95a4f0d2e83171907bf6a5b5b86611106cd497ee846f13da0c0` and the
Phase 0 identity binding is
`2e2117b1c1c14e3dbdb6b4ebde7459fd44c41dbcef33a7aeea1a1c730a28d303`.

## Findings

### RUST-001 — all PCI integer macros have been given the wrong Rust integer type

`pci_ids.h` defines the PCI identifiers as unsuffixed hexadecimal integer
literals: for example `PCI_CLASS_NOT_DEFINED` is `0x0000` at line 15 and
`PCI_CLASS_STORAGE_SATA_AHCI` is `0x010601` at line 25.  Every one of the
2,895 `PCI_*` defines is such a literal; the largest has six hex digits and
therefore fits the signed `int` used by both frozen Linux targets.  C preserves
that `int` type at every macro expansion and applies the C conversion and
usual-arithmetic-conversion rules at each use.

The candidate instead declares every corresponding item as a `u32`, including
`PCI_CLASS_NOT_DEFINED` at `src/include/linux/pci_ids.rs:18` and
`PCI_CLASS_STORAGE_SATA_AHCI` at line 27.  This forces unsigned 32-bit typing
before each consumer expression.  It changes mixed signed/unsigned comparisons,
arithmetic, promotions, and the casts required when a selected Linux consumer
stores an ID in a signed or narrower object.  A module `const` also cannot
preserve the source macro's context-dependent C integer typing.  The exact
Linux mechanism therefore is not preserved; the candidate must use a
source-evidenced representation that retains the required per-use C conversion
semantics rather than imposing `u32` globally.

Evidence: `vendor/linux/include/linux/pci_ids.h:15`,
`vendor/linux/include/linux/pci_ids.h:25`, and the full set of 2,895 `PCI_*`
defines through line 3268; corresponding candidate declarations begin at
`src/include/linux/pci_ids.rs:18`.

### RUST-002 — the sealed semantic proposal does not bind the current candidate

The sealed task proposal records candidate SHA-256
`8b006aec7b522fe2d5d656a35e421034ae1611acfe97a4b74a4ca3b3f819ed62`
(proposal record and sealed proposal
`rewrite/logs/tasks/S014598/semantic-closure-proposal.sha256`, proposal digest
`32e3da5e0b54396e3bfe0519a2eb7a8feb4f8b545176743f267584bb09c6307f`).  The
current `src/include/linux/pci_ids.rs` hashes to
`d20f1f0ca8694670cd0869c75c9fdc44eb438d225ddca0abbb2d42aab9cc3ca3`.

Consequently the exact current candidate has no sealed proposal/citation binding
for this attempt.  Slot-2 semantic attestation would be false and must not be
submitted; the task needs a newly sealed proposal for the unchanged candidate
before independent review can be attested.

## Additional manual checks

The 2,895 Linux `PCI_*` macro names and numeric values have a one-to-one match
in the 2,895 Rust declarations.  The candidate contains no `unsafe`, FFI,
layout declaration, callback, allocation, `Drop`, pointer, test, panic, or
conditional-compilation construct.  Those categories therefore introduce no
separate finding in this header; they do not cure RUST-001 or RUST-002.
