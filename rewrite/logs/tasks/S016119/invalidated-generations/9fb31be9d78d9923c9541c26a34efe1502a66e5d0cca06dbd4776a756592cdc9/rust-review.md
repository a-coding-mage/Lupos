# Rust review — S016119

Scope reviewed independently: `src/include/uapi/linux/ethtool_netlink_generated.rs` against pinned `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.  The task row is leased to P02 and was `REVIEWING` when inspected.  This was a manual source review only; no compiler, formatter, test, Git command, historical Rust source, or other task evidence was read.

## Findings

1. **UAPI string macro representation changes the C-facing value and ABI.**  The pinned header defines `ETHTOOL_GENL_NAME` and `ETHTOOL_MCGRP_MONITOR_NAME` as C string-literal macros at lines 10 and 962.  In a C expression those literals carry a terminating NUL and can decay to `const char *`.  The candidate instead exposes `&str` at lines 6 and 730.  A Rust `&str` is a fat data/length value and neither represents nor guarantees a NUL-terminated C string.  It therefore cannot be substituted at a Linux-facing call or structure field expecting the source macro's C-string semantics.  Preserve the source-level macro contract with a NUL-terminated representation usable at the C ABI boundary, without introducing an allocation or panic path.

2. **The source SPDX expression was not retained.**  The source begins with `SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)` (header line 1); candidate line 1 changes it to `GPL-2.0-only`.  This is an unauthorized removal of the UAPI syscall-note/BSD licensing expression and violates the required retention of upstream SPDX identifiers.  Restore the exact upstream identifier.

3. **Named-enum ABI and type identity remain unresolved; the candidate erases them as aliases.**  The pinned header declares the distinct tagged enum types `enum ethtool_header_flags`, `enum ethtool_tcp_data_split`, `enum hwtstamp_source`, and `enum ethtool_pse_event` at lines 29, 35, 48, and 71.  Candidate lines 9–12 replace these with four aliases of `i32`; aliases have no nominal identity or independently documented C ABI.  This can alter compile-time type compatibility and gives no `repr(C)`/layout proof at interfaces carrying these values.  The frozen evidence does not resolve that uncertainty: all 136 S016119 ABI rows, all 136 lifetime rows, and all 1,446 symbol rows remain `PENDING_REVIEW`.  The applier must establish the selected C enum representation and required cross-boundary uses from pinned source/context, then use a representation that preserves both required ABI and type contract (or block the task if exact parity cannot be established).

## Checked items

- Candidate provenance names the pinned source, exact revision, `common` architectures, and task ID (lines 2–5).
- Manual comparison found all 652 header enumerator/macro names represented by 652 Rust constants.  Explicit integer sequences preserve the examined implicit C enumerator values, including `*_CNT - 1` maxima; values are small and the Rust subtractions cannot overflow here.
- No `unsafe`, raw-pointer operations, FFI declarations, `repr`, interior mutability, allocation, callbacks, `Drop`, panic/unwrap/expect, conditional test code, or project-authored Rust tests appear in the candidate.  Consequently there is no borrow, aliasing, pinning, Send/Sync, pointer provenance, or Drop-timing implementation to approve in this file.
- Aside from the SPDX loss above, no Lupos branding was introduced; no configuration guard in the source header was dropped other than the C include guard, which has no direct Rust analogue.

Result: **FINDINGS — not ready for source acceptance until all three findings, including the unresolved frozen ABI/lifetime/symbol records, are resolved with pinned-source evidence.**
