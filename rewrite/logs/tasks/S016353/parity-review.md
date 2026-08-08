# Parity review — S016353 / attempt 1 / P01

Reviewer: parity reviewer (`gpt-5.6-terra`, high)

Reviewed source only: `vendor/linux/include/uapi/linux/reboot.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen scope/symbol manifests,
the relevant pinned reboot consumers, and the candidate snapshot.

## Result: FINDINGS

### P1 — UAPI macro expression types are not preserved

`LINUX_REBOOT_MAGIC1` and `LINUX_REBOOT_CMD_HALT`,
`LINUX_REBOOT_CMD_CAD_ON`, `LINUX_REBOOT_CMD_POWER_OFF`,
`LINUX_REBOOT_CMD_RESTART2`, and `LINUX_REBOOT_CMD_SW_SUSPEND` are unsuffixed
hexadecimal integer constants whose values do not fit `int`; under the frozen
x86_64 and AArch64 C integer ranks they have type `unsigned int`.  In contrast,
the remaining decimal and fitting hexadecimal literals in the header have type
`int`.  The candidate exposes every macro as `u32`, changing the public
expression type for `LINUX_REBOOT_MAGIC2`, `LINUX_REBOOT_MAGIC2A`,
`LINUX_REBOOT_MAGIC2B`, `LINUX_REBOOT_MAGIC2C`, `LINUX_REBOOT_CMD_RESTART`,
`LINUX_REBOOT_CMD_CAD_OFF`, and `LINUX_REBOOT_CMD_KEXEC`.

This is not merely documentary: `vendor/linux/kernel/reboot.c:731-805` accepts
`magic1`, `magic2`, and `cmd` as `int`, compares them with these macros, assigns
`LINUX_REBOOT_CMD_HALT` to `cmd`, and switches on `cmd`; `kernel/pid_namespace.c:320-340`
also switches on an `int cmd`.  C's usual arithmetic conversions and the
conversion on assignment are therefore part of the selected behavior.  A Rust
consumer cannot use these `u32` constants in an `i32` ABI/command context without
an explicit conversion, and the candidate establishes neither a source-proven
conversion boundary nor the C expression-type contract.

Evidence: `include/uapi/linux/reboot.h:9-13,29-36`; the cited selected kernel
consumer contexts.  This must be resolved from pinned-source-compatible UAPI
translation rules before the task can be accepted.

### P1 — Upstream UAPI SPDX identifier was changed

The pinned header begins `/* SPDX-License-Identifier: GPL-2.0 WITH
Linux-syscall-note */` (`include/uapi/linux/reboot.h:1`).  The candidate begins
`// SPDX-License-Identifier: GPL-2.0-only`.  The latter drops the UAPI
`Linux-syscall-note` exception and is not an allowlisted branding change.

Evidence: pinned header line 1; candidate snapshot line 1.

### P2 — C include-guard contract has no explicit Rust-level mapping

The frozen symbols inventory selects `_UAPI_LINUX_REBOOT_H` and its enclosing
`#ifndef`/`#endif` for both architectures (`include/uapi/linux/reboot.h:2-3,40`).
The candidate contains no documented mapping for this namespace/one-definition
contract.  A Rust module may supply an analogous single-module namespace, but
the task evidence must establish the source-grounded mapping rather than silently
dropping each selected conditional/macro.

Evidence: `SYMBOLS.tsv` rows for `_UAPI_LINUX_REBOOT_H`, `ifndef@2`, and
`endif@40`; candidate snapshot has no corresponding mapping.

No compiler, formatter, analyzer, test, or runtime command was used.
