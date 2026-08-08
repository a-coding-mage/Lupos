# Application resolution — S013468 (attempt 1, P01)

## Outcome

**BLOCKED — do not accept the candidate or semantic-closure proposal.**

The candidate was left unchanged.  The source establishes the enumerator
values and their integer-expression uses, but it does not establish an exact
cross-target representation and value-domain contract for the three named C
enum types.  Choosing a Rust enum representation or silently replacing those
public types with a primitive integer would be a new design decision.

## Reopened evidence

- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Complete header: `vendor/linux/include/linux/asn1.h:1-65`.
- Direct header macro consumer: `vendor/linux/include/linux/asn1_ber_bytecode.h:75-76`.
- Selected direct consumers: `vendor/linux/lib/asn1_encoder.c:42`,
  `vendor/linux/lib/asn1_decoder.c:74,86,99-101,230,246,262-264`,
  `vendor/linux/crypto/asymmetric_keys/pkcs7_parser.c:440`,
  `pkcs7_verify.c:115`, and `x509_cert_parser.c:565,650`.
- Candidate snapshot SHA-256:
  `3a82e9ca1d043502948555e159dadefa8b28fe8ac3b1e871fba5e23dd9cb37f9`.
- Both semantic review attestations bind that same candidate snapshot and the
  proposal SHA-256 `37eac526373769c23d2ef1e67ff97d9263b38bca73b0e5ffa1fad9784dd6cb91`.
- Frozen `ABI.tsv` rows for `enum asn1_class`, `enum asn1_method`, and
  `enum asn1_tag` on x86_64 and aarch64 retain `PENDING_REVIEW` for both
  layout and alignment.

## Finding dispositions

### P1 — enumerator and macro integer-expression interface

**Accepted; candidate rejected.**

`asn1.h:12-60` introduces file-scope C enumerators, and `_tag` in
`asn1_ber_bytecode.h:75` shifts and ORs them.  The direct consumers above also
compare and combine the values with byte masks.  The candidate instead
re-exports Rust enum variants and fixes the object-like macros as `i32`.
Neither representation preserves the C header's unqualified integer-constant
expression mechanism at every use boundary.  This is independently confirmed
by the parity report's P1 and the Rust report's RUST-2.

Providing top-level primitive constants would repair the demonstrated
enumerator-value uses, but cannot by itself preserve the public named C enum
types; no source edit is applied in this resolution-only pass.

### P2 — named C enum ABI records

**Upheld; terminal blocker.**

The header spells the enumerators but fixes no underlying enum type, layout,
alignment, packing, or cross-language valid-value domain.  The frozen ABI
records deliberately retain those details as `PENDING_REVIEW` on both selected
architectures.  The direct source uses prove integer expressions and
byte-oriented consumers, not that `#[repr(C)]` has the identical size,
alignment, FFI calling/storage contract, or arbitrary representation behavior
for each named enum on both targets.

Phase 1 forbids a compiler or layout probe.  Retaining the candidate's Rust
enums, replacing them with `i32`, or selecting any other representation would
therefore be unsupported.  The proposal's conversion of the ABI/lifetime
records to `COMPLETE` is rejected.

### RUST-1 — Rust enum restricted value domain

**Upheld; same terminal blocker as P2.**

No pinned source evidence limits all named enum objects at public C/FFI
boundaries to listed discriminants.  A Rust enum's valid-discriminant contract
cannot be accepted as the C enum contract without the missing ABI/value-domain
evidence.

### RUST-2 — Rust variants are not C integer constants

**Accepted; same repair requirement as P1.**

The source-level shifts, ORs, comparisons, and assignments cited above require
integer-expression values at their use boundaries.  Re-exported Rust enum
variants do not establish that contract.

## Required queue disposition

Both review reports and their independent attestations are present.  Enter
`APPLYING`, then mark this attempt `BLOCKED` through the atomic queue tool for
the unresolved x86_64/aarch64 named-enum ABI, layout, alignment, and value-domain
contract.  Do not produce a semantic final, commit semantic closure, or mark
`DONE`.
