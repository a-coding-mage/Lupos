# Implementation S014598 (attempt 3)

- Branch: `feat/bun-like-rewrite-test`
- Pipeline/lease: `P01` / `codex-root-resume-20260807-p01`
- Linux source: `vendor/linux/include/linux/pci_ids.h`
- Destination: `src/include/linux/pci_ids.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`
- Translation: all 2902 numeric `#define` macros are represented as public Rust constants with `core::ffi::c_int` (C `int`) semantics; the include guard is represented by `_LINUX_PCI_IDS_H = 1`.
- No conditional branches beyond the source include guard are active in this header.

Frozen hashes at seal:

- source SHA-256: `55928d1f2c4f7e6b912c54baf64b876fc8cd4d083d2016a45a87ca33ebc9439d`
- candidate SHA-256: `198a6f12bbb053e7f70ca55b3af8ffaba758159d4ebbfaf81041a6312834e24b`
- queue SHA-256: `4bec21517025faccfba156f8ff4fe8197b8170dde4cdba22891c96448182ad30`
- queue fingerprint file SHA-256: `9b1b6e2c647d1ee9b24ccf49666ce3f44f0537ed3d81b0768c9661f157fe0ba7`
- phase identity SHA-256: `6e2df070e502b65ad41d9eeb061a402cf7b0c9c158bdc3428006babfd2917381`
- pinned SHA file SHA-256: `7d3ae3944cd4d7a7d27b0df137485334e72bc9b9e04657abec78c4249ac9f692`
