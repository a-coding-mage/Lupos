# Application resolution — S016119

The applier reopened the complete pinned
`vendor/linux/include/uapi/linux/ethtool_netlink_generated.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate, both
fresh reviews, the task queue row, frozen configurations, `SCOPE.tsv`,
`SYMBOLS.tsv`, `ABI.tsv`, `LIFETIMES.tsv`, and `PORTING.md`. This was manual
source inspection only; no compiler, formatter, linker, test, diagnostic, or
historical Rust source was used.

| Finding | Disposition | Pinned source evidence |
| --- | --- | --- |
| Parity 1 — `ETHTOOL_GENL_NAME` and `ETHTOOL_MCGRP_MONITOR_NAME` were `&str` | Resolved. Both are now public fixed-size byte-array references containing their terminating NUL: `b"ethtool\\0"` and `b"monitor\\0"`. This preserves the literal bytes, extent, and C-pointer availability without allocation or a fat `&str`. | The macros are string literals at header lines 10 and 962. `net/ethtool/netlink.c:1578` initializes `.name` from `ETHTOOL_MCGRP_MONITOR_NAME`, and line 1582 initializes `.name` from `ETHTOOL_GENL_NAME`. |
| Parity 2 — named enum types erased as `i32` aliases | Blocked. The aliases cannot be accepted as a final representation, but no exact replacement can be established from the permitted source evidence. The header declares four distinct C enum types at lines 29, 35, 48, and 71. A transparent `i32` newtype would assume the C enum storage ABI; a Rust `repr(C)` enum would impose a closed valid-discriminant domain incompatible with the active bitwise `enum ethtool_pse_event` use. Neither assumption is established by the frozen source records. | `include/linux/ethtool.h:928` and `include/linux/net_tstamp.h:39,68` store `enum hwtstamp_source`; `net/ethtool/tsconfig.c:271` declares it. `drivers/net/pse-pd/tps23881.c:1147` ORs an `enum ethtool_pse_event` value into an `unsigned long` event mask, and line 1244 carries that enum through control flow. All 136 S016119 ABI rows and all 136 lifetime rows remain `PENDING_REVIEW`; none supplies a size, alignment, or compatible-integer fact for either frozen architecture. |
| Parity 3 — upstream UAPI SPDX expression replaced | Resolved. The first line now retains the exact upstream SPDX expression. The four immutable provenance lines remain one each and unchanged. | Header line 1 is `SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`; no branding allowlist entry authorizes a change. |
| Rust 1 — C-facing string macro ABI | Resolved by the same NUL-terminated fixed-size public byte-array representations described for Parity 1. | Header lines 10 and 962, plus their active `.name` consumers at `net/ethtool/netlink.c:1578,1582`. |
| Rust 2 — SPDX expression not retained | Resolved by the exact upstream SPDX line described for Parity 3. | Header line 1. |
| Rust 3 — named enum ABI/type identity unresolved | Blocked for the same source-evidence gap described for Parity 2. The nominal Rust representation and C storage ABI cannot be selected without guessing. | Header lines 29, 35, 48, and 71; active `hwtstamp_source` storage at `include/linux/ethtool.h:928` and `include/linux/net_tstamp.h:39,68`; active `ethtool_pse_event` bitwise use at `drivers/net/pse-pd/tps23881.c:1147,1244`; frozen ABI/lifetime records remain `PENDING_REVIEW`. |

The source include guard at header lines 7–8 and 964 is a C preprocessor
multiple-inclusion mechanism. The frozen scope maps this header one-to-one to
the Rust module and `PORTING.md` contains no rule requiring a runtime or
linkage substitute for that mechanism; no guard symbol or replacement was
introduced.

All implicit enum values, explicit values, `*_CNT` and `*_MAX` expressions,
and public constant names outside the two corrected string macros were left
unchanged. The unresolved named-enum ABI prevents closure of the task semantic
records and therefore prevents `DONE`.
