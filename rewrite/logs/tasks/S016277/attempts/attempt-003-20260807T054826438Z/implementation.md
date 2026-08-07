# Implementation S016277 (attempt 3)

- Source: vendor/linux/include/uapi/linux/netfilter/nf_tables.h
- Destination: src/include/uapi/linux/netfilter/nf_tables.rs
- Linux revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
- Architectures: common
- Lease: P02, attempt 3

Translated the complete pinned UAPI header: all constants, enum declarations and progression, aliases, masks, and the __KERNEL__ conditional NFT_REG32_MAX branch. C literal suffixes were converted to Rust-valid typed suffixes (u32/i32). NFT_META_IIFTYPE alias and NFT_META_OIFTYPE progression are preserved. No historical Rust source, compiler, formatter, tests, or index changes were used.

Source citations: nf_tables.h lines 1-2022; frozen scope/symbol inventory row S016277 in rewrite/SYMBOLS.tsv and rewrite/SCOPE.tsv.

