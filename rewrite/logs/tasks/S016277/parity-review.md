# Parity review — S016277

Reviewed `vendor/linux/include/uapi/linux/netfilter/nf_tables.h` at pinned
revision `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/netfilter/nf_tables.rs`.

## Findings

1. **P1 — all selected named enum types are omitted, so the candidate does not
   reproduce the public typed UAPI.**  Upstream declares 115 named types,
   starting with `enum nft_registers` at upstream line 22 and ending with
   `enum nft_tunnel_attributes` at line 2013.  The frozen symbol inventory has
   115 distinct `type` records for S016277 (one per architecture, 230 rows in
   total), and the ABI manifest likewise records these as named enum types.
   The candidate has zero `pub enum` and zero `pub type` declarations: each
   source declaration is represented only by a `// enum ...` comment followed
   by untyped `u32` constants (for example candidate lines 10--32,
   41--76, and 963--968).  Comments do not provide a Rust symbol for types
   such as `nft_registers`, `nft_verdicts`, `nft_table_attributes`, or the
   other 112 selected enum types.  Flattening all enumerators to `u32`/`i32`
   also discards the distinct named-type API and its C-compatible ABI
   representation.  The applier must add an explicit faithful representation
   for every selected named enum type, with a documented representation and
   constants/values that retain its source association; a single untyped
   numeric namespace is not a type mapping.

2. **P1 — `NFT_REG32_MAX` loses its required `__KERNEL__` conditional.**
   Upstream lines 49--51 define this macro only inside `#ifdef __KERNEL__`.
   That conditional is itself recorded for S016277 in `SYMBOLS.tsv`
   (`ifdef@49`/`endif@51`).  Candidate line 979 exports `NFT_REG32_MAX`
   unconditionally; the preceding comment acknowledges the restriction but
   does not implement it.  Preserve the selected compile-time branch rather
   than widening this UAPI-facing symbol.

3. **P1 — SPDX provenance changes the upstream syscall-note license
   identifier.**  Upstream line 1 is
   `SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note`; candidate line
   1 says `GPL-2.0-only`.  This drops the Linux syscall exception and conflicts
   with the requirement to retain upstream SPDX identifiers.  Preserve the
   source SPDX expression in the translated file provenance.

## Exhaustive namespace check

I compared all enum-declaration labels (115 upstream / 115 candidate labels)
and the object-like macro/enumerator identifier set.  No enumerator or
object-like macro identifier is absent from the candidate other than the C
include guard, which has no direct Rust-module equivalent.  The candidate
does include the expected numeric constants and aliases, but that name/value
coverage does not resolve the missing named types or the conditional and
provenance findings above.

No source, manifest, queue, build, format, or test action was performed.
