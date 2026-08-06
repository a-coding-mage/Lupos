# Parity review — S016143 (slot 1)

Reviewed `vendor/linux/include/uapi/linux/hash_info.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/hash_info.rs`, the frozen x86_64/aarch64 scope rows,
the task symbol/ABI records, and selected pinned consumers.

## Finding P1 — the Rust public representation is not the C enum's public integer-constant surface

The candidate models the declaration as a closed Rust `#[repr(C)] enum`.
That preserves the sequence of named discriminants but does not reproduce the
C header's public interface: in C, `enum hash_algo` is an enum-category
integer type and every `HASH_ALGO_*` declarator is an unqualified ordinary
integer constant.  The candidate instead requires qualified Rust variants
(`hash_algo::HASH_ALGO_*`) and restricts values to the listed variants.

This distinction is operative in selected consumers.  The pinned
`include/crypto/hash_info.h:38-39` uses `HASH_ALGO__LAST` as an integral array
bound; `lib/crypto/hash_info.c:11-34` uses all enumerators as designated
integer array indices; `crypto/asymmetric_keys/pkcs7_verify.c:162-165` obtains
an `int` index and assigns it through an `enum hash_algo *`; and
`security/keys/trusted-keys/trusted_tpm1.c:800-806` iterates an `int` from zero
through the sentinel.  The UAPI type is unconditional in both frozen
configurations.

The applier must replace this closed/namespace-changing representation with a
representation that preserves the C integer category and exports each
unqualified public `HASH_ALGO_*` constant (including `HASH_ALGO__LAST`), while
first resolving the still-`PENDING_REVIEW` C enum ABI record in
`rewrite/ABI.tsv` for both architectures from Phase-0 evidence.  A raw
ABI-width integer alias plus public typed constants is the direct mechanism if
that evidence establishes the expected C integer type; do not infer the
underlying type merely from the current `#[repr(C)]` spelling.

## Verified items

- The candidate's provenance names the correct source, revision, task, and
  `common` architecture scope; its SPDX identifier and retained copyright are
  consistent with the pinned header.
- The source has one unconditional declaration and no configuration-dependent
  branch.  Both frozen configuration records select `enum hash_algo`.
- All 24 names are present, in source order, and their explicit values preserve
  the C implicit sequence exactly: `HASH_ALGO_MD4 = 0` through
  `HASH_ALGO_SHA3_512 = 22`, with `HASH_ALGO__LAST = 23`.
- No additional algorithm, branding delta, runtime logic, test, or placeholder
  was introduced.

Conclusion: revision required for P1; no source was edited by this reviewer.
