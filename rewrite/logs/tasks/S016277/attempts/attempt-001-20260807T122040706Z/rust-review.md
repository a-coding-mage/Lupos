# Rust source review — S016277 / attempt 1 / slot 2

**Verdict: REJECT.** This review was manual source inspection only. No compiler,
formatter, rust-analyzer diagnostic, test, or runtime command was invoked.

## Review basis

- Queue state observed before review: `REVIEWING`, pipeline `P02`, attempt `1`,
  lease owner `codex-root-repair-20260807-p02`.
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.
- Phase-0 identity binding: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`.
- The reviewed candidate is
  `src/include/uapi/linux/netfilter/nf_tables.rs`, SHA-256
  `1643e518a48fb8e41a33a1f5175ec3dde9bea9c59d869cf558d6adf59add76f1`.
- The sealed proposal is `semantic-proposal-v1`, 5,737 records, proposal
  SHA-256 `145c650b638a289b6813a9ed966a74562dbe3e9efb6bb69e271a94c15d22862a`;
  its records bind candidate SHA-256
  `34d2dcf1b3d327017305f30967dabd32803d902d32f90c3c59007f041be2e741`.

The proposal therefore does not describe the reviewed candidate. Its claimed
`COMPLETE` closure state cannot be attested for this source revision.

## Findings

### RUST-S016277-01 — stale sealed proposal invalidates record-level closure

Severity: blocking.

The candidate hash currently is `1643e518…add76f1`, whereas every proposal row
is sealed against `34d2dcf1…be2e741`. The seal independently repeats the same
proposal hash, task, attempt, pipeline, queue fingerprint, and Phase-0 identity.
Consequently no slot-2 semantic attestation can truthfully supply exact current
candidate hashes for this proposal.

Evidence: current candidate file; proposal rows including
`SC1-7891aee3610f34ec984fc67dc1794d5a5d477b7c3dab2e557a6fa740fa5346e2`;
proposal seal `semantic-closure-proposal.sha256`.

Closure keys: the candidate binding on every one of the 5,737 proposal records,
including `SC1-7891aee3610f34ec984fc67dc1794d5a5d477b7c3dab2e557a6fa740fa5346e2`.

Required resolution: regenerate and reseal the proposal from the exact current
candidate, then repeat independent reviews; do not carry the old closure state
forward.

### RUST-S016277-02 — `NFT_REG32_MAX` loses its `__KERNEL__` condition

Severity: blocking.

Pinned source lines 49–51 define `NFT_REG32_MAX` only under `#ifdef __KERNEL__`.
The Rust candidate line 976 exports the value unconditionally and contains no
conditional mapping. This changes the UAPI/header visibility mechanism for
non-kernel consumers and is not an exact translation of the conditional branch.

Evidence: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:49-51`;
`src/include/uapi/linux/netfilter/nf_tables.rs:976`.

Closure keys:
`SC1-08e33fd3d749293fdf31ae3b01814315e063f1f08d542c4e905be14b22302a77`,
`SC1-56e19584fe02938c2f1058ca04fee0f14dbb50fb05106692edd7d0de48c7f28d`,
`SC1-91c69283317f93b278bca6df507aa8894ab78c89ae7a02ae146d3bae0c663824`,
and `SC1-69eeb13964c5549ca8fe8e3311a4dde4456a8a0b8f76139a097864834ee812de`.

Required resolution: retain the source conditional in the Rust configuration
surface, or establish with frozen source/config evidence that this Rust module
is exclusively the `__KERNEL__` view and record that conclusion. A universal
public constant is not equivalent.

### RUST-S016277-03 — `enum nft_data_types` has incompatible signedness/type surface

Severity: blocking.

The pinned enum explicitly assigns `NFT_DATA_VERDICT = 0xffffff00U` and documents
the `0xffffff00`–`0xffffffff` range as reserved (lines 500–509). The candidate
declares the enum tag alias as `i32` (line 229) but exposes its enumerator as
`u32` (line 231). The value cannot be represented as a positive `i32`, so the
constant is unusable as the declared alias without an explicit bit-changing cast;
the candidate supplies neither the correct common type nor a reviewed FFI/layout
contract. This is a C enum representation and promotion/FFI mismatch, not a
style issue.

The frozen ABI and lifetime records for `enum nft_data_types` remain
`PENDING_REVIEW`; declaring the related proposal rows `COMPLETE` does not close
that unresolved ABI fact. The correct ABI-compatible representation for both
approved targets must be established from the pinned toolchain/config evidence
before choosing a Rust type; this review makes no unproven choice.

Evidence: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:500-509`;
`src/include/uapi/linux/netfilter/nf_tables.rs:229-231`; ABI rows for S016277,
source line 504, both `aarch64` and `x86_64`.

