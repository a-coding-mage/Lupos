# Parity review — S016264

Reviewed independently against pinned `vendor/linux/include/uapi/linux/net_namespace.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the S016264 row in the frozen manifests, and the three mechanically selected consumers (`net/core/dev.c`, `net/core/net_namespace.c`, and `net/netlink/af_netlink.c`) for both x86_64 and AArch64.

## Finding P1 — original UAPI license notice omitted

`src/include/uapi/linux/net_namespace.rs` preserves the SPDX identifier, copyright holder, author, and immutable provenance, but drops the original header's GPLv2 grant paragraph:

> This program is free software; you can redistribute it and/or modify it under the terms and conditions of the GNU General Public License, version 2, as published by the Free Software Foundation.

This is a UAPI copyright/license notice in the pinned source, not a branding delta; `rewrite/BRANDING_ALLOWLIST.tsv` contains no authorization to omit it. Restore the notice as a Rust comment without changing the source/revision/architecture/task provenance.

## Checked with no parity finding

- The anonymous C enum has no named enum type or object. Its enumerators are C `int` constants on both selected targets. The candidate exposes the exact `i32` sequence: `NETNSA_NONE = 0`, `NETNSA_NSID = 1`, `NETNSA_PID = 2`, `NETNSA_FD = 3`, `NETNSA_TARGET_NSID = 4`, `NETNSA_CURRENT_NSID = 5`, and `__NETNSA_MAX = 6`.
- `NETNSA_NSID_NOT_ASSIGNED` remains a separate `i32` constant with value `-1`, positioned after `NETNSA_NONE` in the source order. It is not an enum member in C, and the candidate does not incorrectly shift the later selectors.
- `NETNSA_MAX` remains derived from `__NETNSA_MAX - 1`, yielding `5`; this matches the selected `net/core/net_namespace.c` policy/table bounds, parser bounds, and selector loop.
- The header has no configuration-controlled branches. It is selected for `common`, with `CONFIG_NET=y` and `CONFIG_NET_NS=y` in both frozen configurations; its values and consumers are architecture-invariant.
- No exported data/function symbol, layout, allocation, locking, cleanup, or runtime state exists in this header.

No compiler, formatter, linker, test, emulator, debugger, or benchmark was run.
