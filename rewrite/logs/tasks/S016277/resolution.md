# S016277 application resolution — attempt 3

Pinned source reopened: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`. Current destination digest:
`e633dc83a457fa2cfd964ed06a9c46a530122e3981466d124d169bf7bd9efe42`.
This was a manual source review only; no compiler, formatter, linker, test,
rust-analyzer diagnostic, or historical Lupos source was used.

## Parity review dispositions

1. `F001` — **RESOLVED_CHANGED.** Upstream line 1 is exactly
   `GPL-2.0 WITH Linux-syscall-note`. The Rust SPDX line now uses that exact
   expression; the prior `-only` addition was removed. Source citation:
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:1`.

2. `F002` — **RESOLVED_NO_CHANGE.** Upstream's `#ifndef`/`#define`/`#endif`
   at lines 2--3 and 2022 prevent duplicate *textual C inclusion*. This path
   is a single Rust source module, not a textual-include payload, so there is
   no corresponding executable/exported guard declaration to add. Adding
   `_LINUX_NF_TABLES_H` as a Rust public constant would not implement the C
   preprocessor behavior and would introduce an extra API. Source citation:
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:2-3,2022`.

3. `F003` — **RESOLVED_CHANGED.** The selected kernel-only definition
   `NFT_REG32_MAX -> NFT_REG32_15` is restored as the kernel Rust module's
   `NFT_REG32_MAX`. The frozen x86_64 and AArch64 header-consumer commands in
   `rewrite/FILE_MAP.tsv:18951,23267` both carry `-D__KERNEL__`. Source
   citation: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:49-51`.

4. `F004` — **RESOLVED_CHANGED.** Every listed macro now names the exact
   upstream operand(s): `NFT_TABLE_F_MASK`, `NFTA_TABLE_MAX`,
   `NFT_CHAIN_FLAGS`, `NFTA_SET_FIELD_MAX`, `NFTA_SET_ELEM_MAX`,
   `NFTA_SET_ELEM_LIST_MAX`, `NFTA_FLOW_MAX`, `NFT_LOGLEVEL_MAX`,
   `NFTA_QUEUE_MAX`, `NFTA_DUP_MAX`, `NFTA_CT_HELPER_MAX`,
   `NFTA_FLOWTABLE_MAX`, `NFTA_FLOWTABLE_HOOK_MAX`, all tunnel `*_MAX`
   aliases, and `NFT_TUNNEL_F_MASK`. Source citations:
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:194,218,225,379,470,488,1215,1342,1360,1544,1640,1735,1751,1928-2020`.

5. `F005` — **BLOCKED.** This header declares `enum nft_registers` and
   `enum nft_data_types` but gives neither an explicit representation nor an
   ABI layout assertion. The latter has the unsigned literal `0xffffff00U`,
   which establishes the enumerator value but does not, from source alone,
   establish the representation of either enum for both pinned targets. The
   dependent pinned kernel header places `enum nft_data_types` in `struct
   nft_data_desc` and passes both enums by value, so this cannot be dismissed
   as unobservable. No permitted manual source evidence resolves the exact
   ABI; compiler/toolchain evidence is forbidden in Phase 1. Source citations:
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:22-46,504-507`;
   `vendor/linux/include/net/netfilter/nf_tables.h:232-252`.

## Rust review dispositions

1. `RUST-001` — **RESOLVED_CHANGED.** Same source-backed correction as
   `F003`: the selected `__KERNEL__` macro is now represented by
   `NFT_REG32_MAX = NFT_REG32_15` in the kernel Rust module. Source citation:
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:49-51`.

2. `RUST-002` — **RESOLVED_CHANGED.** Same source-backed operand repairs as
   the first six entries of `F004`. Source citations:
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:194,218,225,379,470,488`.

3. `RUST-003` — **RESOLVED_CHANGED.** Same source-backed operand repairs as
   the remaining entries of `F004`, including all tunnel aliases and mask.
   Source citations:
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:1215,1342,1360,1544,1640,1735,1751,1928-2020`.

4. `RUST-004` — **DISPROVED.** The reviewer compared different artifacts.
   `f9bc404a...` is the SHA-256 of `candidate.diff`, exactly as the sealed
   proposal's `candidate_sha256` field requires; `candidate.diff` itself
   records the then-current destination SHA-256 `448237c7...`, which matched
   the pre-application destination. The sealed proposal did not claim that
   its `candidate_sha256` was the destination-file digest. Evidence:
   `rewrite/logs/tasks/S016277/semantic-closure-proposal.tsv:2` and
   `rewrite/logs/tasks/S016277/candidate.diff:1-6`.

No semantic final/commit was prepared: `F005` remains unresolved, so a
source-only Phase 1 completion would be unsupported. The task must remain
`BLOCKED` pending a later authorized ABI-evidence workflow.
