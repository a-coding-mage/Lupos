# S012622 Rust semantics review

Reviewed `src/include/crypto/ecdh.rs` against pinned
`vendor/linux/include/crypto/ecdh.h` and the defining
`vendor/linux/crypto/ecdh_helper.c` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen AArch64 scope.

## Result

PASS — no Rust-semantics findings.

## Checked

- `#[repr(C)] struct ecdh` preserves the C field order, mutable `char *`
  pointee mutability, `unsigned short` width, and the target's ordinary C
  trailing padding/alignment.  It contains raw pointers only and creates no
  Rust references or ownership claim over caller storage.
- The curve constants retain the C `int` values.  The three declarations use
  the C ABI with `char *`/`const char *`, `unsigned int`, C `int`, and the
  matching raw `ecdh` pointer mutability.
- All C-pointer dereferences remain behind `unsafe extern "C"` declarations;
  there are no Rust `unsafe` blocks, conversions, allocation, `Drop`, panic,
  or aliasing abstraction that can change C behavior.
- The documentation accurately retains the important decode behavior from
  `crypto_ecdh_decode_key`: success stores a non-owning pointer into the
  caller-provided packet buffer, whose lifetime must outlive use through
  `ecdh.key`.

No source, manifest, or queue fields were edited by this review.  No build,
format, test, runtime, or compiler command was run.
