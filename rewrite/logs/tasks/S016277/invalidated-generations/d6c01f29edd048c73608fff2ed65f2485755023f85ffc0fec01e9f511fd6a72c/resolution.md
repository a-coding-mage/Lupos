# S016277 applier resolution

Task `S016277` remains the pinned, common-architecture mapping
`include/uapi/linux/netfilter/nf_tables.h` to
`src/include/uapi/linux/netfilter/nf_tables.rs`.  I independently reopened the
complete 2,022-line upstream header at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the fresh Phase 0 identity and
task records, the candidate, and both fresh review reports.  The branch is
`feat/bun-like-rewrite-test`; the verified immutable queue fingerprint is
`d6c01f29edd048c73608fff2ed65f2485755023f85ffc0fec01e9f511fd6a72c` and the
Phase 0 identity binding is
`1c793a8db8b4971fdfc15cd7b9e5f503d6b4399893cb0cb5ab171f95c9e46a23`.

## Review dispositions

1. Parity review: accepted.  A separate identifier extraction gives the same
   962 contract identifiers in source and candidate: 115 enum tags and 847
   public constants (730 enumerators plus 117 object-like macros).  There are
   no missing or extra contract names.  I confirmed the signed verdicts,
   explicit flag values, aliases, mask and `*_MAX` relationships, the
   compatibility alias `NFT_META_IIFTYPE`, and the unsigned
   `0xffffff00U` data range.
2. Rust review: accepted.  The source has no function, extern declaration,
   struct, union, field, bitfield, packing, alignment, or call-convention
   declaration.  The candidate is likewise declarations/constants only, with
   no `unsafe`, pointer, reference, allocation, panic, test, or FFI surface.
3. No source change is required.  The candidate retains the exact SPDX
   expression and required immutable provenance and introduces no branding
   change.

## Semantic-record closure

The fresh Phase 0 rows for this header deliberately retain their mechanical
`PENDING_REVIEW` status; this resolution closes their semantic content for both
frozen targets, rather than changing frozen Phase 0 artifacts.

- All 115 enum-type ABI/lifetime rows for each of x86_64 and AArch64 are
  resolved as 32-bit scalar enum contracts with 4-byte alignment: `i32` for
  every tag except `nft_data_types`, which is `u32` because its enumerator and
  `NFT_DATA_RESERVED_MASK` are `0xffffff00U`.  The header declares no linked
  object or callable symbol, so linkage and calling convention are not
  applicable.
- All 730 enumerator rows and 117 object-like macro rows are resolved as
  compile-time integer contracts only: no storage duration, allocation,
  ownership, lifetime, locking, RCU, refcount, or cleanup behavior exists.
  The candidate preserves the C source values/aliases and operator relations.
- The header guard is preprocessor-only.  The only non-guard conditional,
  `#ifdef __KERNEL__` at source lines 49--51, is selected in both frozen
  kernel contexts; its `NFT_REG32_MAX` alias is present.  No other
  architecture or configuration branch exists.
- Therefore all task-scoped `PENDING_REVIEW` semantic questions in
  `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` have a final disposition here;
  none remains unresolved for this task.

No compiler, formatter, rust-analyzer diagnostic, build, test, linker,
debugger, or runtime command was used.
