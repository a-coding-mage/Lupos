# Applier resolution — S012622

Reopened the complete pinned `vendor/linux/include/crypto/ecdh.h` and its
defining implementation `vendor/linux/crypto/ecdh_helper.c` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.  I also rechecked the direct
`crypto/ecdh.c` and `net/bluetooth/ecdh_helper.c` consumers for the frozen
AArch64 scope.

## Review dispositions

| Review | Disposition | Pinned evidence |
| --- | --- | --- |
| Parity review: no finding | Accepted. | `include/crypto/ecdh.h:26-81`; `crypto/ecdh_helper.c:27-84` |
| Rust review: no finding | Accepted. | `include/crypto/ecdh.h:37-40,52,67,81`; `crypto/ecdh_helper.c:56-82` |

## Final source and contract

- The four curve identifiers retain their exact untyped C integer values as
  `c_int` constants.  `crypto/ecdh.c:135,160,185` consumes those values as
  C `unsigned int` curve identifiers without a changed numeric value.
- `#[repr(C)] struct ecdh` preserves the exact C order and ABI fields:
  mutable `char *key` followed by `unsigned short key_size`.  On the frozen
  AArch64 ABI this remains a pointer plus 16-bit unsigned field with the
  ordinary C alignment and trailing padding.  It owns no key storage and
  creates no Rust reference or derived ownership claim.
- The declarations retain the C ABI, original symbol names, argument order,
  pointee mutability/constness, and widths: `unsigned int`, `int`, `char *`,
  `const char *`, and `struct ecdh *`/`const struct ecdh *`.  The implementing
  symbols are supplied by the separate `crypto/ecdh_helper.c` task; this
  header task correctly declares rather than substitutes their behavior.
- `crypto_ecdh_decode_key` copies only its packet metadata, then assigns
  `params->key` to the remaining bytes in the caller's `buf`
  (`crypto/ecdh_helper.c:72-80`).  The raw mutable pointer therefore retains
  C's non-owning alias contract.  I removed the candidate documentation's
  unsupported requirement that the backing storage remain unchanged; the
  final documentation requires only that its storage outlive use through
  `p.key`.
- The C include guard has no Rust-module equivalent.  There is no selected
  configuration branch, allocation, lock, RCU/refcount, callback, or cleanup
  contract in this header.  The upstream author notice was retained alongside
  the SPDX and copyright provenance.

No unresolved source, ABI, ownership, lifetime, locking, or semantic
dependency remains for this task.  No compiler, formatter, linker, test,
emulator, debugger, benchmark, or runtime command was run.
