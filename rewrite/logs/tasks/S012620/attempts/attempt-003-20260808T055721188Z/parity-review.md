# Parity review — S012620 (slot 1)

## Result

PASS.  Manual, source-only comparison found no missing selected symbol, branch,
ABI declaration, layout field, linkage difference, mechanism change, or
unallowlisted branding in `src/include/crypto/dh.rs`.

## Frozen inputs

- Task row: `S012620`, `include/crypto/dh.h` →
  `src/include/crypto/dh.rs`, architecture `aarch64`, pipeline `P01`, attempt
  `3`.
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching
  the candidate provenance.
- Phase-0 identity SHA-256:
  `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`.
- Translation-task immutable-field SHA-256:
  `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.

## Exhaustive source evidence

- `struct dh` — Linux `include/crypto/dh.h:32-39` has, in order, three
  `const void *` fields followed by three `unsigned int` fields.  Candidate
  `src/include/crypto/dh.rs:17-25` preserves that order as three `*const
  c_void` fields and three `c_uint` fields under `#[repr(C)]`.  On the frozen
  AArch64 target those are pointer-width/32-bit unsigned C representations;
  the resulting C field order, alignment, and trailing padding are preserved.
  `Copy, Clone` preserves C struct-by-value copying and introduces no field,
  allocation, ownership, or FFI-layout change.
- `crypto_dh_key_len` — Linux declaration at `dh.h:51` is preserved at Rust
  line 29 with external symbol name, `extern "C"` calling convention,
  `const struct dh *` input (`*const dh`), and `unsigned int` result
  (`c_uint`).
- `crypto_dh_encode_key` — Linux declaration at `dh.h:67` is preserved at Rust
  line 32 with `char *` (`*mut c_char`), `unsigned int` (`c_uint`), const
  parameter pointer, and `int` (`c_int`) result.
- `crypto_dh_decode_key` — Linux declaration at `dh.h:82` is preserved at Rust
  line 35 with `const char *` (`*const c_char`), 32-bit unsigned length, and
  mutable output `struct dh *` (`*mut dh`), including its alias-into-input
  buffer contract.
- `__crypto_dh_decode_key` — Linux declaration at `dh.h:97-98` is preserved at
  Rust lines 38-39 with the double-underscore external symbol name and the
  same C ABI and parameter/result types.  The candidate does not add a
  wrapper that would alter the Linux helper's no-check semantics.
- Header guard `_CRYPTO_DH_` — the selected Linux conditional/macro records
  (`SYMBOLS.tsv` rows for source lines 8, 9, and 98) are C-preprocessor
  inclusion machinery and have no Rust runtime or ABI counterpart.  The Rust
  module expresses the complete guarded declaration content exactly once;
  no selected declaration is omitted.
- Provenance and copyright — candidate lines 1-9 retain
  `GPL-2.0-or-later`, the upstream Copyright `(c) 2016, Intel Corporation`,
  and the named author.  The provenance source, revision, architecture, and
  task ID agree with the frozen task and pinned revision.
- Branding — the frozen allowlist has no permitted replacement rows; the
  candidate contains no `Lupos` branding or Linux-name substitution.

## Semantic-record closure evidence

The frozen `SYMBOLS.tsv` records for the guard and `struct dh`, the `ABI.tsv`
record for `struct dh`, and the `LIFETIMES.tsv` record for `struct dh` were
initialized `PENDING_REVIEW`.  Source review resolves their substance as
follows: `struct dh` is a caller-owned, by-value-copyable C layout carrying
three borrowed immutable byte-buffer addresses and their three explicit byte
lengths; it provides no allocation, lock, RCU, refcount, callback, or
destruction behavior.  The decode declarations permit the output record to
receive addresses into the caller-supplied input buffer, exactly as the Linux
header documents, so caller lifetime must encompass use of those addresses.
The C declarations have ordinary external linkage and the frozen AArch64 C
calling convention.  No unresolved source-level semantic question remains
for this task.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or runtime
tool was invoked or used as review evidence.
