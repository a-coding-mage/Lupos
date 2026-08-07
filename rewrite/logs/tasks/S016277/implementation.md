# Implementation S016277

- Lease: P02 / S016277; branch `feat/bun-like-rewrite-test`.
- Source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h` at revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/netfilter/nf_tables.rs`.
- Architectures: x86_64,aarch64 (common UAPI).
- Coverage: complete pinned header translation, including all enum type aliases, enum constants, aliases, masks, limits, and scalar UAPI macros. C enum auto-increment values and explicit expressions were preserved; the `NFT_META_IIFTYPE` compatibility alias and comments embedded in enum entries were handled explicitly.
- No compiler, formatter, test, linker, runtime, or historical-source inspection was used.

