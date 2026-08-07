# Implementation S016277

- Lease: P02 / S016277, attempt 1; branch `feat/bun-like-rewrite-test`.
- Pinned source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h`, Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/netfilter/nf_tables.rs` (common x86_64/AArch64 scope).
- The complete UAPI header was mechanically translated: every enum identifier and enumerator is represented by a Rust type alias and typed public constant, and every object-like macro is represented by a public constant. C numeric suffixes and line continuations are removed without changing values or expressions. The `__KERNEL__`-guarded `NFT_REG32_MAX` is retained with a Rust feature gate.
- `nft_data_types` uses `u32` so `NFT_DATA_VERDICT = 0xffffff00U` and `NFT_DATA_RESERVED_MASK` preserve the unsigned Linux value.
- No compiler, formatter, linker, test, or historical Lupos source was used.
