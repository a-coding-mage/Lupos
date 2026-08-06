# Parity review — S012618

Reviewed pinned `vendor/linux/include/crypto/ctr.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/crypto/ctr.rs`, for the frozen common (`x86_64,aarch64`) task
scope.

## Result

No parity findings.

The candidate preserves all three selected operative macros with their exact
Linux names and literal values: `CTR_RFC3686_NONCE_SIZE = 4`,
`CTR_RFC3686_IV_SIZE = 8`, and `CTR_RFC3686_BLOCK_SIZE = 16`.  Each literal is
represented as `core::ffi::c_int`, matching the type of an unsuffixed decimal
integer constant in the C header for the selected targets.  The source has no
conditional configuration branches beyond the C include guard; Rust module
inclusion provides the corresponding single-definition property.  No functions,
types, statics, ABI/linkage items, or allowlisted branding deltas exist in the
upstream header.

The immutable provenance identifies the exact source path, revision, common
architecture scope, and task ID.  Source inspection only; no compiler,
formatter, analyzer, build, or test tooling was used.
