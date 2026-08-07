# Parity review — S016119 (slot 1)

Scope reviewed: pinned `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df` against `src/include/uapi/linux/ethtool_netlink_generated.rs`, plus the frozen task/symbol/config evidence and necessary pinned header callers.  This was a manual source-only review; no compiler, formatter, test, or diagnostic tool was used.

## Findings

1. **`ETHTOOL_GENL_NAME` and `ETHTOOL_MCGRP_MONITOR_NAME`: C string-literal macro semantics are not preserved.**  The pinned header defines `ETHTOOL_GENL_NAME` as `"ethtool"` at line 10 and `ETHTOOL_MCGRP_MONITOR_NAME` as `"monitor"` at line 962.  In C, each macro expands to a NUL-terminated character-array literal (and commonly decays to one C character pointer).  The candidate instead exposes `pub const ...: &str` at Rust lines 6 and 730.  A Rust `&str` is a pointer-and-length fat reference, has no required trailing NUL, and cannot be passed or stored with the C literal/pointer representation.  This changes both the UAPI value representation and FFI behavior; preserve C-string/NUL semantics rather than substituting `&str`.

2. **`enum ethtool_header_flags`, `enum ethtool_tcp_data_split`, `enum hwtstamp_source`, and `enum ethtool_pse_event`: the candidate erases the four distinct named C enum types.**  The pinned UAPI header declares these as named enum types at lines 29, 35, 48, and 71; the candidate makes all four `pub type ... = i32` aliases at lines 9–12.  Although the enumerator numeric values match, aliases are the same Rust type and provide neither the C named-enum identity nor an explicit ABI-bearing type.  This is operative in local pinned declarations: `struct kernel_ethtool_ts_info` uses `enum hwtstamp_source` at `vendor/linux/include/linux/ethtool.h:928`, and `struct hwtstamp_provider` and `struct kernel_hwtstamp_config` use it at `vendor/linux/include/linux/net_tstamp.h:39` and `:68`.  Retain distinct, explicitly represented types while preserving the C integer/bitwise value domain.

3. **`ETHTOOL_GENL_NAME` UAPI header provenance/SPDX is changed without authority.**  The header that defines `ETHTOOL_GENL_NAME` carries `SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)` at pinned line 1.  The candidate replaces it with `GPL-2.0-only` at Rust line 1.  This fails the required retention of the source SPDX identifier and changes the UAPI header's licensing notice; no branding allowlist evidence authorizes it.

## Checked without additional findings

- The frozen `SYMBOLS.tsv` has 649 enum-constant records and 68 enum/type records per architecture for this header.  Source inspection found 652 exported C macro/enumerator names (three real macros plus 649 enumerators), exactly matching the candidate's 652 public constants in order.  All 649 enum values, including implicit values and every `*_CNT`/`*_MAX` alias, match the pinned C evaluation.
- The candidate contains no functions, statics, structs, stubs, TODO/unimplemented markers, Rust test configuration, or non-allowlisted Lupos branding.  Its `pub const` visibility preserves the header's externally consumable identifiers.
- The C include guard `_UAPI_LINUX_ETHTOOL_NETLINK_GENERATED_H` is a private preprocessor multiple-inclusion mechanism.  It has no separate runtime/linkage symbol; the path-mapped Rust module has no textual C include operation to reproduce.  No independent guard finding is recorded on that basis.

Result: **FINDINGS — not parity-ready.**
