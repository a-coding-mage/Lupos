# S016119 parity review (slot 1)

Reviewed `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/ethtool_netlink_generated.rs`.

## Result: findings required

1. **UAPI string macros do not retain C-string/FFI representation.**
   Upstream lines 10 and 962 define `ETHTOOL_GENL_NAME` and
   `ETHTOOL_MCGRP_MONITOR_NAME` as C string literals.  Such a macro supplies
   a NUL-terminated character array and, in expression context, a thin pointer
   to its first character.  The candidate lines 23 and 811 instead publish
   non-NUL-terminated Rust `&str` values (a UTF-8 slice/fat pointer).  This is
   not substitutable at the upstream uses: `net/ethtool/netlink.c:1578` puts
   the monitor macro into `struct genl_multicast_group.name`, and line 1582
   puts the family macro into `struct genl_family.name`, both C-string pointer
   fields.  Represent the macros with an ABI-appropriate NUL-terminated
   byte/C-string form rather than `&str`.

2. **Named enum declarations are not faithfully exposed.**
   Upstream lines 29, 35, 48, and 71 declare the four named C enum types.
   Candidate lines 9--21 replace each with a transparent `u32` newtype while
   declaring every corresponding enumerator as a standalone `u32` constant.
   Thus, for example, `HWTSTAMP_SOURCE_NETDEV` (candidate line 44) cannot be
   used where candidate `hwtstamp_source` is required; the wrapper has no
   associated values or conversion preserving the C declaration.  The
   candidate comment's asserted "full unsigned netlink value range" is not
   evidence from the source and is incompatible with the source's concrete
   named-enum contract.  This tag is operational outside the header:
   `net/ethtool/tsconfig.c:271` declares `enum hwtstamp_source source`, and
   `include/linux/net_tstamp.h:39,68` stores the type in kernel structures.
   Re-express the named enums and their enumerators with the pinned
   architecture's established C-enum representation and retain the values as
   that enum type; do not invent an unrelated unrestricted `u32` wrapper.

## Exhaustive checks that passed

- SPDX and immutable task/source/revision provenance are present and match the
  task and `vendor/linux.SHA`; the task architecture is correctly `common`.
- The source has only the include guard (no selected configuration branches).
- A complete declaration inventory found all 650 numeric enumerators in the
  candidate, with identical identifier spelling and evaluated numeric value,
  including every `*_CNT`/`*_MAX` relation.  The three upstream object-like
  macros are present by name; findings 1 and 2 concern their/type semantics,
  not numeric omissions.
- No functions, structs, driver code, tests, branding changes, or prohibited
  placeholders were introduced.

The frozen identity and queue verified before this review; no build, format,
test, or runtime command was run.
