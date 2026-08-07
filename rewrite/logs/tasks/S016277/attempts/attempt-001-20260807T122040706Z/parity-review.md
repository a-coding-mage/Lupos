# Parity review — S016277, attempt 1, slot 1

Review basis: the pinned `vendor/linux/include/uapi/linux/netfilter/nf_tables.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate, the
current frozen manifests/configurations, and the sealed task-local closure
proposal only.  This was a manual source review; no compiler, formatter,
test, rust-analyzer diagnostic, or historical Lupos source was used.

## Findings

1. **PARITY-001 — `enum nft_data_types` has no single Rust representation
   capable of carrying both of its Linux enumerators.**  Linux declares the
   `nft_data_types` enumeration at header lines 504–507, with
   `NFT_DATA_VALUE` and `NFT_DATA_VERDICT = 0xffffff00U` belonging to that
   same type.  The candidate instead declares `pub type nft_data_types = i32`
   at `nf_tables.rs:229`, makes `NFT_DATA_VALUE` an `i32` at line 230, and
   makes `NFT_DATA_VERDICT` a `u32` at line 231.  Consequently the latter
   cannot be used where the former type alias is required, unlike the C
   enumeration; it also leaves the selected enum's ABI/width decision
   internally contradictory.  The high unsigned literal makes the exact
   C enum-compatible representation an ABI question that must be resolved
   from the pinned toolchain evidence, not guessed as `i32`; until then the
   candidate is not parity-safe.  Closure keys: aarch64
   `SC1-6821af34feec0515e474561991a9b81d3f3b8d469c36e86bc93568c108abfa07`,
   `SC1-ac290ff379f4ce0bdd3d5755e308b3f46628c922a0c8b3214cc2e98e9aadcae4`;
   x86_64
   `SC1-75c0eebedd16ef5f00790c374ad803bc85062061eae465b396d988bea2047548`,
   `SC1-e7a7a638e4069260a7d3ee16f58e61893e57b236ba68e165620b1e6576fd4f10`.

2. **PARITY-002 — the sealed closure does not describe the candidate that was
   reviewed, so no selected Linux UAPI entity can be closure-attested.**  The
   proposal and its seal bind every one of 5,737 records to candidate SHA-256
   `34d2dcf1b3d327017305f30967dabd32803d902d32f90c3c59007f041be2e741`,
   whereas the current `src/include/uapi/linux/netfilter/nf_tables.rs` hashes
   to `1643e518a48fb8e41a33a1f5175ec3dde9bea9c59d869cf558d6adf59add76f1`.
   This affects, for example, the selected Linux symbol `NFT_NAME_MAXLEN`
   (header line 5; closure keys aarch64
   `SC1-66873d0a196b05dff7c9dab16657c689011c9cd59348c2ca05e2cfa5e6f39151`
   and x86_64
   `SC1-a862c18781aa48075ae5221d0f0709f7b9444bacd41de653ff27ee7f285f1b79`),
   as well as every other selected mapping.  The proposal itself hashes to
   `145c650b638a289b6813a9ed966a74562dbe3e9efb6bb69e271a94c15d22862a` and
   its `.sha256` seal records the same digest.  The semantic gate must reject
   attestation rather than associate this report with a different candidate.

## Checked without additional findings on the current source

Manual comparison found all header macro names (other than the C-only include
guard) represented, all 115 C enum tags represented, and all generated UAPI
constant names present.  The direct value/progression audit covered the
`NFT_REG*`, message/attribute, flag/mask, object, trace, and tunnel sequences;
the deliberate macro-in-enum spelling around `NFT_META_IIFTYPE`/`NFT_META_OIFTYPE`
still yields `NFT_META_IFTYPE = 8`, alias `NFT_META_IIFTYPE = 8`, and
`NFT_META_OIFTYPE = 9` as at Linux lines 968–1008.  `NFT_NAME_MAXLEN` remains
256, and the Rust translations of multiline mask/max expressions preserve
their operands.  `NFT_REG32_MAX` is present at Rust line 976 but is only
defined by Linux under `__KERNEL__` (header lines 49–51); the frozen closure
marks it selected for both architectures (aarch64 keys
`SC1-08e33fd3d749293fdf31ae3b01814315e063f1f08d542c4e905be14b22302a77`,
`SC1-56e19584fe02938c2f1058ca04fee0f14dbb50fb05106692edd7d0de48c7f28d`;
x86_64 keys
`SC1-91c69283317f93b278bca6df507aa8894ab78c89ae7a02ae146d3bae0c663824`,
`SC1-69eeb13964c5549ca8fe8e3311a4dde4456a8a0b8f76139a097864834ee812de`),
but the proposal supplies no executable preprocessor proof that `__KERNEL__`
is the selected Rust interface condition.  The stale candidate binding in
PARITY-002 prevents accepting that closure assertion.

No C structs, unions, functions, exported link symbols, locking, RCU,
refcount, allocation, cleanup, or runtime error paths exist in this UAPI
constant header; none can be supplied by a Rust substitute.  The candidate
uses the required source/revision/architecture/task provenance and contains
no unauthorized Lupos branding.

Result: **FINDINGS**.  Slot-1 semantic attestation is required to reject
because its sealed candidate hash is stale; no queue transition is authorized.
