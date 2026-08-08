# Rust review — S012620 (slot 2)

## Scope and frozen inputs

- Candidate: `src/include/crypto/dh.rs`.
- Oracle: `vendor/linux/include/crypto/dh.h` at Linux
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen architecture: `aarch64`; config SHA-256
  `b374fc24835033cb3e317e45c89dd2ba3335ebac7ab81b2ac548fac5bffc1578`.
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.
- Phase-0 identity binding: `03f3c4afb3c7edc167ddeadac5493cbee736042cb7781182d4fdf43b2b79166d`.
- Semantic-closure frozen inputs: base
  `aef2f1fa3f0822a588eae6ed4bf97f1d2bf4d66f3dfb2311dc2c52aa7a6cdecc`,
  schema `d4217408de5a1cd609d92bebbd04b07a4f2d2f3354fb6d86512891c4ec67b6fd`,
  task-keyset `9348fd13ca44935e847cb943d8f69a5c3f13fef8bc8d8ecaf44cc142e80e3a4c`.

Only the pinned header, candidate, focused candidate diff, and S012620 rows in
the frozen task/scope/symbol/lifetime/ABI records were inspected. No compiler,
formatter, test, Git command, historical source, or prior review was used.

## Source findings

### RUST-1 — unsafe FFI caller contracts are incomplete (must resolve)

The `unsafe extern "C"` declarations correctly leave calls unsafe, but their
Rust documentation does not state the caller obligations required by the
pinned declarations. In particular, it does not state that `params` must be a
valid readable `struct dh` for `crypto_dh_key_len`; that encode requires a
writable `buf` of at least the advertised packet-key length and valid readable
key/P/G regions for their recorded sizes; or that both decode declarations
require a readable `buf` for `len` bytes and a writable `params`. The pinned
header further specifies that decode stores pointers into `buf`; its lifetime
must outlive every use of those fields, and the caller must preserve suitable
aliasing/synchronization for the borrowed packet bytes. The unchecked helper
has the same raw-pointer obligations without the exported helper's basic
parameter checks.

This is not a request for safe references or a lifetime-bearing redesign: raw
pointers are the appropriate ABI representation. The applier must document
these caller obligations on the imported unsafe functions and close the
corresponding frozen lifetime records from the pinned source. Do not add a
safe wrapper that changes the C API, allocation, validation, or aliasing
mechanism.

## Verified Rust/ABI properties

- `#[repr(C)]`, field order, pointer constness, and `c_uint`/`c_int`/`c_char`
  mappings retain the header's C ABI on the frozen AArch64 target. The struct
  has no bitfields, packing directive, union, endian conversion, or by-value
  FFI parameter requiring another representation.
- `key`, `p`, and `g` remain raw non-owning `*const c_void` values. The
  candidate creates no Rust reference, borrow, pinning assertion, allocation,
  drop action, callback, refcount, interior-mutability wrapper, or pointer
  arithmetic. Accordingly it does not strengthen the C aliasing or lifetime
  contract.
- `Copy, Clone` performs the same shallow descriptor copy available for the C
  aggregate and introduces no ownership transfer or destructor. It does not
  make pointee storage owned, shared, pinned, `Send`, or `Sync`.
- The four C names and their C calling convention, argument pointer
  mutability, and integer return/parameter widths are preserved. `extern "C"`
  is an import declaration, so this header task does not claim to supply the
  helper implementations.
- There are no `unsafe` blocks, so no unsafe-block safety comment is missing.
  No panic path, bounds-checked indexing, allocation, eager fallback, or
  Rust-owned cleanup was introduced.

## Required semantic-record closure

The S012620 rows in `SYMBOLS.tsv`, `LIFETIMES.tsv`, and `ABI.tsv` still mark
the selected header condition/macro and `struct dh` representation, ownership,
and lifetime fields `PENDING_REVIEW`. Before `DONE`, the applier must close
them with these pinned-source facts: `struct dh` is a caller-owned,
non-owning descriptor; its three pointer fields refer to caller-provided
storage; the six fields are ordered exactly as declared; and decode makes its
output pointer fields alias the supplied packet buffer. The Rust candidate
itself has no code-level ABI mismatch after RUST-1 is resolved.

## Disposition

**REJECT PENDING RUST-1 AND SEMANTIC-RECORD CLOSURE.** Source-only review;
manual inspection only.
