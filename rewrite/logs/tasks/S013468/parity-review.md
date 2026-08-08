# Parity review — S013468 / P01 attempt 1

Reviewed `vendor/linux/include/linux/asn1.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against the leased candidate
snapshot only.  No compiler, formatter, test, analyzer, or historical Lupos
source was used.

## Result: FINDINGS

### P1 — Enumerators and macro tokens no longer have the C integer-expression interface

`enum asn1_class`, `enum asn1_method`, and `enum asn1_tag` at
`include/linux/asn1.h:12`, `:21`, and `:28` provide C enumerator constants,
which are integer constant expressions.  The candidate instead re-exports
closed Rust enum variants.  Consequently the candidate cannot directly
preserve source forms that combine those values with the header macros:

* `include/linux/asn1_ber_bytecode.h:75-76` defines `_tag(CLASS, CP, TAG)` as
  shifts and ORs of `ASN1_##CLASS`, `ASN1_##CP`, and `ASN1_##TAG`;
  `lib/asn1_encoder.c:42` uses `_tag(UNIV, PRIM, INT)` to write a byte.
* `crypto/asymmetric_keys/pkcs7_parser.c:440` shifts and ORs `ASN1_UNIV` and
  `ASN1_SEQ`; `pkcs7_verify.c:115` and `x509_cert_parser.c:650` OR
  `ASN1_CONS_BIT` with enum constants; `lib/asn1_decoder.c:101` shifts
  `ASN1_PRIM`.

No `BitOr`/`Shl` integer semantics or source-proven conversion contract exists
for the re-exported Rust enum values.  The candidate also fixes each
object-like macro as `i32`, whereas the C replacement list is an untyped
integer constant expression whose type is determined after the consumer's C
integer promotions and assignment context.  This changes the public header
mechanism, not merely its spelling.

Affected semantic-closure records:

* `SC1-24c6d208ca380833ed53190da96d2053102ecfa23016fc3478ac5c0b52f28d4e`,
  `SC1-1aa79425365fd684568f0cbcc7bd36a4dfdf765c181d0a7be76db3871fc8c1a2`,
  `SC1-221570ee7c8aa111768da7a23f4f514255a861601f61497c4e38095dd20ae85b`
  (aarch64 macro expression records);
* `SC1-8f8f022c0d7a4fdc4c838e56b9cbf87d3773b02c99ccc4b16e7f9fd4a603b272`,
  `SC1-888c3d721214fe31b680a40664980fc656382c7e20194ddc9c63b605120034fe`,
  `SC1-5e8b7bfb6150fedcb36a2009714c62f59eb99b0232f06b2187f1f19f224b5e1e`
  (x86_64 macro expression records);
* `SC1-7661699e0cc01ceaa3a9918a2eb8dc240c480fcce7f324d963a5c0aabfdeaef4`,
  `SC1-0f13aeca9405fccc398b0464ff272b9603b82c840bfeb7ce3325d6c33007f9a5`,
  `SC1-9cfc96fbb67850eb7c47af995858cf9e1ef95fc3b1030392de262bb63f208ab7`,
  `SC1-a430bd3a0f8b10f9f7810316c4083aee377ddb4674c75af14ab4b7f03648ea2e`,
  `SC1-b2160cbba08e0b6809246081fec4b04ce34b1e8150cc87ed3d8845f84a5fd7e6`,
  `SC1-b106e18d5914bb01c1ea5dd68f8816d5ac912a36b4c72deec3330207dce6639a`
  (the six enum expression records).

### P2 — The candidate asserts C-repr enums despite unresolved frozen enum ABI records

The candidate chooses `#[repr(C)]` for all three named enums, but the frozen
ABI manifest retains `PENDING_REVIEW` for layout and alignment on both selected
architectures.  Neither the candidate nor the frozen records establish a
source-derived C-enum representation/valid-value interoperability contract.
The header’s values flow as C integer expressions into byte-oriented paths
(the cited `_tag` and comparisons above), so closing those records as complete
without that contract is unsupported.  Exact parity therefore cannot be
accepted from source evidence presently available.

Affected semantic-closure records:

* aarch64: `SC1-cd56caa527473da0e4b134450abdf654c7ef4948fc0822e208e03ef147df7640`,
  `SC1-97517f2540ee2cbdaef77f42dd3be539da8133b24f6c19158f8497aa061185d2`,
  `SC1-1f3fee0d3d8326f3a5bc31a76172d943f547a93fb0947865ac1ca0b6bf04adae`,
  `SC1-367f06815f79c9e9b6954142c0eda38bcba9bb640d240225f2c7886e94f4955d`,
  `SC1-10c24f488fcf4d5c3f01ad5463733f4728e2df050c6a603b3d437b7468f0f623`,
  `SC1-dd16c8ab05d94dd3e7f2af60101dda9efd77ca277183e1b298e8de7ccf4d4790`;
* x86_64: `SC1-aa9ac03a23c1200d0b4c3ab4e0e85edd729b7c494db0a8a5608519e93adc8220`,
  `SC1-1f145c94eb026592125e6598d73dbdbd250672e03b655bb2504875038c3b82bd`,
  `SC1-b11cb8d994c84724489b45d27c6fe33b1d4f7f3c5d75924cdd298df471d5bac6`,
  `SC1-c9851a7f396ebbe36c4564ebc423cfeae1d636638097fedea55e834840c3c07a`,
  `SC1-35e99b436f1bedcd4164bfe5fbdeb619a73d0093f0b391ab7d6bcbc2bd878a20`,
  `SC1-f5a70b611cf8ef7e4ad6916b341a5269aa68c75db5c73352404837b19541831c`.

No additional functions, statics, callbacks, allocation paths, locking, RCU,
or exported link symbols are declared by this pinned header.
