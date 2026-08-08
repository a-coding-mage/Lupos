# Rust source review — S016428, attempt 1, slot 2

Result: **FINDINGS**. This was a manual source inspection only. No compiler,
formatter, linker, test, rust-analyzer diagnostic, historical Rust source, or
Git history/diff was used.

Reviewed current candidate:

- Linux source: `vendor/linux/include/uapi/linux/tty_flags.h`
  (`sha256=c55caade93a4a3a190111d1d4884240491a0b4bc54d1e7a829b4846ace7e8066`)
- Rust destination: `src/include/uapi/linux/tty_flags.rs`
  (`sha256=305595986b16312b18c160aa12d7e46027c42cca13fc09ca12792d5ab82930db`)
- Candidate evidence: `candidate.diff`
  (`sha256=283857b177a0d9059c0557f74082021e150bd9e2c90797e803d5218aee449c2c`)
- Implementation evidence: `implementation.md`
  (`sha256=737039526adf11bb7a9a5e0f3eac3cf51dd66ffc460786fa5142355397d1b9e0`)

Frozen current-proposal closure binding:

- proposal: `semantic-closure-proposal.tsv`, 261 records,
  `sha256=83a3dbd0aa2e49813384e0e3a8be34037186f69a02259deffd66abe86c0060f1`
- Phase 0 identity: `sha256=0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- frozen closure base manifests: `SCOPE.tsv=b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a`,
  `SYMBOLS.tsv=7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`,
  `ABI.tsv=ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`, and
  `LIFETIMES.tsv=0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`.

## RUST-001 — `__KERNEL__` macro visibility was erased

Reject the proposal's `COMPLETE` closure decisions for the two `#ifndef
__KERNEL__` regions on both architectures. The original has two distinct
preprocessor contracts:

- Linux lines 42–53 define the ten obsolete `ASYNCB_*` positions only when
  `__KERNEL__` is not defined.
- Linux lines 84–95 likewise define nine obsolete `ASYNC_*` masks only when
  `__KERNEL__` is not defined.

The candidate instead publishes all nineteen names unconditionally at Rust
lines 29–38 and 69–77. In particular, the C expansion of the unconditionally
spelled `ASYNC_SUSPENDED` at Linux line 57 deliberately remains unusable in a
kernel preprocessing context because its `ASYNCB_SUSPENDED` token was not
defined there; the candidate defines both items and makes that expression
available to every Rust consumer. This changes the header's compile-time
interface for the frozen kernel consumers. A typed Rust item also cannot stand
in for an absent C preprocessor token.

The applier must preserve this selection boundary using a source-proven,
consumer-bound Rust configuration mechanism, or split the UAPI/user and core
surfaces according to the frozen file map. If such a mechanism cannot be
established from the frozen source/configuration, this task must remain
blocked; retaining the unconditional exports is not an equivalent substitute.

Affected sealed proposal keys:

- aarch64: `SC1-f79a16d4be1ff66afc074b828b6c44a7b346321a26153d04877d0a0f4449773f`,
  `SC1-61794dff24e5a1645971747b0ee1bfd20065ac4df2e6ffb2378c3e54efbe1aa2`,
  `SC1-4f6d118ff3fbf6668f59441719d014516fb817670da427286d204953fe531766`,
  `SC1-6c73c30ba237754bc70c9533f8de8ed402cd1e6f7ab731be0ef948e65169e21a`.
- x86_64: `SC1-c913c691bfc17fbcf30acef4fd353c271388b62d0e05ca319b9201d838acb3b8`,
  `SC1-490f386c549aba81642e5fd8d25c948a6297a07d6e12b5d68dc93b1caa372f7b`,
  `SC1-ec47cd5fcbbd65e66c1bed63e5e6a9c4e6e7f20abeff8d7cda1c64aed00faafe`,
  `SC1-74052589ae9cd69c5fb62a1e17b207a6def4966f4870e1933a231174458aa72c`.

## Checked and not found

For the macros that are visible in the corresponding C context, the candidate
correctly models every unsuffixed bit-index literal as a 32-bit signed `int`
value on both frozen ABIs, and every `1U`-rooted expression as a 32-bit
unsigned value. All source shift counts are 0 through 31, so the Rust constant
expressions introduce no out-of-range shift or signed-overflow change. The
mask compositions, `ASYNC_FLAGS`, and the unsigned complement in
`ASYNC_INTERNAL_FLAGS` retain the source's `unsigned int` width and values.

This header declares no storage, FFI function, structure, union, bitfield,
packing, alignment, callback, lock, allocation, ownership, refcount, RCU, or
pinning contract. The candidate contains no `unsafe`, `Drop`, `repr`, FFI,
panic path, or test configuration to audit. No additional finding arose in
those categories.
