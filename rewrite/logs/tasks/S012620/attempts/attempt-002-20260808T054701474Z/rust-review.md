# Rust semantics review — S012620 / P01 / attempt 2 / slot 2

## Review basis

- Pinned Linux source: `vendor/linux/include/crypto/dh.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Selected architecture/configuration: `aarch64`,
  `rewrite/configs/aarch64/frozen.config` (`CONFIG_64BIT=y`,
  `CONFIG_MODULES=y`, `CONFIG_CRYPTO_DH=m`).
- Frozen Phase 0 identity SHA-256:
  `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`.
- Frozen translation-queue fingerprint SHA-256:
  `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.
- Current candidate SHA-256:
  `eca68d69b1f1c13d6cac030e0141367eab1531a1fb7625a257a98221b1641873`.
- `implementation.md` and `candidate.diff` both seal that same current hash.
- No compiler, formatter, linker, test, runtime, rust-analyzer diagnostic, or
  historical Rust source was used. This is a manual source review.

## Findings

### RUST-S012620-001 — BLOCKER: all local SC1 closure records are bound to a different candidate

`semantic-closure-proposal.tsv` contains 15 SC1 records whose literal
`candidate_sha256` is
`19e046d7b079eabc3a940c6ef6d4c1a2e44045a8b42ddf5788ac7184db69845f`,
not the current candidate hash `eca68d69b1f1c13d6cac030e0141367eab1531a1fb7625a257a98221b1641873`.
The proposal's own SHA-256 is
`a453761ea8b99bf2cd0c7403e2da4cf11497a978211845d5911bc3d258bdd5de`.

Those semantic-closure records therefore cannot evidence the current source
candidate. The applier must regenerate/reseal them against the current
candidate, then independently verify that their proposed values remain
supported. Do not close this task from the stale SC1 evidence.

Literal current SC1 records and proposed final values:

| SC1 record | Manifest field | Literal proposed final value |
| --- | --- | --- |
| `SC1-149215b282f2bc1fa8b5552f471ffcff1f8521e8803f4bcdb5d94033ffb61329` | `SCOPE.tsv:semantic_status` | `COMPLETE` |
| `SC1-9eddc55ab85a6ea371b8319558db7962b0c86ddaa15103c0e749ff4ac3bd572c` | `SYMBOLS.tsv:ifndef@8:status` | `NOT_APPLICABLE` |
| `SC1-4b63287e5d1a43152a84e2314bd3efd74aec5beddb870b3214e92dc5135ad612` | `SYMBOLS.tsv:endif@98:status` | `NOT_APPLICABLE` |
| `SC1-92bb08f249814ef488849c2689ec51c339256ffc9182850831bf47e2a4e8571f` | `SYMBOLS.tsv:_CRYPTO_DH_:selection_expression` | `NOT_APPLICABLE` |
| `SC1-344b9b869c46c7048c712d23121435b173e74d253f9e26ef52257a88b4322d35` | `SYMBOLS.tsv:_CRYPTO_DH_:status` | `NOT_APPLICABLE` |
| `SC1-2f2d4e365af5fe65dff1ccb6771e2dbe9bf87a43b6123c280f22f5b369ec7b0c` | `SYMBOLS.tsv:struct dh:selection_expression` | `aarch64 header closure selected via crypto/dh.o` |
| `SC1-dadf8a3d5a37ab1693c48a785950464a30eea7995b99ef3f8d26af8bea30bcab` | `SYMBOLS.tsv:struct dh:status` | `COMPLETE` |
| `SC1-cfe2f177cf52777ad7254118c246fb60506631e7b464da773998fc55f92cfe2d` | `ABI.tsv:struct dh:alignment` | `8 bytes on aarch64` |
| `SC1-b6c0837226615fa21d91c8b2621b1b089e6f1e38e752867ffe87a7ef84029179` | `ABI.tsv:struct dh:export_kind` | `C ABI type declaration; no standalone symbol` |
| `SC1-014aa097f792fd278553c1b8c81ab8726f10c20c75592e4e4079f50f7c2ef941` | `ABI.tsv:struct dh:layout` | `key@0,p@8,g@16,key_size@24,p_size@28,g_size@32; sizeof=40 bytes` |
| `SC1-1fbc8a6492da805a96fec4c7ae2cdf2a8c4ccfad20e5c94ad2738e31526dd14a` | `ABI.tsv:struct dh:status` | `COMPLETE` |
| `SC1-986127ce80f8aebb873c68ac3da18d3fa133bbc047473e44259a9208ebf626b2` | `LIFETIMES.tsv:struct dh:lifetime_contract` | `caller retains pointed-to storage; decode makes p alias buf` |
| `SC1-fc1a7bd1d3a8c1c166903feed386102be9e0500d1ae34d5eda0eab40f303e122` | `LIFETIMES.tsv:struct dh:locking_rcu_refcount` | `no intrinsic locking, RCU, or refcounting` |
| `SC1-e50364836c9e6d0ef20e3178eaec1d9df48f60c898608d8cbbdc45f7f2d52d12` | `LIFETIMES.tsv:struct dh:ownership` | `non-owning pointers; caller owns key,p,g storage` |
| `SC1-19ac9e76b491d34c162f8d5f9e87228f5e2c17c80c1ef7bb0a94734c84bb398d` | `LIFETIMES.tsv:struct dh:status` | `COMPLETE` |

