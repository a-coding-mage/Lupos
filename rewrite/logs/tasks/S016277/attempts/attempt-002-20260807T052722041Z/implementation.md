Task S016277 attempt 2 implementation evidence.

Translated the complete pinned Linux UAPI header `include/uapi/linux/netfilter/nf_tables.h` into the path-preserving Rust destination. All enumerators, aliases, constants, masks, and the `__KERNEL__` conditional for `NFT_REG32_MAX` are represented with C-width integer constants and transparent enum-tag wrappers. Source revision: 425f94c2954b1fe80ebdbf9b29854e89750355df. Architectures: common.

Candidate SHA256 is recorded by the semantic-closure proposal and must be verified unchanged after sealing.
