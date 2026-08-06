# S016119 application resolution

Reviewed the complete pinned
`include/uapi/linux/ethtool_netlink_generated.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate, and both
independent reports.  Both reported defects are resolved in the destination
file only.

1. **C string-literal macro representation — accepted and fixed.**
   Upstream defines `ETHTOOL_GENL_NAME` at line 10 as `"ethtool"` and
   `ETHTOOL_MCGRP_MONITOR_NAME` at line 962 as `"monitor"`.  The Rust
   constants are now `&[u8; 8]` byte-string values
   `b"ethtool\\0"` and `b"monitor\\0"`, respectively.  Each retains the
   terminating NUL and a thin pointer to the first byte is available only when
   the caller explicitly crosses the C-pointer boundary with `as_ptr()`.  This
   matches their uses for the `name` fields in
   `net/ethtool/netlink.c:1578` and `:1582`; the rejected `&str` form had
   neither the terminator nor pointer-compatible representation.

2. **Named enum types and enumerator values — accepted and fixed.**
   The four tags declared at header lines 29, 35, 48, and 71 remain distinct
   `#[repr(transparent)]` public types.  Their fourteen global enumerator
   constants now have the corresponding tagged type rather than bare `u32`:
   `ethtool_header_flags` (3), `ethtool_tcp_data_split` (3),
   `hwtstamp_source` (2), and `ethtool_pse_event` (6).  This permits direct
   use of `HWTSTAMP_SOURCE_NETDEV` where the named type is required, as in the
   upstream `enum hwtstamp_source` objects in `include/linux/ethtool.h:928`,
   `include/linux/net_tstamp.h:39,68`, and
   `net/ethtool/tsconfig.c:271`.  The wrapper's single `u32` field preserves
   the target's 32-bit enum object storage while retaining the C tag as a
   Rust type.

The generated header contains no selected configuration branch.  A final
name-inventory comparison found no missing or extra translated constants; the
two repairs change only the string and named-enum type surfaces, not any
numeric mapping.  SPDX and immutable provenance remain unchanged.  No build,
format, test, compiler, or runtime command was run.
