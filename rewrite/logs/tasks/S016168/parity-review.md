# S016168 parity review — FINDINGS

Reviewed only the pinned Linux UAPI header, the current candidate, its sealed
candidate diff/proposal, the S016168 frozen rows, and the direct frozen header
consumer context. No compiler, formatter, linker, test, rust-analyzer, or
historical Lupos source was used.

## Finding PR1 — the selected public C include guard is omitted

- Linux symbols/branches: `_LINUX_IF_INFINIBAND_H`, `ifndef@25`, and
  `endif@30`.
- Local evidence: `vendor/linux/include/uapi/linux/if_infiniband.h:25-30`
  publishes the `#ifndef`/`#define`/`#endif` guard, and `SYMBOLS.tsv` has those
  conditional records plus the guard macro selected for both `aarch64` and
  `x86_64`. The current candidate at
  `src/include/uapi/linux/if_infiniband.rs:29-30` explicitly substitutes a
  Rust module boundary and emits no equivalent public preprocessor guard.
- Parity impact: a Rust module boundary neither defines nor exposes the C
  macro used by the UAPI header contract. The candidate therefore omits a
  selected macro and changes the conditional/public namespace mechanism;
  the proposal's `COMPLETE` decisions for these records are unsupported.
- SC1 mapping: `SC1-f9c748f96188fdc333c96cce609ae4f5a8b46ddeb8537e514cc14a69fb720c36`,
  `SC1-be45e3d6d1d1b2fa93d61c87f2cb4d5227eea99d0a8b12d3108ac78e1601ac16`,
  `SC1-2c210f1812866ab400f39957a86d58469de18c4703ca1adee2ff05ea9810027c`,
  `SC1-097f4bbebcc14a81e92ec4d07a7c34bba2feaec7d4c726b784be354c024867d9`,
  `SC1-162b83e375e646704d59d9e17c485178a5255ee763de595dbf7803a3d3c334b3`,
  `SC1-ce07edbda0bf845d53767d3048d17ea9cc749d73aa1ed764dd9eec90a4f8f7d9`,
  `SC1-f41c8c58d2bf479eadfa540cfa05d60dbf9cc25b06a602da3bd2e18308b83f59`,
  `SC1-9b79cc7f63fdd6dc5e98c6617ffc16aafc68d7d2c99391fc72ba1b20fa76c56e`.

## Finding PR2 — `INFINIBAND_ALEN` changes from a public macro token to a typed Rust item

- Linux symbol: `INFINIBAND_ALEN`.
- Local evidence: the pinned header defines `#define INFINIBAND_ALEN 20` at
  `vendor/linux/include/uapi/linux/if_infiniband.h:28`; it has no declared
  type and expands in its C caller's expression context. The sole frozen
  header-closure consumer is `net/ipv6/addrconf.c` for both architectures;
  that source includes the header at line 53 and compares
  `dev->addr_len != INFINIBAND_ALEN` at line 2346. `struct net_device` declares
  `addr_len` as `unsigned char` in `include/linux/netdevice.h:2311`, so C's
  normal integer promotions govern that use. The candidate instead publishes
  `pub const INFINIBAND_ALEN: i32 = 20` at
  `src/include/uapi/linux/if_infiniband.rs:36`.
- Parity impact: the value `20` is retained, but a typed Rust constant is not
  the untyped C preprocessor replacement token or its C UAPI namespace. The
  candidate's `i32` assertion selects one Rust expression context rather than
  preserving the selected macro's context-dependent expansion. No source
  evidence establishes an exact ABI-compatible bridge for C UAPI consumers.
- SC1 mapping: `SC1-3b195e8d4b5c47f2fbe0e4029fd3ee81eb4c61cc5c8ace6c84cef103f9557c39`,
  `SC1-f2823575495581bf2bc08476473993d101e8c5943c1af2e1b7658cb4296a0500`,
  `SC1-0c4f1fc56290a075b3151b086b07995f1eeac06a16186a88836af285476a1d00`,
  `SC1-bd12cea1a6100a797e0bfd5fb391bca1b948c3d364e46981959430df8fee15cf`.

## Finding PR3 — candidate snapshot is not the reviewed source

- Linux symbol: `INFINIBAND_ALEN` (the only operative non-guard symbol in the
  translation unit).
- Local evidence: the sealed proposal binds candidate hash
  `fabcad884c4e98b3bd49cfc9ec1ef4f241dc95d17fb0a01082300ae6894d5f38`, which
  is the hash of `rewrite/logs/tasks/S016168/candidate.diff`. That all-additions
  patch contains the synthetic line `/* Pinned Linux header notices preserved
  verbatim. */` and omits the current candidate's actual source lines 2-23,
  29-35. The current candidate hash is
  `65ae1b379fc01d667042856ad79e82e84655ae4adf7b460e1c5cdf7f319fef62` and
  contains the full Linux notice plus the guard and `i32` rationale. Thus the
  patch cannot serve as the candidate snapshot for the source that defines
  `INFINIBAND_ALEN`.
- Parity impact: the sealed semantic proposal and this source review cannot be
  reproducibly tied to one candidate. In particular, the snapshot does not
  disclose the mechanism changes in PR1 and PR2, so its claimed fidelity and
  any `COMPLETE` closure based on it are not auditable.
- SC1 mapping: `SC1-b326ee0463b2d2499b745752d072ffca8548ccc10078694265e9360d815d398f`.

The current source retains the exact SPDX expression, the upstream notice, the
macro spelling, and the numeric value `20`; no unauthorized Lupos branding was
observed. The findings above nevertheless reject semantic approval because the
selected UAPI macro/guard mechanisms and review snapshot are not parity-safe.
