# S016277 implementation

- Task: `S016277`
- Pipeline/attempt: `P02` / `3`
- Linux source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h`
- Destination: `src/include/uapi/linux/netfilter/nf_tables.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (frozen x86_64/aarch64 union)
- Lease owner: `codex-root-repair-20260807-p02`
- Translation: all enums and operative macros from the pinned 2022-line header were recreated as Rust type aliases and typed constants with explicit C enum values. `nft_data_types` is `u32`, preserving `NFT_DATA_VERDICT = 0xffffff00U`; other C enum domains retain signed `i32` representation. The `NFT_META_IIFTYPE` alias and declaration order are preserved. The `__KERNEL__`-only `NFT_REG32_MAX` branch is excluded because the frozen scope marks it not applicable; no unproven target conditional was introduced.
- No compiler, formatter, linker, test, runtime, or Git mutation was run.
- Destination SHA-256 at seal: `448237c7b9a9582058e9faac8c61685a42d65f3dbed459a2e309cd65f4c0a973`
