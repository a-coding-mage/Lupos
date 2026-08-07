# S016277 implementation

- Task: `S016277`; pipeline `P02`; attempt `1`; lease owner `codex-root-repair-20260807-p02`.
- Source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/netfilter/nf_tables.rs`; architecture union `common`.
- Scope: all enum tags, enum constants, aliases, and operative macros selected in `rewrite/SYMBOLS.tsv` (including the complete 2022-line header). No structs or callable symbols occur in the pinned source.
- Representation: C enum tags are `i32` aliases and constants retain source values/order. `NFT_DATA_VERDICT` and `NFT_DATA_RESERVED_MASK` retain the unsigned `0xffffff00U` representation as `u32`. C bitwise expressions are represented with Rust bitwise operators; continued C lines are joined without backslashes.
- Source traps addressed: `NFT_NAME_MAXLEN` and dependent name limits are 256; `NFT_META_OIFTYPE` remains in its exact enum position after `NFT_META_IFTYPE`; `NFT_META_IIFTYPE` remains the source alias; all max/mask expressions preserve their source relationships. Frozen compile-command evidence defines `__KERNEL__` for both approved architectures, so `NFT_REG32_MAX` is emitted as the selected kernel macro value without inventing a Rust feature flag.
- Semantic proposal closure: UAPI values have no ownership or runtime lifetime; enum tags are integer ABI names. No unresolved semantic decision remains for this task.
