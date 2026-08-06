# Rust review — S016277

Reviewed candidate: `src/include/uapi/linux/netfilter/nf_tables.rs`  
Pinned source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`

Result: **changes required**.

## Findings

1. **HIGH — all public C enum types are absent, so this is not an ABI/type-surface translation.**

   Upstream declares 115 named enum types, beginning with `enum nft_registers`
   at `include/uapi/linux/netfilter/nf_tables.h:22`, `enum nft_verdicts` at line
   68, and continuing through `enum nft_tunnel_attributes` at line 2013.  The
   candidate has zero `pub enum`, `pub type`, `#[repr(...)]`, struct, or union
   declarations; it publishes only independent constants (`842` `u32` and five
   `i32`).  Consequently Rust callers cannot name, store, pass, or FFI-map any
   of the selected source types, and no representation/alignment contract
   exists for those types.  The constant types are also not a faithful mapping
   of C enum types: the candidate chooses `u32` for virtually every enum but
   `i32` for `nft_verdicts`, without an ABI decision.  This matters especially
   for `enum nft_data_types`, whose `NFT_DATA_VERDICT = 0xffffff00U` is an
   unsigned C enumerator (`nf_tables.h:504-507`), while the remaining enum
   definitions require their frozen C enum representation to be established.
   Provide a named Rust representation for every selected enum, with explicit
   ABI representation/underlying-type decisions recorded from the pinned
   target toolchain, and expose constants as associated values or otherwise
   preserve their typed use.  Do not infer one global `u32` representation.

2. **MEDIUM — the `__KERNEL__` conditional API was made unconditional.**

   Upstream defines `NFT_REG32_MAX` only under `#ifdef __KERNEL__`
   (`nf_tables.h:49-51`).  The candidate instead always exports
   `pub const NFT_REG32_MAX: u32 = 23` at line 979.  Its adjacent comment
   acknowledges the source condition but does not implement it.  Preserve the
   condition with the equivalent Rust build configuration, or establish and
   document in the task ABI record that this translated module is never
   visible to a non-kernel consumer.  The current public Rust interface has a
   symbol the non-kernel UAPI header intentionally does not provide.

3. **LOW — SPDX identifier was changed.**

   The pinned UAPI header is `SPDX-License-Identifier: GPL-2.0 WITH
   Linux-syscall-note` (`nf_tables.h:1`), while the candidate says
   `GPL-2.0-only` (Rust line 1).  This violates the required retention of the
   upstream SPDX notice and removes the syscall-note exception.  Restore the
   exact upstream identifier.

## Checked successfully

- Static name inventory found all 847 unique `NFT_*`/`__NFT_*` enumerator and
  object-like macro names from the pinned header represented by a candidate
  constant; no missing or extra name was found.  This includes the historical
  alias `NFT_META_IIFTYPE` (upstream line 978), masks, maxima, and the
  multi-line `NFT_FLOWTABLE_MASK` expression (upstream lines 1707-1712).
- The candidate contains no structs, unions, raw pointers, `unsafe` blocks,
  extern declarations, endian wrappers, or FFI code.  Therefore there is no
  separate unsafe/lifetime issue in its present constant-only implementation;
  the missing typed enum ABI is the controlling Rust/FFI issue.
- No source, queue, manifest, index, build, formatting, or test file was
  modified by this reviewer.
