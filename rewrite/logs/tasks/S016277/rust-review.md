# Rust source review — S016277, attempt 3, slot 2

Result: **FINDINGS**.  This was a manual, source-only review; no compiler,
formatter, test, rust-analyzer diagnostic, or historical Lupos source was used.

Review binding: queue row is `REVIEWING`, `P02`, attempt `3`, with the pinned
source `include/uapi/linux/netfilter/nf_tables.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.  The proposal seal declares
identity `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
and queue fingerprint
`cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.

## Findings

1. **RUST-001 — kernel-only macro omitted.**  Upstream
   `nf_tables.h:49-51` defines `NFT_REG32_MAX` as `NFT_REG32_15` under
   `#ifdef __KERNEL__`.  The candidate has no equivalent conditional export
   between `NFT_REG_MAX` and `NFT_REG_SIZE` (Rust lines 48-51).  The selected
   kernel branch therefore loses a UAPI symbol rather than preserving the C
   condition.  Restore the symbol under the Rust configuration mechanism that
   represents the frozen kernel build; it must not become unconditionally a
   userspace UAPI export.  Closure keys:
   `SC1-91c69283317f93b278bca6df507aa8894ab78c89ae7a02ae146d3bae0c663824`,
   `SC1-08e33fd3d749293fdf31ae3b01814315e063f1f08d542c4e905be14b22302a77`.

2. **RUST-002 — malformed identifiers break the selected macro aliases.**
   The candidate substitutes identifiers which are not declared anywhere in
   the file: `NFT_TABE_*`/`__NFTA_TABE_MAX` (lines 194, 216),
   `NFT_CHAIN_HW_OFFOAD` (223), `__NFTA_SET_FIED_MAX` (375),
   `__NFTA_SET_EEM_MAX` (466), and `__NFTA_SET_EEM_LIST_MAX` (484).  Upstream
   uses the corresponding declared spellings in `nf_tables.h:194,218,225,379,
   470,488`.  These are not harmless aliases: each affected public constant
   cannot denote the C expression.  Closure keys:
   `SC1-bc6599774eb83ca323ed1aaa56a4adfaa5167bb04c285401840654ed127d7522`,
   `SC1-93f24579bfedca2e06f77593b78c28a947c8975a6f7fa4c7396f4434325d79ae`,
   `SC1-0e0a6b3df98069c5046330f6d9800818f41df0fe6cd3be8f46fb4fdb1a8ebed4`,
   `SC1-6fd895088545f7ce6278e74d3e2ad621cf6f4893c75c4a2ccda6e27e22f042ad`,
   `SC1-21647846e0abba93c05d61c77e4c7d5aa628b560c5a13dab1fdd0ee321eb211d`,
   `SC1-5d3d448e7392a9ee4cfe38f645ffeaaf25f0bc21ae8105567ba8e1ddbd015a2e`.

3. **RUST-003 — the same unresolved-identifier defect recurs through the
   latter UAPI.**  `NFTA_FLOW_MAX` (1210), `NFT_LOGLEVEL_MAX` (1337),
   `NFTA_QUEUE_MAX` (1355), `NFTA_DUP_MAX` (1539), `NFTA_CT_HELPER_MAX`
   (1635), `NFTA_FLOWTABLE_MAX`/`NFTA_FLOWTABLE_HOOK_MAX` (1729/1745), and
   all listed tunnel `*_MAX`/`NFT_TUNNEL_F_MASK` aliases (1922-2012) refer to
   misspelled `FOW`, `QUEE`, `DP`, `HEPER`, `FOWTABE`, `LOGLEVE`, or `TUNNE`
   names rather than their declared upstream enumerators.  This changes every
   corresponding macro's value from its C expression to an unresolved Rust
   path.  The source evidence is `nf_tables.h:1215,1342,1360,1544,1640,
   1735,1751,1928-2020`.  Closure keys:
   `SC1-c926da4f3520ce0c9ef4206261218b19f7ebb75e0c72041e18858f403a4e0b4b`,
   `SC1-a1b6d816ecad45edf4297e58ac98da64d46a150eca7cc14aae16aa86022b5ef0`,
   `SC1-43421c10d8197caf41b8f4fa4c0584abaa9dd6f2ebae98af1e3c1d88d0cea1f1`,
   `SC1-71f41fd53e3ff82e0eb099880811585079789e9a1c3c7efc2787d2790ccdd657`,
   `SC1-d913041b25beb28537de6ef8a1cdef4aedaba7b534bfd186e2eed98c2113933c`,
   `SC1-80be746d65a610ea344c6b73f2338139e08fa633b6874b1013d1815c1d8af02e`,
   `SC1-3dc66aac0bc01d4bdeedbc5473fed9807c7105b09f7bf241595ec68c9380a2a0`,
   `SC1-ed5c45d2c77d604ad3c673a2357eb04bdfdc01bd9db9644714256faf16480550`,
   `SC1-49d9ad3a2521af9ced757dd5838c13566573d5fa67502e5ed1b42f0a34b896b1`,
   `SC1-a82db53ab4c8104c3bf6d35b5e3aa34e8a2ba111bb014494a2dbe728877fe57e`,
   `SC1-baee217b9c2fbe7be5dd3f65ef10607ab424b3d65bd767c43d8a72fe21d38346`,
   `SC1-bf46b63fbcef3f6757f5b0f3fc6e8f05fce13d9f62bf09f2169dca4ae7963e7e`,
   `SC1-3e6f222f04438b285e1599c1c5a9d875a53aefd7d9157226975952daf1dadb4e`,
   `SC1-55129bc90b8a1ed44b810a2952f07aced54bb10ee207d0b93daaf99ca015d779`,
   `SC1-8433c6e8b87bb2bad7e578508113c9c737a3683d2203924a818dea9baa8edbd7`,
   `SC1-9f8dc3b6cb2037579decf421187c8fa2d432108c0c479c12c0dd9f7fa42c19b1`,
   `SC1-72f89999395496933c5b4b493c3b4654115c28b39be0a18c1e65109fc3469619`,
   `SC1-a290bc388853b0f6c0c69a119962f36dc0d87663ce6d7b3095e3892e27a98341`.

4. **RUST-004 — sealed candidate binding no longer matches the candidate
   under review.**  The current source hashes to
   `448237c7b9a9582058e9faac8c61685a42d65f3dbed459a2e309cd65f4c0a973`,
   while this attempt's sealed proposal records candidate hash
   `f9bc404a3e57f6ebbd09d763cce679b80cfbac989b82c4bbf9752531370d87cc`.
   A slot-2 attestation cannot truthfully bind this review to the sealed
   proposal.  Closure scope key:
   `SC1-7891aee3610f34ec984fc67dc1794d5a5d477b7c3dab2e557a6fa740fa5346e2`.

## Manual Rust-semantics audit

All non-`nft_data_types` enumerator aliases are represented as signed 32-bit
constants, and `nft_data_types` uses `u32`, preserving the source's explicit
`0xffffff00U` (`nf_tables.h:504-510`) without sign extension.  This header has
no aggregate, union, bitfield, pointer, callback, atomic, ownership, pinning,
interior-mutability, `Send`/`Sync`, `Drop`, or FFI function surface, so no
`repr(C)` aggregate or unsafe boundary is required in this file.  I found no
unsafe block/function, panic/unwrap/expect, test configuration, or project
authored test.  These checks do not mitigate Findings 1-4.
