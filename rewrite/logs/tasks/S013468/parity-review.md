# Parity review — S013468

Reviewed `src/include/linux/asn1.rs` against the complete pinned
`vendor/linux/include/linux/asn1.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result

PASS — no actionable parity findings.

## Source and selected-consumer audit

- The upstream header is unconditional apart from `_LINUX_ASN1_H`; it has no
  Kconfig-controlled branch.  The candidate contains the corresponding
  complete common-architecture content and immutable source/revision/task
  provenance.
- SPDX (`GPL-2.0-or-later`) and the Red Hat/David Howells copyright and
  authorship notice are retained.  No branding change, Rust test configuration,
  stub, mutable completion claim, or extra operative behavior was found.
- `enum asn1_class` is represented by `asn1_class = c_int` and retains all four
  C enumerators: `ASN1_UNIV=0`, `ASN1_APPL=1`, `ASN1_CONT=2`, and
  `ASN1_PRIV=3`.
- `enum asn1_method` is represented by `asn1_method = c_int` and retains both
  C enumerators: `ASN1_PRIM=0` and `ASN1_CONS=1`.
- `enum asn1_tag` is represented by `asn1_tag = c_int` and retains every
  upstream enumerator with the same value: 0 through 13
  (`EOC` through `RELOID`) and 16 through 31 (`SEQ` through `LONG_TAG`).  The
  reserved 14 and 15 gap is retained; no replacement enumerator was added.
- The three integer macros retain their C `int` values:
  `ASN1_CLASS_BITS=0xc0`, `ASN1_CONS_BIT=0x20`, and
  `ASN1_INDEFINITE_LENGTH=0x80`.
- Pinned consumers use these names only as integer tag-byte expressions:
  comparisons, masks, shifts, and ORs in `lib/asn1_decoder.c`,
  `lib/oid_registry.c`, `crypto/asymmetric_keys/pkcs7_parser.c`,
  `pkcs7_verify.c`, `verify_pefile.c`, and `x509_cert_parser.c`.  A complete
  pinned-tree search found no object declaration, function parameter, or other
  use of `enum asn1_class`, `enum asn1_method`, or `enum asn1_tag` outside this
  declaration header.  Thus the aliases preserve the selected contract: C
  enumerator expressions are `int`-valued and the header exports no selected
  enum-object ABI.
- The direct header users `asn1_decoder.h`, `asn1_encoder.h`, and
  `asn1_ber_bytecode.h` likewise consume only the constants; in particular,
  `_tag` and `_tagn` use their C integer-expression semantics, which the
  candidate exposes through `c_int` constants.

No source was edited and no build, formatter, compiler, linker, test, or
runtime command was run.
