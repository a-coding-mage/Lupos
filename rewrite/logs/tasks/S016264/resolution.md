# Resolution — S016264

Applier independently reopened the complete pinned upstream source
`vendor/linux/include/uapi/linux/net_namespace.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen task/manifests, the
candidate, both independent reviews, and selected local consumers
`net/core/net_namespace.c`, `net/netlink/af_netlink.c`, and `net/psp/psp_nl.c`.

## Finding dispositions

### P1 — original UAPI GPLv2 grant notice omitted: resolved

The candidate now retains the upstream notice verbatim in Rust comments:

> This program is free software; you can redistribute it and/or modify it
> under the terms and conditions of the GNU General Public License,
> version 2, as published by the Free Software Foundation.

This is the complete grant paragraph from upstream lines 5–8. The immutable
provenance remains unchanged and no branding delta was introduced.

## Independent final semantic closure

- The anonymous C enum has no named object or layout-bearing type. Its
  enumerators are C `int` constants, represented here as public `i32`
  constants: `NETNSA_NONE = 0`, `NETNSA_NSID = 1`, `NETNSA_PID = 2`,
  `NETNSA_FD = 3`, `NETNSA_TARGET_NSID = 4`, `NETNSA_CURRENT_NSID = 5`, and
  `__NETNSA_MAX = 6`.
- `NETNSA_NSID_NOT_ASSIGNED` is an independent macro between the first and
  second enumerators; it is not an enum member and remains signed `i32` value
  `-1`. Consequently it does not alter the following enumerator sequence.
- `NETNSA_MAX` remains the signed expression `__NETNSA_MAX - 1`, hence `5`.
  This matches the source consumers' `NETNSA_MAX + 1` attribute table bounds
  and inclusive selector loops, while preserving the source-level derivation.
- The include guard has no Rust runtime counterpart. The frozen x86_64 and
  AArch64 selection records both select this common UAPI header with no
  configuration-dependent branch. It declares no storage, ABI layout, FFI
  symbol, ownership/lifetime, locking, RCU, refcount, allocation, cleanup, or
  unsafe boundary.

The S016264 `PENDING_REVIEW` entries describe only the include guard, macros,
and anonymous constants above; their source semantics are closed by this
resolution. No compiler, formatter, linker, test, runtime, or diagnostic tool
was used.
