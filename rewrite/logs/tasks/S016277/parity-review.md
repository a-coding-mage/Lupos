# S016277 parity review — slot 1

Role: parity reviewer (`gpt-5.6-terra`, high reasoning effort)

## Scope and frozen inputs

- Task remains `REVIEWING` on pipeline `P02`:
  `include/uapi/linux/netfilter/nf_tables.h` ->
  `src/include/uapi/linux/netfilter/nf_tables.rs`.
- Reviewed pinned Linux revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df` on the required branch.
- Verified the current Phase 0 queue fingerprint
  `d6c01f29edd048c73608fff2ed65f2485755023f85ffc0fec01e9f511fd6a72c`
  and identity binding
  `1c793a8db8b4971fdfc15cd7b9e5f503d6b4399893cb0cb5ab171f95c9e46a23`.

## Exhaustive source comparison

The fresh `SYMBOLS.tsv` inventory has 967 per-architecture records: four
preprocessor-control records, the C header guard, and 962 contract
identifiers.  The 962 contract identifiers are 115 enum tags, 730 enumerators,
and 117 object-like UAPI macros.  The candidate has exactly 115 matching type
aliases and 847 matching public constants; no identifier is absent or extra.

I evaluated all 730 C enumerator values in source order and all 117 object-like
macro expressions after their source aliases/bit operations, then compared the
847 resulting values with the candidate expressions.  All values and aliases
match.  This includes the negative verdict values, every `__*_MAX`/`*_MAX - 1`
relationship, compatibility alias `NFT_META_IIFTYPE`, and all masks.  The sole
unsigned value family is preserved as `u32`: `enum nft_data_types`,
`NFT_DATA_VALUE`, `NFT_DATA_VERDICT`, and `NFT_DATA_RESERVED_MASK` (value
`0xffffff00`).  The remaining enum and macro values are represented as `i32`,
matching their source integer range.

The complete source header has no include dependency, function declaration,
struct, union, bitfield, packed/alignment declaration, or layout-bearing UAPI
object.  Its ABI is therefore the enum integer representation and the exposed
integer constants only.  Both fresh ABI inventories list the same 115 enum
types at the same source lines for x86_64 and AArch64; the candidate maps all
but `nft_data_types` to `i32`, and maps that unsigned enum to `u32`.

The only operative conditional is `#ifdef __KERNEL__` around `NFT_REG32_MAX`.
The frozen Kbuild contexts define `__KERNEL__`, so that macro is selected and
is present with the source alias value.  The include guard and its control
records are preprocessor mechanics rather than a public Rust identifier.  No
other configuration conditional or architecture-specific branch exists in the
header.

The candidate retains the exact SPDX license expression and immutable
provenance fields for the pinned source, revision, `common` architecture class,
and task ID.  It adds no branding delta, test configuration, stub, panic, or
non-UAPI behavior.

## Findings

None.  The candidate is source-parity complete for the selected UAPI header.

No compiler, formatter, rust-analyzer diagnostic, build, test, debugger, or
runtime command was used.
