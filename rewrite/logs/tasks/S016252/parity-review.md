# S016252 parity review — attempt 2, slot 1

Status: FINDINGS

Reviewed only the pinned `include/uapi/linux/mptcp_pm.h`, its frozen task
records, the current Rust candidate, candidate diff, and direct pinned MPTCP
callers. No compiler, formatter, linker, test, runtime, historical source, or
other task evidence was used.

## Findings

### F001 — named UAPI enum types are absent

Proposal keys: `named_enum_types`

Linux symbols: `enum mptcp_event_type`; `enum mptcp_event_attr`.

Local evidence: the pinned header declares the named enum types at lines 44
and 110, and the frozen `SYMBOLS.tsv` selects both as `type` for x86_64 and
AArch64 (with matching ABI records). `vendor/linux/net/mptcp/protocol.h:1161`
declares `mptcp_event(enum mptcp_event_type type, ...)`, while
`vendor/linux/net/mptcp/pm_netlink.c:523` and `:572` define functions taking
that type. The candidate exports only independent `i32` constants and no
public representation of either named enum. Consequently it cannot preserve
the selected named type surface or its use in the direct caller/callee
declarations. Add ABI-accurate public representations for both named enum
types while retaining the visible enumerator values and their gaps.

### F002 — `MPTCP_PM_NAME` loses C string-literal / C-pointer semantics

Proposal keys: `c_string_macro`

Linux symbol: `MPTCP_PM_NAME`.

Local evidence: pinned `mptcp_pm.h:10` defines the macro as the C string
literal `"mptcp_pm"`, which includes a terminating NUL and decays to a
`const char *` in the direct use at `vendor/linux/net/mptcp/pm_netlink.c:631`
(`.name = MPTCP_PM_NAME`). The candidate instead publishes `&str`; it is a
Rust slice (pointer plus length), has no terminal NUL in its value, and is not
the C pointer expression used by the Linux initializer. Preserve a NUL-
terminated, C-compatible static byte/string representation for this UAPI
macro, with any Rust string convenience view kept separate from the ABI-facing
symbol.

### F003 — upstream SPDX identifier was changed

Proposal keys: `spdx_identifier`

Linux symbol: file-level SPDX identifier for `include/uapi/linux/mptcp_pm.h`.

Local evidence: pinned line 1 is
`SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`;
the candidate line 1 is `SPDX-License-Identifier: GPL-2.0-only`. This changes
the selected UAPI header's retained upstream SPDX identifier, contrary to the
required source provenance contract. Restore the exact upstream identifier.

## Checked without a finding

All selected enumerator names and explicit/implicit values are present as
`i32` constants in the candidate: event values retain the 3→6, 7→10,
11→13, and 13→15 gaps; each anonymous enum sentinel has its recorded value;
and all six `*_MAX` macros retain the upstream sentinel-minus-one expression.
The header has no configuration branches beyond its include guard, so there
is no configuration-specific value or visibility branch to translate.
