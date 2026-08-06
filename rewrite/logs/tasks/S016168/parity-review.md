# Parity review — S016168

Result: **CHANGES REQUIRED**

Reviewed pinned source `vendor/linux/include/uapi/linux/if_infiniband.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/if_infiniband.rs` for the frozen `common` scope.

## Finding P1 — upstream dual-license notice was materially truncated

`src/include/uapi/linux/if_infiniband.rs:7` retains only the Topspin copyright
line.  It omits the pinned header's complete preceding dual-license notice:
the GPL-v2/OpenIB.org BSD choice, both referenced license locations, the
full warranty/liability disclaimer, and the `$Id$` notice
(`vendor/linux/include/uapi/linux/if_infiniband.h:2-23`).  The SPDX line alone
does not preserve these relevant upstream UAPI licensing/copyright notices.

Resolution required: retain the complete upstream notice in the Rust source,
in addition to the immutable provenance header, without altering its terms.

## Verified source parity items

- The sole exported operational definition is present as public
  `INFINIBAND_ALEN` with value `20`; its `i32` representation matches the
  unsuffixed C integer constant on both frozen Linux architectures.
- There are no configuration-controlled branches.  The C include guard only
  prevents repeated textual C inclusion; the Rust module/item definition has
  equivalent single-definition behavior and requires no runtime or public
  symbol representation.
- The recorded selected consumer is `net/ipv6/addrconf.c`, which includes the
  header and compares `dev->addr_len` to `INFINIBAND_ALEN`; the candidate keeps
  the required name and value.
- SPDX expression, Linux source path, pinned revision, architecture scope, and
  task provenance match the task and pinned source.

No compiler, formatter, linker, test, runtime, or diagnostic tool was used.
