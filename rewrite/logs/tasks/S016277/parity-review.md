# Parity review — S016277 (slot 1)

Reviewed only the pinned Linux UAPI header, the fresh candidate, frozen task
context, and necessary current local header/caller context.  No compiler,
formatter, test, linker, rust-analyzer diagnostic, or historical Lupos source
was used.

## Findings

1. **`NFT_REG32_MAX`: C visibility/contextual-macro behavior is lost.**
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:49-51` defines
   `NFT_REG32_MAX` only while `__KERNEL__` is defined.  This conditional is
   selected and inventoried for both architectures in `rewrite/SYMBOLS.tsv`
   (the `ifdef@49`, `endif@51`, and `NFT_REG32_MAX` records for S016277).
   The candidate exports `pub const NFT_REG32_MAX = NFT_REG32_15;` without any
   equivalent kernel-only boundary at
   `src/include/uapi/linux/netfilter/nf_tables.rs:43`.  It therefore exposes a
   name that the pinned UAPI header deliberately withholds from non-kernel
   consumers.

2. **`enum nft_data_types`: candidate gives the enum tag an incompatible
   signed representation.**  The Linux declaration at
   `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:504-507` assigns
   `NFT_DATA_VERDICT = 0xffffff00U` (4,294,967,040), and the adjacent macro at
   line 509 retains the same unsigned value.  The frozen symbol inventory
   explicitly records both `enum nft_data_types` and `NFT_DATA_VERDICT` for
   x86_64 and aarch64; `rewrite/ABI.tsv` has their ABI/layout records pending
   at rows whose evidence is source line 504.  The candidate correctly spells
   the constant as `0xffffff00u32` at lines 277-281, but simultaneously maps
   the source enum type to `pub type nft_data_types = i32;`.  That signed alias
   cannot represent the recorded enum value.  This type is materially used in
   the local kernel contract: `vendor/linux/include/net/netfilter/nf_tables.h`
   lines 232-263 stores, returns, and passes `enum nft_data_types`, and
   `vendor/linux/net/netfilter/nf_tables_api.c:11948-11958` compares and
   switches on `NFT_DATA_VERDICT`.  Resolve the unsigned enum ABI from the
   pinned x86_64/AArch64 context rather than retaining an i32 alias.

3. **Provenance architecture field is contradictory.**  The frozen task row
   classifies S016277 as `architectures=common`, while the candidate has two
   immutable provenance fields: `//! architectures: x86_64,aarch64` at line 4
   and `//! architectures: common` at line 5.  The required provenance format
   permits one exact architecture field; these two declarations disagree and
   make the source’s frozen architecture provenance ambiguous.

All 115 source enum tags have a candidate type alias, and manual declaration
inventory found no candidate test configuration, placeholder panic, stub,
struct/union omission, or branding delta.  The findings above prevent a PASS.
