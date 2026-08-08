# Parity review — S016464 (slot 1)

Verdict: **APPROVE**

Reviewed only the pinned Linux header, frozen task records, candidate snapshot, current destination file, and direct local consumers. No compiler, formatter, test, runtime, or historical-source material was used.

## Evidence and comparison

- `rewrite/SCOPE.tsv` row `S016464` maps `include/uapi/linux/virtio_ids.h` one-to-one to `src/include/uapi/linux/virtio_ids.rs` for `common`; its recorded frozen consumers cover both x86_64 and AArch64.
- Linux `include/uapi/linux/virtio_ids.h:32-71` defines 40 ordinary Virtio ID integer macros. The Rust candidate defines the identical 40 public names with identical decimal values: `VIRTIO_ID_NET` through `VIRTIO_ID_GPIO` (1–41 with the upstream gaps) and `VIRTIO_ID_SPI = 45`.
- Linux `include/uapi/linux/virtio_ids.h:77-83` defines seven transitional macros. The candidate preserves all seven names and hexadecimal values: `VIRTIO_TRANS_ID_NET` through `VIRTIO_TRANS_ID_RNG` (`0x1000`–`0x1005`) and `VIRTIO_TRANS_ID_9P = 0x1009`.
- All 47 numeric macros are integer constants in the pinned header and use values within C `int`; each Rust counterpart is an `i32`, preserving the selected values and their direct integer-use context. Direct pinned consumers use the IDs as fixed device IDs, matching the candidate’s constants.
- Linux symbols `ifndef@1`, `_LINUX_VIRTIO_IDS_H` at line 2, and `endif@85` are the C preprocessor’s repeat-inclusion guard only. The mapped Rust file is a single dedicated module source, so no runtime value, ABI item, branch, linkage item, allocation, ordering, locking, or error path is omitted by expressing the header through the Rust module boundary rather than a C include guard macro.
- The candidate has no operative conditionals beyond that guard, no types, functions, statics, layouts, linkage, allocation, synchronization, cleanup, or error behavior to translate. The frozen ABI and lifetime records add no applicable item for this macro-only header.
- Candidate provenance names the exact Linux source, frozen revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, `common` architecture scope, and task `S016464`. Its `BSD-3-Clause` SPDX identifier matches the pinned header’s stated BSD three-clause licensing terms. No branding delta is present or allowlist entry required.

No parity findings. The candidate snapshot and current Rust destination agree on this complete macro-only translation.
