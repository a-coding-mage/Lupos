# Parity review — S016275

## Scope and inputs

- Task / pipeline / slot: `S016275` / `P02` / parity slot 1.
- Queue row observed in `REVIEWING`: `include/uapi/linux/netfilter/nf_log.h` -> `src/include/uapi/linux/netfilter/nf_log.rs`, architecture `common`.
- Branch observed: `feat/bun-like-rewrite-test`.
- Pinned revision observed from `vendor/linux.SHA`: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Compared complete pinned source `vendor/linux/include/uapi/linux/netfilter/nf_log.h` with the candidate and the S016275 frozen ABI/header-closure records for both `x86_64` and `aarch64`.

## Exhaustive source comparison

| Pinned UAPI macro | C value/type | Candidate declaration | Result |
| --- | --- | --- | --- |
| `NF_LOG_TCPSEQ` | `0x01` / C `int` | `pub const ...: core::ffi::c_int = 0x01` | exact |
| `NF_LOG_TCPOPT` | `0x02` / C `int` | `pub const ...: core::ffi::c_int = 0x02` | exact |
| `NF_LOG_IPOPT` | `0x04` / C `int` | `pub const ...: core::ffi::c_int = 0x04` | exact |
| `NF_LOG_UID` | `0x08` / C `int` | `pub const ...: core::ffi::c_int = 0x08` | exact |
| `NF_LOG_NFLOG` | `0x10` / C `int` | `pub const ...: core::ffi::c_int = 0x10` | exact |
| `NF_LOG_MACDECODE` | `0x20` / C `int` | `pub const ...: core::ffi::c_int = 0x20` | exact |
| `NF_LOG_MASK` | `0x2f` / C `int` | `pub const ...: core::ffi::c_int = 0x2f` | exact |
| `NF_LOG_PREFIXLEN` | `128` / C `int` | `pub const ...: core::ffi::c_int = 128` | exact |

The source has no enum, struct, union, function, exported linkage declaration, or layout/alignment requirement. Its only remaining preprocessor construct is the private include guard `_NETFILTER_NF_LOG_H`; Rust module loading provides the corresponding single-definition mechanism and it has no UAPI value/layout contract to expose. The candidate has no extra UAPI declarations.

The SPDX identifier is exact: `GPL-2.0 WITH Linux-syscall-note`. All four immutable provenance lines name the exact upstream path, pinned SHA, `common` architecture membership, and `S016275` task ID.

## Findings and verdict

No parity findings. The candidate preserves every operative UAPI constant name, integer value, and C-`int` representation identified by the frozen ABI records for both approved architectures.

Verdict: **ACCEPT**.