Manual source inspection supports the substantive layout and ownership values
above, subject to the required reseal: `dh_helper.c` assigns all three
descriptor pointers into the caller-provided packet buffer in
`__crypto_dh_decode_key`, and does not allocate or transfer ownership. The
proposal wording for the lifetime contract understates this: `key`, `p`, and
`g` (not only `p`) alias `buf` after decode. The regenerated record must state
that complete aliasing fact.

## Manual Rust/FFI assessment of the current candidate

`#[repr(C)] pub struct dh` preserves the six C fields in source order. On the
selected AArch64 ABI, three 8-byte pointer fields followed by three 4-byte
`unsigned int` fields produce offsets `0, 8, 16, 24, 28, 32`, 8-byte alignment,
and 40-byte size, matching the C declaration. There is no C packing, bitfield,
union, endian conversion, or by-value function parameter to reproduce.

`*const c_void` accurately retains C `const void *` pointee constness without
inventing a Rust reference lifetime or exclusivity guarantee. All descriptor
pointers remain raw, non-owning pointers; the candidate introduces no `Drop`,
allocation, bounds check, borrow, pinning promise, interior mutability, atomic
operation, callback, RCU, or refcount behavior. `Copy, Clone` is faithful to
the C record's ordinary value-copy semantics: it duplicates pointer values and
sizes only, never pointed-to storage. It does not create an owning clone.

The type has no explicit `Send` or `Sync` implementation. That avoids making a
Rust-wide cross-CPU safety assertion for arbitrary non-owning storage; the C
header supplies no such synchronization contract. No explicit `unsafe` block
or unsafe trait implementation is present or justified in this declaration.

All four imports use `unsafe extern "C"`, exact unmangled C symbol spellings,
and the C ABI. `c_uint`, `c_int`, and `c_char` retain the selected C ABI scalar
categories; the recorded AArch64 compile command uses `-funsigned-char`, and
the declarations pass `char *`/`const char *` as raw pointers rather than
Rust slices. Raw pointer parameters preserve C's nullability, aliasing, and
caller-managed buffer/lifetime obligations. In particular, decoding can write
aliasing pointers into `params`; Rust creates no references, so does not
strengthen C provenance or borrow duration. FFI calls remain unsafe rather
than being disguised as checked, allocating, or panic-capable wrappers.

No current-source Rust semantic, layout, calling-convention, ownership,
aliasing, or unsafe-boundary defect was found beyond the stale SC1 evidence
and its incomplete three-pointer alias wording. The task must not advance to
`DONE` until RUST-S012620-001 is resolved and the corrected closure is resealed
to the current candidate.
