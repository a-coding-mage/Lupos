# S016277 implementation

- Lease: P02, retry attempt 2; destination `src/include/uapi/linux/netfilter/nf_tables.rs`.
- Source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h`, Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Architectures: `common` (the source row is common to x86_64 and AArch64).
- The fresh file is generated directly from the complete 2022-line pinned header. It contains every selected enum identifier and object-like macro; no C continuation backslashes remain.

## Semantic proposal and source evidence

- Source line 1 SPDX is preserved exactly as `GPL-2.0 WITH Linux-syscall-note`.
- Lines 5-11 define `NFT_NAME_MAXLEN=256`, its table/chain/set/object aliases, `NFT_USERDATA_MAXLEN=256`, and `NFT_OSF_MAXGENRELEN=16`; these are emitted before/alongside the corresponding public constants without changing values.
- `enum nft_registers` (lines 22-45) is represented as `i32` aliases and constants; `NFT_REG_MAX` is `(__NFT_REG_MAX - 1)`, `NFT_REG_SIZE=16`, `NFT_REG32_SIZE=4`, and `NFT_REG32_COUNT=(NFT_REG32_15-NFT_REG32_00+1)` (lines 47, 53-55).
- The `__KERNEL__` branch at lines 49-51 is represented by the target-architecture kernel mapping comment and `#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]` `NFT_REG32_MAX = NFT_REG32_15`; it is not emitted as an unconditional userspace macro.
- Signed verdict values at lines 68-75 remain `i32` values `-1` through `-5`.
- Every C enum is exposed as a fixed-width Rust integer type alias (`i32`, preserving C enum width) with constants retaining exact explicit/implicit values. This avoids nominal Rust enum conversions and preserves arbitrary C enum values.
- `enum nft_data_types` at lines 504-508 is explicitly `u32`: `NFT_DATA_VALUE=0` and `NFT_DATA_VERDICT=0xffffff00U` represented as `4294967040`; `NFT_DATA_RESERVED_MASK` at line 509 is `0xffffff00u32`. This preserves the unsigned ABI value.
- The in-enum `#define NFT_META_IIFTYPE NFT_META_IFTYPE` at line 978 remains a separate alias; `NFT_META_OIFTYPE` follows `NFT_META_IFTYPE` in source order (line 979), preserving ordering.
- Multi-line masks (`NFT_TABLE_F_MASK`, `NFT_CHAIN_FLAGS`, `NFT_INNER_MASK`, `NFT_TUNNEL_F_MASK`) are emitted as single Rust constant expressions with the same bitwise operands and ordering.
- All `*_MAX` macros retain their source expressions (`__MAX - 1`), and all literal suffixes are valid Rust (`u32`, `u64`, `i64` where present).

No tests, stubs, indexes, compiler/build/format commands, or external source were used.
