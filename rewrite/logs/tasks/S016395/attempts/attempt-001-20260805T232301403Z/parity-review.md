# Parity review — S016395 (slot 1)

Reviewed `src/include/uapi/linux/sunrpc_netlink.rs` against the complete pinned
`vendor/linux/include/uapi/linux/sunrpc_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, including the selected x86_64 and
AArch64 header context and the local SunRPC consumers.  No build, formatter,
test, or runtime command was run.

## Findings

1. **High — `SUNRPC_CACHE_TYPE_*` has the wrong source type.**  The C named
   enum is declared at `sunrpc_netlink.h:13-16`, but in the frozen GNU11 C
   source its enumerators are `int` integer-constant expressions.  The
   candidate instead gives `SUNRPC_CACHE_TYPE_IP_MAP` and
   `SUNRPC_CACHE_TYPE_UNIX_GID` the newtype `sunrpc_cache_type`
   (`sunrpc_netlink.rs:33-34`).  This is not substitutable at its selected
   call sites: `net/sunrpc/svcauth_unix.c:590,1291` pass the enumerators to
   `sunrpc_cache_notify(..., u32 cache_type)` (`cache.c:1979-1980`), and
   `svcauth_unix.c:842,846` applies bitwise `&` with those enumerators.
   Preserve enumerator integer-constant semantics (and separately model the
   tagged enum object type only where it is actually required).

2. **High — the three string-literal macros were narrowed to pointer values.**
   `SUNRPC_FAMILY_NAME`, `SUNRPC_MCGRP_NONE`, and `SUNRPC_MCGRP_EXPORTD` are
   C macros expanding to string literals, hence array expressions with the
   literal's exact length, element type, indexing, and `sizeof` behavior.
   The candidate exposes only `*const c_char` constants
   (`sunrpc_netlink.rs:30,92,104`).  This loses those semantics and fails the
   concrete selected use at `net/sunrpc/netlink.c:87-89`: the C
   `SUNRPC_FAMILY_NAME` literal initializes the fixed `genl_family.name` char
   array; a pointer does not do so.  The static backing arrays are private,
   so downstream Rust translations cannot use their array form either.  Keep
   public literal-array forms (with the frozen Kbuild `-funsigned-char`
   element semantics) or an equally capable translation mechanism; provide a
   decay pointer only at pointer-consumption sites.

## Verified items

- Exact SPDX expression and immutable provenance identify the correct source,
  revision, common architecture set, and task ID.
- All seven anonymous enum sets, their hidden maxima, and derived public
  `*_MAX` values are present with the correct integer values: cache-notify
  `1/2/1`; IP-map `1..7/6`; IP-map-requests `1/2/1`; UNIX-GID `1..6/5`;
  UNIX-GID-requests `1/2/1`; cache-flush `1/2/1`; commands `1..7/6`.
- `SUNRPC_FAMILY_VERSION` is present with value `1`; no configuration branch
  is omitted apart from the non-semantic C include guard.
- No unauthorized branding, executable behavior, tests, placeholders, or
  source omissions were found beyond the two type/representation defects
  above.

Result: **reject pending applier correction of both findings.**
