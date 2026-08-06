# Parity review — S016342 (slot 1)

Reviewed the complete pinned `vendor/linux/include/uapi/linux/psample.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/psample.rs`, the current scope/queue/symbol/ABI/lifetime
records, header-closure evidence, frozen configurations, and the relevant
pinned include/caller context.  This was a source-only review; no build, test,
or formatter was run.

## Findings

1. **P1 — named C enum types are collapsed into one Rust integer type.**
   Upstream declares two separately named public enum types: `enum
   psample_command` at `include/uapi/linux/psample.h:35-40` and `enum
   psample_tunnel_key_attr` at `:42-61`.  The candidate instead declares
   `pub type psample_command = i32` (`psample.rs:44`) and `pub type
   psample_tunnel_key_attr = i32` (`:51`).  Rust type aliases are not distinct
   types: both names and `i32` are mutually interchangeable, and neither
   carries a C enum representation or a separately named enum ABI surface.
   This contradicts the candidate comment at `:10-12` that the aliases retain
   the tagged enum domains.  The ABI records for both enum tags remain
   `PENDING_REVIEW` for both x86_64 and aarch64 (`rewrite/ABI.tsv:194477-194481`),
   so the applier must establish the frozen-target C representation and retain
   both distinct tagged public types while preserving the unscoped enumerator
   values.

2. **P1 — the three C string-literal macros no longer have C string-literal
   expansion semantics.**  `PSAMPLE_NL_MCGRP_CONFIG_NAME`,
   `PSAMPLE_NL_MCGRP_SAMPLE_NAME`, and `PSAMPLE_GENL_NAME` expand to C `char`
   array literals at upstream `psample.h:66-68`; in pointer contexts they decay
   to C character pointers (as used by the `genl_family.name` and multicast
   group initializers in `net/psample/psample.c:33-34,89`).  The candidate
   changes them into Rust references to `[u8; N]` at `psample.rs:92-94`.
   Although the ASCII bytes and terminal NUL are correct, a Rust reference to
   an unsigned-byte array is neither the C literal array expansion nor a
   C-compatible character pointer surface.  It also requires an extra,
   unrecorded conversion before use at an FFI boundary.  Preserve the exact
   NUL-terminated literal bytes with a representation whose pointer/element
   semantics are explicitly compatible with the frozen C contract.

## Verified parity

- The anonymous attribute enumerators are present in source order with values
  `0..=17`; the public `PSAMPLE_ATTR_MAX` still evaluates to `16` from the
  sentinel expression.
- All four command values are `0..=3`; all tunnel-key enumerator values,
  including `__PSAMPLE_TUNNEL_KEY_ATTR_MAX`, are `0..=17`.  No extra public
  tunnel maximum macro was added.
- `PSAMPLE_GENL_VERSION` is `1`, and the three string spellings and NUL byte
  counts are correct.  The payload-width, byte-order, flag, nested-data, and
  scaled-probability comments are retained materially.
- The upstream header has no configuration conditional other than its include
  guard.  Both frozen configurations have `CONFIG_PSAMPLE` unset, but the
  header is nevertheless mechanically selected through `net/sched/cls_api.o`
  on both architectures (`rewrite/metadata/header_closure.tsv:7074,11398`);
  no omitted selected branch was found.
- SPDX, Linux source path, pinned SHA, task ID, and all Linux UAPI identifiers
  are retained.  The branding allowlist is empty and no branding delta or Rust
  test/placeholder was found.  The task is mapped and queued as `common`;
  the candidate's `architectures: common` provenance matches that task field.

## Required applier disposition

Resolve both P1 findings against the pinned source and frozen-target ABI, then
close the `PENDING_REVIEW` enum ABI/lifetime records before `DONE`.  No other
source omission was identified in this review.
