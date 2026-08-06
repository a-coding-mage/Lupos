# S016267 parity review

Reviewer: parity reviewer (independent)

## Scope and evidence

Reviewed the complete pinned source `vendor/linux/include/uapi/linux/netdev.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df` against only
`src/include/uapi/linux/netdev.rs`.

The queue row identifies this as the common x86_64/AArch64 UAPI header task.
The candidate provenance matches the pinned source path, revision, common
architecture membership, and task ID `S016267`; its SPDX identifier is exactly
the upstream identifier.

## Comparison

- All 143 meaningful upstream `NETDEV_*` and `__NETDEV_*` identifiers are
  present exactly once in the candidate.  The only extra source token in the
  raw identifier-set comparison is `NETDEV_H`, from the C include guard; the
  guard is not a Rust UAPI item.
- The six tagged C enums (`netdev_xdp_act`, `netdev_xdp_rx_metadata`,
  `netdev_xsk_flags`, `netdev_queue_type`, `netdev_qstats_scope`, and
  `netdev_napi_threaded`) are each represented by a distinct transparent
  `c_int` newtype.  Their enumerator values, including the zero-based values,
  match the source.
- All eleven anonymous enum namespaces have their members and evaluated
  values preserved.  This includes each `__*_MAX` sentinel, every public
  `*_MAX = __*_MAX - 1`, the `NETDEV_A_XSK_INFO_MAX == -1` result, and the
  intentional `NETDEV_A_PAGE_POOL_STATS_ALLOC_FAST = 8` / `NETDEV_A_QSTATS_RX_PACKETS = 8`
  numbering gaps.
- The four object-like macros retain their values: family version is `1`, and
  the three string values are the original ASCII bytes with their required C
  NUL terminator (`netdev`, `mgmt`, and `page-pool`).
- The pinned header has no functions, data structures, configuration-selected
  branches, locking, cleanup, driver code, or branding delta.  The candidate
  introduces none of those and contains no placeholder, panic, test, or
  unsupported-feature marker.
- Read-only consumer checks confirm that the tagged enum names are used in
  typed net queue/NAPI APIs and that the family macro supplies the generic
  netlink family name/version; the candidate preserves the corresponding
  named types and values.

## Findings

No findings.  The candidate is source-parity complete for this task.

No compiler, formatter, linker, test, emulator, debugger, or runtime command
was run.