Closure keys:
`SC1-ac290ff379f4ce0bdd3d5755e308b3f46628c922a0c8b3214cc2e98e9aadcae4`,
`SC1-decfd3220721b7981e07c9328008e0856ce5c0205dd76574334de152d56a7ed1`,
`SC1-ebba0143eafacd67bf391da75bd5e8bbb7a8a3a0d16af2eee8dcf3df4f42110f`,
`SC1-e7a7a638e4069260a7d3ee16f58e61893e57b236ba68e165620b1e6576fd4f10`,
`SC1-3c38aa5aefeadc0fa1a30f6068e968fa27063899f7d1cde9f0ecd790b4987031`,
and `SC1-876c40803018f8d33e2d4ec5d74b3c8e5e0d2dfd5d6d9719568317d9422b989c`.
The associated ABI status keys, also marked `COMPLETE` despite their frozen
base rows being `PENDING_REVIEW`, are
`SC1-0be697b62c8a40dff36476b08226a596e4e6c54d73c81054e8040573be17db4d`
and `SC1-04051c2c6b92ad260d7490dda6f2fe7067acda62bff09ed0ddeacf6cd1259ce4`.

Required resolution: determine the pinned C enum ABI for x86_64 and AArch64,
represent the tag and both enumerators consistently, and document the resulting
layout/FFI decision before closure.

### RUST-S016277-04 — source SPDX identifier was changed

Severity: blocking.

The UAPI header bears `GPL-2.0 WITH Linux-syscall-note` at source line 1. The
candidate substitutes `GPL-2.0-only` at line 1. This is an unauthorized license
identifier change and violates the requirement to retain upstream SPDX
identifiers.

Evidence: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h:1`;
`src/include/uapi/linux/netfilter/nf_tables.rs:1`.

Closure key: `SC1-7891aee3610f34ec984fc67dc1794d5a5d477b7c3dab2e557a6fa740fa5346e2`.

Required resolution: retain the source SPDX expression exactly unless an
allowlisted legal/provenance rule supplies contrary authority.

## Additional manual checks

- The source contains 115 `enum` tags and the candidate contains 115 matching
  `pub type` aliases; the lexical constant inventory has no source-only public
  identifier. This does not cure the conditional, representation, provenance,
  or stale-closure defects above.
- No `unsafe`, allocation, callback, pinning, interior mutability, `Drop`,
  pointer arithmetic, `#[repr]`, FFI function, panic, `todo!`, `unimplemented!`,
  or Rust test construct is present in the candidate. Therefore no additional
  Rust ownership/unsafe finding arises from this constants-only file.
- No compilation or compiler-derived evidence was used. The remaining enum ABI
  uncertainty is deliberately recorded as a source-review finding.

## Slot-2 closure attestation

The mandated `FINDINGS` slot-2 attestation was submitted with the exact current
frozen manifest hashes and exited successfully after a 30-second allowed wait
window. The tool returned JSON with `task_id=S016277`, `slot=2`, `findings=4`,
and proposal SHA-256
`145c650b638a289b6813a9ed966a74562dbe3e9efb6bb69e271a94c15d22862a`.
The attestation exists at
`rewrite/logs/tasks/S016277/semantic-closure-rust-review.tsv` (SHA-256
`a46dc3e421c0381d08a1d47ad68543344dc2d0443ad26a5bc9d2442324b85b13`).

This procedural attestation records the four findings; it does not remedy the
stale candidate binding identified in RUST-S016277-01.

## Queue completion result

After the required 30-second wait/poll, `rewrite_queue.py mark-review --id
S016277 --slot 2 --pipeline P02` exited with code 2 and the exact error:

`error: semantic review attestation binding mismatch for S016277 slot 2`

No successful queue mutation occurred. This review must remain unclosed until
the proposal/attestation are regenerated with bindings that match the current
candidate and attempt state.
