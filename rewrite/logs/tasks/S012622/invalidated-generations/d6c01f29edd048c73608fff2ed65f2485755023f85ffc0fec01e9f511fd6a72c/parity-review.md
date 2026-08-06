# Parity review — S012622 (slot 1)

Reviewed the complete pinned `vendor/linux/include/crypto/ecdh.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against only
`src/include/crypto/ecdh.rs`, for the frozen `aarch64` task scope.  I also
checked the immediate defining/consumer evidence in
`vendor/linux/crypto/ecdh_helper.c`, `crypto/ecdh.c`, and
`net/bluetooth/ecdh_helper.c` to validate the declared ABI and aliasing
contract.

## Result

PASS — no actionable parity findings.

## Checked mapping

- The candidate retains the exact upstream `GPL-2.0-or-later` SPDX identifier,
  Intel copyright notice, immutable source path, pinned revision, `aarch64`
  architecture membership, and `S012622` task ID.  It introduces no branding
  change.
- All four object-like curve-ID macros are present as public C-`int` constants
  with their exact values: `ECC_CURVE_NIST_P192 = 0x0001`,
  `ECC_CURVE_NIST_P256 = 0x0002`, `ECC_CURVE_NIST_P384 = 0x0003`, and
  `ECC_CURVE_NIST_P521 = 0x0004`.  The immediate `crypto/ecc.c` consumers use
  these exact values for curve selection; no value, identifier, or conditional
  arm is omitted.
- `#[repr(C)] pub struct ecdh` retains source field order and C ABI: mutable
  `char *key` followed by `unsigned short key_size`.  `*mut c_char` is a raw,
  non-owning pointer and `c_ushort` is the source-width unsigned field; the
  C-layout representation preserves the pointer-plus-`u16` layout, including
  ABI padding.  The candidate does not introduce ownership or a by-value copy.
- The three declarations preserve identifier, argument order, pointer
  constness/mutability, and C-width return/length types:
  `crypto_ecdh_key_len(const struct ecdh *) -> unsigned int`,
  `crypto_ecdh_encode_key(char *, unsigned int, const struct ecdh *) -> int`,
  and `crypto_ecdh_decode_key(const char *, unsigned int, struct ecdh *) -> int`.
  `unsafe extern "C"` correctly retains their C calling boundary without
  manufacturing a Rust implementation in this header task.
- The aliasing/lifetime statement is faithful to the defining helper:
  `crypto_ecdh_decode_key` assigns `params->key` to packet-buffer storage and
  neither allocates nor copies private-key bytes.  The candidate's raw-pointer
  contract preserves that `p.key` aliases `buf`; no Rust reference or ownership
  claim strengthens the Linux lifetime.
- The only source conditions are the C multiple-inclusion guard.  It has no
  exported ABI or selected runtime branch and is correctly not reproduced in a
  single Rust module.  No configuration-dependent declaration exists in the
  header.

No source, manifest, queue, build, formatter, compiler, test, or runtime file
was changed or run by this reviewer; this report is the sole review artifact.
