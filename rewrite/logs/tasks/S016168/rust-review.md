# Rust source review — S016168, attempt 1, P01

Reviewed `vendor/linux/include/uapi/linux/if_infiniband.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against the current
`src/include/uapi/linux/if_infiniband.rs`, the sealed proposal, and the direct
consumer context `net/ipv6/addrconf.c:2344-2350`. No compiler, formatter,
rust-analyzer, or runtime tooling was used.

## Finding RUST-S016168-001 — BLOCKER: candidate snapshot does not describe the reviewed source

`candidate.diff` (SHA-256
`fabcad884c4e98b3bd49cfc9ec1ef4f241dc95d17fb0a01082300ae6894d5f38`)
records a 39-line new file whose line 2 is `/* Pinned Linux header notices
preserved verbatim. */`. The current candidate (SHA-256
`65ae1b379fc01d667042856ad79e82e84655ae4adf7b460e1c5cdf7f319fef62`) is a
36-line file containing the upstream notice text instead and does not contain
that line. The semantic proposal seals the former diff hash, not an identifier
for the current source file. Consequently the candidate snapshot cannot prove
that the source reviewed here is the source described by the sealed proposal.

This is an evidence-integrity finding, not a change to any SC1 semantic field.
The applier must regenerate the candidate snapshot and proposal/seal for the
actual candidate, then obtain fresh reviews for that candidate before accepting
the task.

## Semantic and Rust audit

The only operative C value is `INFINIBAND_ALEN`, whose replacement token is
the unsuffixed literal `20` at upstream line 28. On both selected Linux ABIs it
has the `int` type because `20` fits in `int`; the direct use compares it with
`dev->addr_len`, which C promotes to `int` before comparison. The Rust `i32`
constant preserves that signed 32-bit literal domain. A Rust translation of a
typed `u8` caller must perform the corresponding explicit promotion; this
header does not introduce an unsound implicit conversion, a truncation, or a
different literal value.

The C include guard has only preprocessor inclusion semantics. Replacing it
with the Rust module boundary creates no UAPI object, exported symbol, layout,
calling convention, or runtime behavior. The file declares no FFI type,
pointer, reference, static mutable state, allocation, `unsafe` block, pinning,
interior mutability, callback, `Send`/`Sync`, `Drop`, endian, alignment, or
packing contract. The constant evaluation is non-panicking and non-allocating.

Subject to the snapshot finding above, I approve the semantic closure proposed
for SC1-b326ee0463b2d2499b745752d072ffca8548ccc10078694265e9360d815d398f
and the twelve architecture-specific SC1 records: each records only the
header's unconditional guard structure or exact macro replacement token and is
supported by `if_infiniband.h:25-30` for both frozen architectures.
