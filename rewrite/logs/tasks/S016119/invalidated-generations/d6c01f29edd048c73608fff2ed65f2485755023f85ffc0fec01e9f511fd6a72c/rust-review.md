# Rust review — S016119 (slot 2)

Reviewed `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h` in full
against `src/include/uapi/linux/ethtool_netlink_generated.rs`.  This review did
not read the parity-review report and made no source changes.

## Findings

1. **High — named-enum constants have the wrong Rust type, so the declared
   replacement enum types cannot be used as their C counterparts.**

   The four source declarations at
   `include/uapi/linux/ethtool_netlink_generated.h:29-79` introduce the named
   C enum types `ethtool_header_flags`, `ethtool_tcp_data_split`,
   `hwtstamp_source`, and `ethtool_pse_event`.  The candidate introduces
   transparent `u32` newtypes at Rust lines 9-21, but exposes every associated
   enumerator as a separate `u32` constant (lines 34-54), not as a value of the
   corresponding newtype.  Rust consequently cannot initialize or assign a
   `hwtstamp_source` from `HWTSTAMP_SOURCE_NETDEV` without an ad-hoc conversion,
   whereas C does so directly.  This is not theoretical: pinned source stores
   `enum hwtstamp_source` in `include/linux/ethtool.h:928` and
   `include/linux/net_tstamp.h:39,68`, and declares a local of that type in
   `net/ethtool/tsconfig.c:271` before assigning the enumerators.

   Resolve by choosing the frozen target ABI representation for each named
   enum and publishing the associated constants with that exact Rust wrapper
   type (or another representation that preserves both the named type and its
   associated values).  Do not assert an unsigned representation merely from
   netlink usage; the current `u32` wrapper and its comment at Rust line 17 are
   unsupported by source/ABI evidence.

2. **High — C string-literal macros lost their NUL-terminated byte-string and
   C-pointer semantics.**

   `ETHTOOL_GENL_NAME` and `ETHTOOL_MCGRP_MONITOR_NAME` are C string-literal
   macros at source lines 10 and 962.  Each expands to a NUL-terminated char
   array in C expression context.  The candidate changes them to `&str` at
   Rust lines 23 and 811: this is a fat Rust slice reference, contains no
   trailing NUL, and cannot be used as the C-string pointer expected by the
   original initialization.  The pinned user at `net/ethtool/netlink.c:1578,
   1582` initializes `.name` fields directly from these macros.

   Preserve a NUL-terminated byte representation suitable for the frozen
   `genl_*` field ABI (and expose a correctly typed C pointer only at the FFI
   boundary); `&str` is not ABI-equivalent.

## Checks with no discrepancy

- Exhaustive parse/evaluation found all 652 C enumerator/object-like macro
  names present in Rust, with no extra names and no numeric/string-value
  mismatch.  This includes every `*_CNT`/`*_MAX` expression.
- The four named C enum tags are present in Rust by name.
- The only C preprocessor conditional is the conventional include guard; the
  source has no feature/configuration conditional branch requiring a Rust
  `cfg` translation.
- SPDX and immutable provenance match the task, source path, revision, and
  `common` architecture membership.  No unsafe code, FFI declaration, or
  Rust test configuration is present.

