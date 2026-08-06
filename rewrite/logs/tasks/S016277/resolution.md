# Resolution — S016277

Pinned source reviewed: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Review finding dispositions

1. **Parity P1 / Rust HIGH — omitted C enum tags: resolved.**  The Rust file
   now declares every one of the 115 named upstream enum tags, in the same
   source order, through 115 `nft_uapi_enum!` invocations.  That macro expands
   each listed tag to a public `#[repr(transparent)]` tuple struct, preserving
   its distinct C tag namespace and the 32-bit integer object representation.
   `nft_verdicts` is the signed `i32` representation required by its
   `-1` through `-5` source values; the other tags, including
   `nft_data_types` with `NFT_DATA_VERDICT = 0xffffff00U`, use `u32`.
   Transparent integer wrappers are used instead of closed Rust enums because
   a C enum object can carry an integer outside the enumerators listed in the
   declaration.  The pre-existing 847 global enumerator/object-macro
   identifiers and their reviewed numeric mappings remain intact; the 115
   adjacent `// enum <tag>` group markers retain each identifier's source-tag
   association.

   Independent static reconciliation: the upstream `^enum <tag> {` inventory
   has 115 names and the Rust `nft_uapi_enum!` inventory has 115 names; sorted
   name comparison has no differences.  This covers the first source tag
   `nft_registers` (line 22) through `nft_tunnel_attributes` (line 2013).

2. **Parity P1 / Rust MEDIUM — widened `NFT_REG32_MAX`: resolved.**  Upstream
   exposes it only between `#ifdef __KERNEL__` at lines 49--51.  The Rust
   declaration is now gated by `#[cfg(feature = "__KERNEL__")]`, the
   compile-time Rust configuration corresponding directly to that source
   define.  There is no runtime condition or fallback symbol, so a build
   without the source kernel define does not receive this API.

3. **Parity P1 / Rust LOW — SPDX mismatch: resolved.**  The file now starts
   with the exact upstream expression
   `SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note`; immutable
   task/source/revision/architecture provenance remains present immediately
   below it.

## Final source-only checks

- The candidate contains no `todo!`, `unimplemented!`, Rust unit-test
  configuration, unsafe code, runtime substitution, or driver code.
- No source outside the leased destination and no queue/manifest/index was
  edited by this applier.  No compiler, formatter, build, test, linker,
  emulator, debugger, or benchmark command was run.

All reviewer findings are resolved with the pinned header as the implementation
oracle.
