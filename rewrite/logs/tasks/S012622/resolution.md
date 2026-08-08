# Applier resolution — S012622 / include/crypto/ecdh.h

## Dispositions

Neither independent report raised a finding, so there are no corrective
dispositions.  The applier independently rechecked the current candidate and
records the following closures rather than treating either approval as a
substitute for source evidence.

| Review report | Finding | Disposition | Upstream and local evidence |
| --- | --- | --- | --- |
| `parity-review.md` | None | APPROVED; no source change required. | `vendor/linux/include/crypto/ecdh.h:26-29` defines the four curve identifiers as `0x0001` through `0x0004`, exactly retained by the four `u32` constants.  Lines 37-40 define `struct ecdh` as `char *key` followed by `unsigned short key_size`; the candidate's `#[repr(C)] Ecdh` retains that field order, pointer representation, and `u16` width.  Lines 52, 67, and 81 have the same three unmangled C declarations, pointer constness, and `unsigned int`/`int` widths as the candidate's `extern "C"` declarations. |
| `rust-review.md` | None | APPROVED; no source change required. | On the frozen AArch64 target, the sealed ABI records establish pointer alignment of eight bytes, `key` at offset zero, `key_size` after the pointer, and C-ABI tail padding.  `#[repr(C)]`, a raw `*mut c_char`, and `u16` express that layout without packing or a Rust-owned representation. |

## Independent semantic closure

- `crypto_ecdh_decode_key` in the pinned
  `vendor/linux/crypto/ecdh_helper.c:56-80` copies the packet header and key
  size, then assigns `params->key = (void *)ptr`; it performs no allocation.
  Thus the key storage is caller-managed, and after decode it aliases the
  input packet.  The candidate deliberately exposes only raw pointers and
  makes no Rust reference, slice, ownership, `Drop`, allocation, or
  exclusivity claim.  This preserves the C provenance and lifetime contract.
- The same helper at lines 27-83 implements and exports all three declared
  interfaces.  The candidate declares their exact C symbol spellings and does
  not replace the packet encoding, validation, error paths, or side effects
  with a Rust wrapper.
- `vendor/linux/crypto/ecdh.c:33-45` consumes `params.key` immediately after
  decode as an input pointer.  No locking, RCU, refcount, callback, pinning, or
  cross-CPU contract is imposed by this header or its helper implementation;
  the candidate adds none.
- The C include guard is only C preprocessing machinery.  It has no Rust ABI
  or runtime counterpart, while every selected declaration remains present.
- The frozen AArch64 configuration selects `CONFIG_CRYPTO_ECDH=m` and the
  header-closure record identifies this header as `RUST_TRANSLATE` with six
  consumers.  The source/revision provenance matches
  `vendor/linux.SHA` (`425f94c2954b1fe80ebdbf9b29854e89750355df`).
- All 23 sealed S012622 semantic-closure proposal records are `COMPLETE`:
  one scope record, fourteen symbol/conditional/macro records, four ABI
  records, and four lifetime records.  Both corresponding independent
  semantic-closure attestations are `APPROVE` with no finding key.

## Eligibility

S012622 is source-review eligible for `DONE`: the current candidate is
non-empty, has the required immutable provenance, contains no placeholder,
test configuration, panic-based substitute, or unreviewed mechanism; both
review reports and their semantic-closure attestations exist; and this
resolution closes the reported-review disposition and frozen semantic fields.
No source blocker was found.  This is a source-only conclusion: no compiler,
formatter, linker, runtime, test, benchmark, or historical Rust source was
used.
