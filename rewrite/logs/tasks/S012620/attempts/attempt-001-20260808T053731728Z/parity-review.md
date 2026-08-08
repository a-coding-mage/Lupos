# S012620 parity review — slot 1

Verdict: **FINDINGS**

Reviewed only the pinned `vendor/linux/include/crypto/dh.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current S012620 candidate,
its sealed semantic proposal, and direct pinned `crypto/dh_helper.c` /
`crypto/dh.c` declaration-use context. No compiler, formatter, test, or
historical translation source was used.

## Finding PARITY-COPYRIGHT-001

Semantic key: `SC1-149215b282f2bc1fa8b5552f471ffcff1f8521e8803f4bcdb5d94033ffb61329`.

Linux source evidence: `include/crypto/dh.h:2-6` carries the upstream
copyright notice naming Intel, the 2016 date, and its authors. Candidate
evidence: `src/include/crypto/dh.rs:1-5` retains the SPDX identifier and adds
the required immutable provenance, but omits that upstream copyright notice.
The required source-tree rule requires relevant upstream copyright notices to
be retained. The proposed `SCOPE.tsv.semantic_status=COMPLETE` is therefore
not supportable while this required header material is absent.

Required resolution: retain the relevant upstream copyright notice in the
candidate alongside the existing immutable provenance.

## Audited parity that is otherwise supported

- **`struct dh`** (`dh.h:32-39`): `#[repr(C)] dh` preserves the three
  `const void *` fields followed in source order by the three `unsigned int`
  fields. For the frozen AArch64 target this yields the native pointer/u32
  field alignment and the C-shaped tail padding; no packing or by-value
  replacement was introduced.
- **`crypto_dh_key_len`**, **`crypto_dh_encode_key`**,
  **`crypto_dh_decode_key`**, and **`__crypto_dh_decode_key`**
  (`dh.h:51,66,80,95-96`): the candidate preserves spelling, C calling
  convention, pointer constness/mutability, `unsigned int` widths, and `int`
  return type. Direct implementation evidence in `crypto/dh_helper.c:34-120`
  confirms the first three have `EXPORT_SYMBOL_GPL` linkage and the double-
  underscore helper is externally declared but not exported; the header
  candidate correctly introduces declarations rather than changing that
  linkage.
- **`struct dh` borrowed-buffer contract** (`dh.h:70-77`): raw non-owning
  pointers and the candidate documentation preserve that decode may point the
  fields into the caller-owned packet buffer, which must outlive use of the
  structure. No allocation, refcount, lock, RCU, callback, or cleanup
  mechanism exists in this header or was substituted in the candidate.
- **`_CRYPTO_DH_`** (`dh.h:8-9,98`): the sealed proposal accurately identifies
  this as a C single-inclusion guard; the Rust candidate adds no behavior,
  branding, shell, stub, or replacement mechanism in its place.

The reviewed candidate contains no `todo!`, `unimplemented!`, Rust test
configuration, unauthorized Lupos branding, or unbounded allocation/polling
substitution. The sole finding above prevents approval.
