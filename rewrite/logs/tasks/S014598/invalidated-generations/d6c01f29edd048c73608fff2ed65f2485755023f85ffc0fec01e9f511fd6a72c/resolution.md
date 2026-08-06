# S014598 application resolution — attempt 2

Applier source review was performed on the frozen
`feat/bun-like-rewrite-test` branch, without compiler, formatter,
rust-analyzer, build, test, debugger, or historical-source input.

- Pinned source: `vendor/linux/include/linux/pci_ids.h`
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/linux/pci_ids.rs`
- Queue fingerprint: `d6c01f29edd048c73608fff2ed65f2485755023f85ffc0fec01e9f511fd6a72c`
- Task scope: common x86_64/aarch64 header

## Review dispositions

| Review item | Disposition | Upstream evidence |
| --- | --- | --- |
| Parity P1: public PCI-ID macro-surface completeness | Resolved. The destination contains all 2,902 `PCI_*` and `PCIE_*` object-like definitions from the pinned header, once each, with identical names and hexadecimal literal spellings. This includes every trailing-comment definition identified during review. | `vendor/linux/include/linux/pci_ids.h:15-3268`; `src/include/linux/pci_ids.rs:17-3273` |
| Rust review: literal type, visibility, layout, and safety | Accepted. Each source expression is an unsuffixed hexadecimal integer literal in the C `int` domain for both frozen targets; the corresponding public Rust item is `i32`. The header has no functions, storage, FFI layouts, derived expressions, or unsafe boundary. | `vendor/linux/include/linux/pci_ids.h:15-3268`; `src/include/linux/pci_ids.rs:17-3273` |

## Independent source checks

The complete pinned header was compared against the destination after excluding
only the C include guard. The comparison covers 2,902 object-like `PCI_*` and
`PCIE_*` definitions and found no missing names, extra names, duplicate names,
or literal mismatches. Every selected definition has a bare unsuffixed
hexadecimal literal; no cast, operator, function-like macro, conditional
branch, or suffix requires another Rust expression form.

The immutable Rust provenance names the exact source path, revision, common
architecture scope, and task ID. The retained upstream SPDX notice is
`GPL-2.0`, while the immutable Rust translation SPDX identifier is
`GPL-2.0-only`, as required by the fresh-source-tree policy.

## Semantic-record closure

The 5,806 operative-macro inventory records comprise the 2,902 numeric
`PCI_*`/`PCIE_*` identifiers plus the `_LINUX_PCI_IDS_H` include-guard macro,
for each frozen architecture. The numeric records are closed by the complete
source comparison above and have no ownership, lifetime, locking, RCU,
refcount, allocation, cleanup, ABI-layout, linkage, or calling-convention
contract. The guard macro and the four inventory conditional records are the
C include-guard open and close directives; they have no Rust value or
configuration behavior and are correctly represented by the Rust module
boundary. This task has no S014598 ABI, lifetime, or driver-ABI rows and no
S014598 blocker.

No project-authored Rust tests, placeholder constructs, unsafe code, or
replacement table were introduced. The final result is source-review complete
only; it makes no compile, link, runtime, or test claim.
