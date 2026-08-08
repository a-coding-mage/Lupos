# Resolution — S016353 / attempt 1 / P01

Applier: `gpt-5.6-terra`, high effort.  This adjudication used only the pinned
Linux source at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen task
records, the sealed candidate snapshot, and the two independent reviews.  No
compiler, formatter, analyzer, test, runtime command, or historical Lupos
source was used.

## Dispositions

### P1 / R1 — accepted; unresolved source-to-Rust UAPI type bridge (BLOCKED)

`include/uapi/linux/reboot.h:9-13,29-36` defines unsuffixed integer literals,
not a uniform unsigned-32-bit API.  On the approved C ABIs, literals that fit
`int` are signed `int`, while the hexadecimal literals above `INT_MAX` select
`unsigned int`.  The selected syscall definition in
`kernel/reboot.c:728-805` accepts `magic1` and `magic2` as `int` and `cmd` as
`unsigned int`; it compares and switches on these macros.  The selected
`reboot_pid_ns` consumer accepts `int cmd` (`kernel/pid_namespace.c:320-340`),
and the pinned nolibc wrapper passes the magic macros through `int` arguments
(`tools/include/nolibc/sys/reboot.h:24-31`).  Thus both the literal expression
types and the C conversion boundaries are operative source behavior.

The sealed candidate exposes every macro as `u32`, including values whose
source expression type is `int`; it consequently omits the required signed
surface and the conversion behavior at the mixed `int`/`unsigned int`
boundaries.  Correcting that requires a frozen, source-proven Rust UAPI
integer-expression and syscall-boundary mapping.  No such mapping or typed
bridge exists in the task's frozen ABI/porting records, and adding one here
would introduce an unreviewed design beyond this header's sealed candidate.
The candidate is therefore not modified and the task is BLOCKED.

Affected proposal records include the macro selection-expression records named
by R1 (both architecture rows); their `SOURCE_REVIEWED_VALUE` proposal cannot
be accepted as `COMPLETE` because it loses this expression-type contract.

### P1 (SPDX) — accepted; correction would invalidate the sealed candidate

The pinned UAPI header begins `SPDX-License-Identifier: GPL-2.0 WITH
Linux-syscall-note` (`include/uapi/linux/reboot.h:1`).  The candidate instead
uses `GPL-2.0-only`.  The reviewer correctly identified the dropped
`Linux-syscall-note` exception.  Updating the source header would change the
candidate after its snapshot and semantic proposal were sealed, requiring a
fresh implementation/review attempt.  Because the independent type-contract
blocker already prevents acceptance, no source edit is made in this attempt.

### P2 — accepted; include-guard translation mapping is not established

The frozen inventory selects the `#ifndef`, `#define _UAPI_LINUX_REBOOT_H`, and
`#endif` records for both architectures (`include/uapi/linux/reboot.h:2-3,40`).
The candidate and implementation evidence simply omit them.  Rust module
single-definition semantics may be a possible analogue, but the frozen records
contain no source-proven mapping that preserves the C preprocessor's
include-order/namespace contract.  Establishing that shared mapping is outside
this sealed file attempt, so this finding also remains a blocker rather than a
silent omission.

## Result

No source correction is applied.  Manual source evidence is insufficient to
close the selected macro-type and include-guard records without an approved
typed Rust UAPI bridge; the required Phase 1 result is `BLOCKED`.
