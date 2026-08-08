# Rust source review — S012620, attempt 4

Reviewed only the pinned `include/crypto/dh.h`, its pinned helper implementation
`crypto/dh_helper.c`, the current candidate `src/include/crypto/dh.rs`, the
current candidate snapshot, and the S012620 rows/proposal in the frozen
manifests.

## Result

ACCEPT — no Rust-source finding requiring a candidate change.

## Rust/FFI audit

- `dh` is `#[repr(C)]` and retains the C declaration order: three `const void
  *` fields, then three `unsigned int` fields.  On the selected AArch64 ABI,
  the raw pointer representation and `u32` widths preserve the required
  natural C layout and eight-byte aggregate alignment; there is no packing,
  bitfield, endian conversion, or Rust reference substitution.
- `Copy, Clone` is behaviorally compatible with the plain C aggregate: it owns
  no pointee and has no release action.  The candidate introduces no `Drop`,
  allocation, pinning claim, interior mutability, or `Send`/`Sync` assertion.
  Its raw pointers retain the caller-controlled aliasing and cross-CPU
  synchronization model instead of creating exclusive Rust borrows.
- The four declarations retain their C names, C ABI, pointer mutability,
  `unsigned int`/`int` widths, and unsafe call boundary.  Rust does not
  dereference, offset, reborrow, or cast any caller pointer, so it creates no
  additional provenance, bounds, panic, or unwind path.
- The declared buffer contracts match `crypto/dh_helper.c`: encode reads the
  three advertised input regions and writes the packet buffer; decode writes
  the destination aggregate and stores `key`, `p`, and `g` as non-owning
  offsets within the supplied packet buffer.  The candidate accurately records
  that the decoded buffer must outlive use of those fields.  Neither the C
  aggregate nor this declaration supplies locking, refcounting, RCU, callback,
  or interrupt lifetime management.
- No `unsafe` block or `unsafe fn` body is present.  The unsafe extern calls
  have focused caller-obligation documentation; no safe wrapper or RAII timing
  is substituted for the Linux contract.

## Frozen semantic-closure proposal

The attempt-4 proposal is source-supported and may be applied unchanged:

- `SCOPE.tsv:12621` semantic status;
- `SYMBOLS.tsv:143217` through `143220`: header guard conditionals, guard
  macro selection/status, and `struct dh` selection/status;
- `ABI.tsv:94741`: C field layout, AArch64 alignment, non-export status, and
  completion status;
- `LIFETIMES.tsv:90682`: non-owning ownership, decoded-buffer lifetime,
  caller-provided synchronization/aliasing, and completion status.

The proposed lifetime wording is specifically corroborated by
`crypto/dh_helper.c`: `__crypto_dh_decode_key()` assigns all three fields from
offsets within `buf` without allocation, and the public decoder performs only
post-decode validation.  No unresolved Rust ownership, layout, ABI, or
unsafe-boundary question remains for this task.
