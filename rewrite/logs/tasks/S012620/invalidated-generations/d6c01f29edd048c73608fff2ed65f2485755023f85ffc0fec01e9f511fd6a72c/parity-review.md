# Parity review — S012620

Reviewed `src/include/crypto/dh.rs` against the complete pinned
`vendor/linux/include/crypto/dh.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus the operative consumers and
definitions in `crypto/dh_helper.c` and `crypto/dh.c`.  Scope is AArch64 only.
No build, test, formatter, or runtime command was run.

## Finding P1 — upstream SPDX expression changed

`include/crypto/dh.h:1` is `SPDX-License-Identifier: GPL-2.0-or-later`, while
`src/include/crypto/dh.rs:1` states `GPL-2.0-only`.  This is neither an
allowlisted branding change nor an immutable-provenance match.  Restore the
exact upstream SPDX expression (`GPL-2.0-or-later`).

## Verified mappings

- `struct dh` is present with `#[repr(C)]`, and retains the three `const void *`
  fields followed in order by the three `unsigned int` length fields.  The raw
  non-owning pointer representation and `c_uint` fields preserve the selected
  AArch64 layout and field mutability.
- The four declared functions retain their C symbol names, C ABI, return
  widths, and input/output pointer mutability: `crypto_dh_key_len`,
  `crypto_dh_encode_key`, `crypto_dh_decode_key`, and
  `__crypto_dh_decode_key`.
- The mutable encode buffer and immutable decode buffer match the header
  signatures.  The decoder documentation correctly preserves the operative
  `dh_helper.c` behavior: successful decode stores `key`, `p`, and `g` as
  pointers into the caller-provided packet buffer, performs no allocation, and
  transfers no ownership.
- Error-producing decode and encode behavior remains owned by the corresponding
  helper implementation task; the declarations introduce no substitute
  behavior or altered error return.  The candidate contains no configuration
  branch, linkage rename, test, stub, or additional state.

Result: one correction required before final application; no other parity
finding in this header translation.
