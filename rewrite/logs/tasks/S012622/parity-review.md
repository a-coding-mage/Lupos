# Parity review — S012622 / include/crypto/ecdh.h

Status: APPROVE

Reviewed the complete pinned `vendor/linux/include/crypto/ecdh.h` against
`src/include/crypto/ecdh.rs`, its candidate diff, the frozen S012622 scope,
symbol, ABI, lifetime, queue, and provenance records, and the narrow local
callee/caller context in `crypto/ecdh_helper.c`, `crypto/ecdh.c`, and
`net/bluetooth/ecdh_helper.c`.

No parity findings.

- Linux macros `ECC_CURVE_NIST_P192`, `ECC_CURVE_NIST_P256`,
  `ECC_CURVE_NIST_P384`, and `ECC_CURVE_NIST_P521`
  (`include/crypto/ecdh.h:26-29`) retain the four exact values in the Rust
  constants.
- Linux type `struct ecdh` (`include/crypto/ecdh.h:37-40`) is represented by
  `#[repr(C)] Ecdh` with a mutable `char *`-equivalent pointer first and an
  `unsigned short`-equivalent `u16` second.  On the selected AArch64 ABI this
  preserves the pointer-aligned layout and tail padding recorded for the
  frozen ABI item.
- Linux declarations `crypto_ecdh_key_len`, `crypto_ecdh_encode_key`, and
  `crypto_ecdh_decode_key` (`include/crypto/ecdh.h:52,67,81`) retain their
  unmangled C names, argument mutability, pointer constness, and `unsigned
  int`/`int` result and parameter widths through the `extern "C"`
  declarations.  The decode declaration still permits the documented alias of
  `params.key` into the input buffer; this matches the implementation at
  `crypto/ecdh_helper.c:56-80`.
- The Rust module makes the C include guard `_CRYPTO_ECDH_`
  (`include/crypto/ecdh.h:8-9,83`) unnecessary without changing the selected
  header declarations.  No selected branch, API, linkage, layout, error
  contract, or allowlisted branding delta is omitted.
- SPDX and all immutable provenance fields match the pinned header, SHA,
  architecture, and task ID.  The candidate adds no test, placeholder,
  wrapper, allocation, synchronization mechanism, or unauthorized branding.

Semantic closure: no SC1 finding keys; all 23 sealed S012622 proposal records
are approved.
