# S013468 final resolution

## Scope and source recheck

I reopened the complete pinned source
`vendor/linux/include/linux/asn1.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate
`src/include/linux/asn1.rs`, the frozen x86_64 and aarch64 configurations,
the header-closure evidence, direct ASN.1 headers, and selected consumers.
The task is the unconditional common header selected for both targets; its
only conditional structure is the C include guard.

The candidate is complete and requires no source correction.  It retains the
three named C enum tags as `core::ffi::c_int` aliases and each enumerator as a
`c_int` constant.  This is the selected contract: C enumerator expressions are
`int`-valued and selected consumers use them in byte comparisons, masks,
shifts, and bitwise-or expressions.  There is no selected object declaration,
function parameter, or exported ABI carrying any of the three nominal enum
tags.  The `asn1_tag` constants deliberately leave values 14 and 15 reserved,
exactly as upstream.  All three macro literals and their integer-expression
types are preserved.

## Review dispositions

1. Parity review: accepted.  I independently confirmed the complete upstream
   enumerator sets (`asn1_class` 0..3, `asn1_method` 0..1, and `asn1_tag`
   0..13 plus 16..31), the two reserved gaps, and macro values `0xc0`, `0x20`,
   and `0x80`.  No parity finding requires a change.
2. Rust review: accepted.  The aliases avoid manufacturing Rust-enum validity
   constraints where the pinned C interface provides unrestricted integer
   expressions.  The file has no unsafe code, layout-bearing object,
   allocation, ownership protocol, panic/placeholder, test configuration, or
   unauthorized branding.  No Rust finding requires a change.

## Semantic-record closure

- All 18 S013468 `SYMBOLS.tsv` rows now identify the include-guard-only
  conditions, exact macro replacements, and the always-selected enum
  declarations for both frozen targets.
- All six `ABI.tsv` rows now record the target C `int` scalar contract
  (`size=4`, `align=4`, `core::ffi::c_int` alias), integer-expression
  enumerators, and the intentional `asn1_tag` 14/15 gap, citing the pinned
  lines and each frozen target's Phase-0 compile-command evidence.
- All six `LIFETIMES.tsv` rows now record `NONE` ownership: this header has no
  object storage, allocation, aliasing, callback, destruction, locking, RCU,
  or refcount lifecycle.
- No function, static object, export, calling-convention, lock/RCU, refcount,
  callback, or driver-ABI row exists for this constant-only header; each is
  therefore not applicable rather than an unresolved semantic family.

No compiler, formatter, linker, test, runtime, or benchmark command was run.
