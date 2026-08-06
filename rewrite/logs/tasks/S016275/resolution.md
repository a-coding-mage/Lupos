# Applier resolution — S016275

## Inputs independently reopened

- Pinned source: `vendor/linux/include/uapi/linux/netfilter/nf_log.h:1-15`
  at revision `425f94c2954b1fe80ebdbf9b29854e89750355df` from
  `vendor/linux.SHA`.
- Frozen task records: the `S016275` rows in `rewrite/SCOPE.tsv`,
  `rewrite/SYMBOLS.tsv`, `rewrite/ABI.tsv`, and `rewrite/LIFETIMES.tsv`, for
  both `x86_64` and `aarch64`.
- Candidate: `src/include/uapi/linux/netfilter/nf_log.rs:1-15`.
- Independent reports: `parity-review.md` and `rust-review.md`.

No compiler, formatter, rust-analyzer, build, linker, test, debugger, or
runtime tool was invoked or used as evidence.

## Review-report dispositions

| Report | Finding | Disposition |
| --- | --- | --- |
| Parity | No finding; eight UAPI macro names, values, and C `int` representations match. | Accepted after independent full-header comparison. |
| Rust | No finding; the constant-only header adds no ownership, layout, linkage, unsafe, or drop surface. | Accepted after independent source inspection. |

## Independent source adjudication

The complete upstream header defines only the private include guard and the
following eight object-like integer macros.  The candidate exports each public
UAPI name as `core::ffi::c_int`, preserving the source's C `int` type and
exact value on both approved 64-bit Linux architectures:

| Source macro | Source value | Candidate result |
| --- | ---: | --- |
| `NF_LOG_TCPSEQ` | `0x01` | exact `c_int` constant |
| `NF_LOG_TCPOPT` | `0x02` | exact `c_int` constant |
| `NF_LOG_IPOPT` | `0x04` | exact `c_int` constant |
| `NF_LOG_UID` | `0x08` | exact `c_int` constant |
| `NF_LOG_NFLOG` | `0x10` | exact `c_int` constant |
| `NF_LOG_MACDECODE` | `0x20` | exact `c_int` constant |
| `NF_LOG_MASK` | `0x2f` | exact `c_int` constant |
| `NF_LOG_PREFIXLEN` | `128` | exact `c_int` constant |

The header has no include dependency, type, enum, struct, union, function,
storage object, linkage declaration, configuration-selected alternate branch,
layout, alignment, calling convention, ownership, locking, refcount, RCU, or
cleanup contract.  Its sole conditional (`_NETFILTER_NF_LOG_H`) is a private C
multiple-inclusion guard, not a UAPI value or ABI item; the Rust module has no
corresponding exported item.  The absence of S016275 rows in the frozen ABI
and lifetime ledgers is therefore correct: this source creates none.

All S016275 `PENDING_REVIEW` symbol entries are closed by this source evidence
for both architectures: the guard/`#ifndef`/`#endif` have no Rust ABI mapping,
and every listed operative UAPI macro maps to the exact constant above.  The
candidate has no extra declarations.  Its SPDX identifier is exactly
`GPL-2.0 WITH Linux-syscall-note`, and all immutable provenance fields name
the exact source path, revision, `common` architecture scope, and task ID.

## Final disposition

No source change is required.  S016275 is accepted for `DONE` as a complete
translation-only, source-reviewed UAPI constant header.
