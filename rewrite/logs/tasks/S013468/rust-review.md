# Rust semantic review — S013468 attempt 1, P01

Result: **FINDINGS**.  This was a manual source review only; no compiler,
formatter, test, analyzer, or historical Lupos source was used.

## RUST-1 — C enum ABI and value domain are not established

`include/linux/asn1.h:12-60` declares three C enum types.  The frozen ABI
records for their layout and alignment remain `PENDING_REVIEW` on both
x86_64 and AArch64.  `src/include/linux/asn1.rs:7-14`, `20-24`, and `30-64`
nevertheless commit to Rust `#[repr(C)]` enums.  The pinned source and frozen
records do not establish that this Rust representation, alignment, and valid
discriminant domain exactly match the C compiler's enum ABI for either target.
In particular, a Rust enum has a restricted valid-discriminant domain whereas
a C enum object can be supplied with an arbitrary representation through
integer storage/FFI.  No source-only basis was found to close this ABI question
or to prove that the public type boundary is never reached with a non-listed
value.

Affected semantic-closure keys (ABI layout):

- `SC1-97517f2540ee2cbdaef77f42dd3be539da8133b24f6c19158f8497aa061185d2`
- `SC1-367f06815f79c9e9b6954142c0eda38bcba9bb640d240225f2c7886e94f4955d`
- `SC1-dd16c8ab05d94dd3e7f2af60101dda9efd77ca277183e1b298e8de7ccf4d4790`
- `SC1-1f145c94eb026592125e6598d73dbdbd250672e03b655bb2504875038c3b82bd`
- `SC1-c9851a7f396ebbe36c4564ebc423cfeae1d636638097fedea55e834840c3c07a`
- `SC1-f5a70b611cf8ef7e4ad6916b341a5269aa68c75db5c73352404837b19541831c`

## RUST-2 — Re-exported Rust variants do not preserve C integer-expression semantics

In C, the named enumerators are integer constants and participate in ordinary
integer conversions.  Pinned selected consumers demonstrate that contract:
`crypto/asymmetric_keys/pkcs7_parser.c:440` uses
`ASN1_UNIV << 6`, and `x509_cert_parser.c:565` defines
`SEQ_TAG_KEYID` as `ASN1_CONT << 6`; `lib/asn1_decoder.c:74,86,101` compares
raw `unsigned char` tags to enumerators and combines them with bit masks.

The candidate re-exports enum variants (`asn1.rs:16`, `26`, `66-72`) rather
than representing the C enumerators as integer constants in their original
expression domain.  Those variants retain their Rust enum types, not C's
integer-constant behavior, so the translation does not preserve the required
mask, comparison, shift, or promotion semantics.  This is a source-level
semantic mismatch independent of compilation.  An exact replacement must be
derived together with the unresolved enum ABI, rather than assuming Rust enum
typing is interchangeable with C enumerators.

Affected semantic-closure keys (selected enum type mapping):

- `SC1-7661699e0cc01ceaa3a9918a2eb8dc240c480fcce7f324d963a5c0aabfdeaef4`
- `SC1-0f13aeca9405fccc398b0464ff272b9603b82c840bfeb7ce3325d6c33007f9a5`
- `SC1-9cfc96fbb67850eb7c47af995858cf9e1ef95fc3b1030392de262bb63f208ab7`
- `SC1-a430bd3a0f8b10f9f7810316c4083aee377ddb4674c75af14ab4b7f03648ea2e`
- `SC1-b2160cbba08e0b6809246081fec4b04ce34b1e8150cc87ed3d8845f84a5fd7e6`
- `SC1-b106e18d5914bb01c1ea5dd68f8816d5ac912a36b4c72deec3330207dce6639a`

There are no raw pointers, callbacks, allocation paths, synchronization
objects, or `unsafe` blocks in this candidate.  That absence does not resolve
the ABI and integer-context failures above.
