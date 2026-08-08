# Parity review — S012620 (slot 1)

Result: **REJECT — two source-level findings require applier disposition.**

Reviewed only the pinned `vendor/linux/include/crypto/dh.h`, the current
`src/include/crypto/dh.rs`, the S012620 candidate summary, and the S012620
rows in the frozen queue/scope/symbol/ABI/lifetime manifests.  No compiler,
formatter, test, historical source, archive, or implementer rationale was
used.

## Findings

1. **P1 — upstream SPDX identifier changed.**
   Linux file `include/crypto/dh.h` begins at line 1 with
   `SPDX-License-Identifier: GPL-2.0-or-later`; the candidate begins with
   `SPDX-License-Identifier: GPL-2.0-only`.  No S012620 branding allowance is
   present for a licence identifier change.  This violates the requirement to
   retain upstream SPDX identifiers.  Restore `GPL-2.0-or-later` exactly.

2. **P1 — the frozen S012620 interface inventory omits four declarations.**
   Linux symbols `crypto_dh_key_len` (line 51), `crypto_dh_encode_key` (line
   66), `crypto_dh_decode_key` (line 80), and `__crypto_dh_decode_key` (lines
   94–96) are external declarations in the leased header.  The only S012620
   `SYMBOLS.tsv` rows are the include-guard conditional/macro and `struct dh`
   (rows 143217–143220); the only S012620 `ABI.tsv` row is `struct dh` (row
   94741); and the only S012620 `LIFETIMES.tsv` row is likewise `struct dh`
   (row 90682).  Thus the candidate's four FFI declarations have no frozen
   selected-symbol, linkage, calling-convention, error-contract, or lifetime
   record.  The candidate does not cure that frozen-manifest omission.  The
   applier must obtain the protocol-required scope/manifest disposition; it
   must not silently treat these interface symbols as closed.

## Source checks with no discrepancy found

* Linux symbol `struct dh` (lines 32–39) has fields `const void *key`, `p`,
  `g`, then three `unsigned int` sizes.  The candidate preserves that order,
  const-qualified raw-pointer form (`*const c_void`), and AArch64 widths
  (`u32`) under `#[repr(C)]`.  By the AArch64 C layout rules this is pointer
  offsets 0/8/16, size fields 24/28/32, alignment 8, and tail-rounded size 40.
  `Copy, Clone` is consistent with C aggregate value-copy semantics and adds
  no allocation, cleanup, or ownership transfer.
* Linux symbols `crypto_dh_key_len`, `crypto_dh_encode_key`,
  `crypto_dh_decode_key`, and `__crypto_dh_decode_key` retain their exact
  external spellings, C calling convention, pointer mutability, `unsigned
  int`/`int` returns and parameters as `u32`/`i32`, and `char *`/`const char
  *` pointer form as `*mut c_char`/`*const c_char`.  `crypto/dh_helper.c`
  confirms that the first three are GPL-exported definitions and the double
  underscore helper is an externally linked but non-exported definition; the
  header declarations themselves introduce no alternate linkage.
* The candidate retains the Intel copyright and author attribution.  Its
  safety documentation accurately records the Linux `crypto_dh_decode_key`
  and `__crypto_dh_decode_key` aliasing rule: the resulting `key`, `p`, and
  `g` pointers designate data inside the input buffer and therefore require
  that buffer to outlive their use.  It adds no Lupos branding.  The shorter
  Rust API comments are not a behavioural substitution for the Linux helper
  implementations.

## Frozen semantic-closure proposal

* `#ifndef _CRYPTO_DH_` / `#define _CRYPTO_DH_` / `#endif` (Linux lines 8–9,
  98; `SYMBOLS.tsv` rows 143217–143219) are a textual-C include guard only.
  Proposed closure: Rust module identity supplies the single-definition
  property; no Rust-exported `_CRYPTO_DH_` item is permitted or needed.
* `struct dh` requires the source-derived layout above.  Its three pointees
  are borrowed, non-owning byte regions; `crypto_dh_decode_key` and
  `__crypto_dh_decode_key` make all three point into the supplied packet
  buffer.  No allocation, refcount, lock, RCU, callback, or destruction rule
  occurs in this header.
* The four declarations cannot be closed from the frozen S012620 rows because
  finding 2 leaves their required manifest records absent.  Their source-level
  mapping in the candidate is otherwise the corresponding `unsafe extern
  "C"` declaration; the supplying implementation must preserve the helper
  error/validation behavior rather than treating these declarations as a
  replacement implementation.

