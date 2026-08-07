# Applier resolution — S014598, attempt 3

Pinned source reopened: `vendor/linux/include/linux/pci_ids.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`. Queue task `S014598` remains `APPLYING` in `P01`; the frozen Phase-0 identity and queue fingerprint match the sealed proposal and both reviewer attestations.

## Findings

### P001 — DISPROVED

`semantic_closure.py` defines proposal `candidate_sha256` as `sha256_file(paths["candidate"])`, and `paths["candidate"]` is `rewrite/logs/tasks/S014598/candidate.diff` (tool lines 63–70, 487–496, 513–522, 717–743). The sealed digest `35cdc7e8196d9ac2bd382ca3a91a0b2a8e9266caea8f158ba48f8902616e66ce` equals that current evidence artifact. The proposal format does not define this field as a destination-source hash, so the differing `src/include/linux/pci_ids.rs` digest is not a stale-proposal defect.

### P002 — RESOLVED_CHANGED

Upstream's only conditional is the include guard: `#ifndef _LINUX_PCI_IDS_H` at line 10, an empty `#define _LINUX_PCI_IDS_H` at line 11, and the closing `#endif` at line 3270. I removed the value-bearing `pub const _LINUX_PCI_IDS_H: core::ffi::c_int = 1`. Rust's path-module/import-once mechanism represents this non-value preprocessing guard; no exported typed surrogate remains. The closure's recorded upstream facts are unchanged, so its final semantic values require no alteration.

### F1 — DISPROVED

Same disposition as P001: the sealed artifact binds `candidate.diff`, not the destination source file. The closure tool's validation accepts the current `candidate.diff` digest and defines no source-hash requirement for `candidate_sha256`.

### F2 — RESOLVED_CHANGED

Same source correction as P002. The final Rust file exports exactly the 2,902 value-bearing upstream PCI macro mappings and no `_LINUX_PCI_IDS_H` integer item. A source-only name/value comparison of all upstream `#define` entries excluding the guard against Rust constants returned zero deltas.

## Final source evidence

- Upstream: 2,902 value-bearing `#define` entries; candidate: 2,902 `pub const` entries; normalized name/value delta: zero.
- The header's only preprocessor conditional is its two-line opening/closing include guard.
- No compiler, formatter, linker, test, runtime, benchmark, or rust-analyzer diagnostic was invoked.
