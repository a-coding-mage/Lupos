# Application resolution — S016277

Applied from the pinned Linux source at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.  The task remains the frozen
common header translation for `include/uapi/linux/netfilter/nf_tables.h`.
No compiler, formatter, linker, test, rust-analyzer diagnostic, or historical
Rust source was used.

## Parity review finding 1 — `NFT_REG32_MAX` conditional visibility

**Resolved.**  Upstream places `NFT_REG32_MAX` exclusively between
`#ifdef __KERNEL__` and its matching `#endif` at
`vendor/linux/include/uapi/linux/netfilter/nf_tables.h:49-51`.  The frozen
header-closure compile commands for both selected consumers define
`-D__KERNEL__`; the task's x86_64 and aarch64 conditional inventory records
the same condition and macro in `rewrite/SYMBOLS.tsv`.  The candidate now uses
`#[cfg(feature = "__KERNEL__")]` on this one constant, preserving its
kernel-only availability rather than exporting it to the non-kernel UAPI
context.

## Parity review finding 2 — `nft_data_types` signed representation

**Resolved.**  Upstream assigns `NFT_DATA_VERDICT = 0xffffff00U` in
`vendor/linux/include/uapi/linux/netfilter/nf_tables.h:504-507`; the same
unsigned value is retained by `NFT_DATA_RESERVED_MASK` at line 509.  It cannot
be represented by signed `i32`.  Both pinned UAPI ABI type headers define
`__u32` as `unsigned int` (`include/uapi/asm-generic/int-ll64.h:27` and
`include/uapi/asm-generic/int-l64.h:27`), and the pinned kernel uses
`enum nft_data_types` for the stored/passed `nft_data_desc.type` contract
(`include/net/netfilter/nf_tables.h:231-245`) and unsigned-range comparison
against `NFT_DATA_VERDICT` (`net/netfilter/nf_tables_api.c:11948-11958`).
The Rust alias is therefore `u32`, and both enumerator constants plus the
reserved mask are explicitly typed as `nft_data_types`.

## Parity review finding 3 — contradictory provenance architecture

**Resolved.**  The frozen S016277 queue and scope rows classify the header as
`architectures=common`.  The extra `x86_64,aarch64` provenance line was
removed; the immutable header now contains exactly the required `common`
architecture declaration.

## Rust review finding 1 — `nft_data_types` type/value mismatch

**Resolved.**  Same upstream evidence and source change as parity finding 2:
the `u32` alias and explicitly typed constants make `NFT_DATA_VALUE` and
`NFT_DATA_VERDICT` values of the declared type while preserving the unsigned
32-bit reserved value.

## Rust review finding 2 — erased `__KERNEL__` gate

**Resolved.**  Same upstream and frozen-command evidence as parity finding 1.
The Rust constant now carries the exact kernel-context gate
`#[cfg(feature = "__KERNEL__")]`.

## Rust review finding 3 — contradictory provenance architecture

**Resolved.**  Same frozen queue/scope evidence and source change as parity
finding 3.  The candidate declares only `//! architectures: common`.
