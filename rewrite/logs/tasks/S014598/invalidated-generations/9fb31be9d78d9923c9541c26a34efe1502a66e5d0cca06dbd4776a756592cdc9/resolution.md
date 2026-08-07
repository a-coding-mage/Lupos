# Application resolution — S014598

Task: `include/linux/pci_ids.h` → `src/include/linux/pci_ids.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`  
Disposition: all source-review findings resolved by source inspection; no compiler, formatter, linker, test, runtime tool, or historical Rust source was used.

## 1. Duplicate/mismatched architecture provenance — RESOLVED

Accepted.  The single architecture provenance line is now
`//! architectures: common`.  `rewrite/SCOPE.tsv` and the immutable queue row
for `S014598` both assign `common`; it is the task's protocol membership, even
though the header is selected by both approved architectures.  This is also the
exact string required by `tools/rewrite_queue.py` when it validates immutable
provenance (`required_headers` includes `//! architectures: {row['architectures']}`).
The duplicate `x86_64,aarch64` line was removed.  The other four required
provenance lines match the task, source path, and `vendor/linux.SHA`.

## 2. Blanket `u32` constants — RESOLVED

Accepted and corrected.  I reopened the complete pinned 3,270-line header and
audited every non-guard object-like macro.  It contains exactly 2,902
`#define NAME 0x...` replacement lists: every literal is unsuffixed hexadecimal,
has no expression or configuration branch, and lies in the inclusive range
`0x00000000..0x000d1010`, so each has the active signed 32-bit C `int` type on
the frozen x86_64 and AArch64 targets identified by the fresh Rust review.
`u32` changed that source integer type.  Every translated item is now an
explicit `pub const NAME: i32 = 0x...;`; consumers must perform a conversion
only where their corresponding C context performs one.

The final candidate is byte-for-byte equal to the deterministic projection of
the pinned header after only these mechanical transformations: remove the
C include guard, add the five immutable Rust provenance lines, and replace
each macro definition with its same-name `i32` constant.  Projection/candidate
SHA-256: `89db61b4b7e4030b9ec9c0ee3b10a42fc26520900612a23041c17a916c0fd653`.

## 3. Application audit: macros with trailing comments omitted from the prior candidate — RESOLVED

During application, the fresh reports' stated 2,902-candidate count was
disproved: the prior file had 2,845 constants and omitted 57 definitions whose
source lines carry trailing comments.  This was material missing behavior, so
all 57 were restored in source order with identifier, literal, and comment
unchanged.  Examples include `PCI_VENDOR_ID_COMPEX2` (pinned line 529),
`PCI_DEVICE_ID_NEC_CBUS_1` (702), `PCI_VENDOR_ID_CREATIVE` (1413),
`PCI_DEVICE_ID_INTEL_LIGHT_RIDGE` (2746), and
`PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_RAS` (3031).  The final candidate therefore
has exactly 2,902 public `i32` constants, no extras, no omissions, and no
name/value mismatch against the pinned source.

## Final task semantic evidence

`pci_ids.h` is a common, configuration-independent catalogue of integer
macros: it has no functions, types, storage, ABI layout, ownership, lifetime,
locking/RCU, refcount, allocation, callback, or error-path semantics.  The
only C conditional is its include guard, represented by the Rust module
boundary.  `SYMBOLS.tsv` contains 5,810 completed per-architecture records
for this task and there are no matching ABI, lifetime, driver-ABI, or blocker
records.  The frozen scope row's task-level `PENDING_REVIEW` field cannot be
hand-edited in Phase 1; this resolution supplies its required source-level
closure without changing the frozen manifest.

Manual final checks established the exact required provenance, the complete
2,902-to-2,902 mapping, absence of `todo!`, `unimplemented!`, and Rust test
configuration, and clean focused patch whitespace.  This is a source-review
completion only; it makes no compile, link, boot, runtime, or test claim.
