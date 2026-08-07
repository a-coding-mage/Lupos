# S016105 slot-1 parity review — `include/uapi/linux/dpll.h`

Reviewer: parity reviewer (independent source inspection)  
Scope: pinned `vendor/linux/include/uapi/linux/dpll.h` versus
`src/include/uapi/linux/dpll.rs` only, with frozen task metadata and local
callers needed to establish UAPI use. No compiler, formatter, linker, test,
rust-analyzer diagnostic, or Git command was run or used as evidence.

## Preconditions

- The checked-out worktree branch reference is
  `refs/heads/feat/bun-like-rewrite-test`.
- `vendor/linux.SHA` is `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching
  the candidate provenance at `src/include/uapi/linux/dpll.rs:3`.
- The frozen queue row is S016105, `REVIEWING`, P02, destination
  `src/include/uapi/linux/dpll.rs`; `rewrite/SCOPE.tsv` classifies the source
  as common `RUST_TRANSLATE`.

## Findings

### P1 — eleven C `*_MAX` aliases were encoded as duplicate Rust enum discriminants

Linux symbols: `DPLL_MODE_MAX`, `DPLL_LOCK_STATUS_MAX`,
`DPLL_LOCK_STATUS_ERROR_MAX`, `DPLL_CLOCK_QUALITY_LEVEL_MAX`,
`DPLL_TYPE_MAX`, `DPLL_PIN_TYPE_MAX`, `DPLL_PIN_DIRECTION_MAX`,
`DPLL_PIN_STATE_MAX`, `DPLL_PIN_OPERSTATE_MAX`, `DPLL_A_MAX`,
`DPLL_A_PIN_MAX`, and `DPLL_CMD_MAX`.

Local evidence: each Linux enum deliberately makes its public max constant an
alias of the preceding ordinary value: `vendor/linux/include/uapi/linux/dpll.h`
lines 20–27, 43–52, 91–104, 114–122, 133–143, 151–158, 173–181, 194–203,
232–250, 252–288, and 290–306. The candidate represents each pair as two
variants with the same explicit `#[repr(i32)]` discriminant; e.g.
`dpll_mode::__DPLL_MODE_MAX` is implicitly 3 while
`dpll_mode::DPLL_MODE_MAX = 2` duplicates `DPLL_MODE_AUTOMATIC = 2` at
`src/include/uapi/linux/dpll.rs:13–18`. The equivalent duplicate form occurs
at lines 22–29, 33–40, 44–55, 61–67, 71–79, 83–88, 97–103, 108–115, 136–153,
156–191, and 194–209.

Rust C-like enum variants must have distinct discriminants; this is a manual
Rust language/source-semantics observation, not compiler output. Consequently
these declarations do not provide a valid representation of Linux's public
integer aliases. Preserve the enum constants as distinct public integer
constants (or otherwise represent the sentinel and alias without duplicate
variants) with the exact C values.

### P1 — public C enum constants were changed into scoped, closed Rust tagged types

Linux symbols: every enumerator in `enum dpll_mode`, `dpll_lock_status`,
`dpll_lock_status_error`, `dpll_clock_quality_level`, `dpll_type`,
`dpll_pin_type`, `dpll_pin_direction`, `dpll_pin_state`,
`dpll_pin_operstate`, `dpll_pin_capabilities`, `dpll_feature_state`,
`dpll_a`, `dpll_a_pin`, and `dpll_cmd`.

Local evidence: the pinned header declares ordinary C enum constants, which
are integer constant expressions in the enclosing C identifier namespace
(`vendor/linux/include/uapi/linux/dpll.h:20–306`). The candidate instead
places all of them behind distinct Rust enum type paths
(`src/include/uapi/linux/dpll.rs:13–209`). This changes both name resolution
(for example, Linux `DPLL_A_ID` versus Rust `dpll_a::DPLL_A_ID`) and
integer-expression use. It also makes each set a closed Rust enum rather than
the C integer-domain representation expected for netlink values.

This is operative, not merely stylistic: local consumers use these bare
constants as array bounds, indices, bit counts, netlink attribute/command
integers, and family fields; see `vendor/linux/drivers/dpll/dpll_nl.c:15–167`
and `vendor/linux/drivers/dpll/dpll_netlink.c:120–141,1327–1343`. The
candidate has no same-named module-level public integer constants. Provide
the public constants with C-equivalent integer semantics and preserve the
needed enum/tag representation separately if it is genuinely required by the
frozen ABI guidance.

### P1 — `dpll_pin_capabilities` no longer represents legal flag combinations

Linux symbols: `DPLL_PIN_CAPABILITIES_DIRECTION_CAN_CHANGE`,
`DPLL_PIN_CAPABILITIES_PRIORITY_CAN_CHANGE`, and
`DPLL_PIN_CAPABILITIES_STATE_CAN_CHANGE`.

Local evidence: Linux declares the values 1, 2, and 4 as integer flags at
`vendor/linux/include/uapi/linux/dpll.h:205–216`. A selected local DPLL driver
forms a combined value with `PRIORITY_CAN_CHANGE | STATE_CAN_CHANGE` at
`vendor/linux/drivers/dpll/zl3073x/prop.c:208–210`; DPLL netlink tests the
individual masks against the stored combined field at
`vendor/linux/drivers/dpll/dpll_netlink.c:1364–1460`. The candidate defines a
closed `#[repr(i32)] enum dpll_pin_capabilities` with only the three singleton
variants (`src/include/uapi/linux/dpll.rs:117–123`), so it contains no value
for legal combinations such as 6 and removes the integer bitwise-mask
contract. Represent these as integer flag constants/a mask-compatible type
with the same C values and operations.

### P1 — string macros lost C string-literal representation and terminating NUL

Linux symbols: `DPLL_FAMILY_NAME` and `DPLL_MCGRP_MONITOR`.

Local evidence: the source supplies C string-literal macros at
`vendor/linux/include/uapi/linux/dpll.h:10` and `:308`; a C string literal
includes a trailing NUL and decays to a `char *`-compatible pointer in such
uses. `DPLL_FAMILY_NAME` is consumed as the `.name` family field in
`vendor/linux/drivers/dpll/dpll_nl.c:166–167`. The candidate changes both to
Rust `&str` (`src/include/uapi/linux/dpll.rs:7,211`), a UTF-8 slice with fat
pointer representation and no terminal NUL. This is neither the C macro's
source-level type nor an FFI-compatible C string. Preserve the byte/NUL and
pointer-compatible UAPI contract at the boundary.

### P2 — the required UAPI SPDX identifier was replaced with a different license

Linux symbol/file provenance: `include/uapi/linux/dpll.h`.

Local evidence: the pinned header begins with
`SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`
at `vendor/linux/include/uapi/linux/dpll.h:1`. The candidate replaces it with
`GPL-2.0-only` at `src/include/uapi/linux/dpll.rs:1`. This loses both the
Linux-syscall-note exception and the BSD-3-Clause alternative; neither is an
allowlisted branding difference. Retain the upstream UAPI SPDX expression in
the translated file's provenance/license notice.

### P2 — public protocol documentation and generated-source provenance were materially removed

Linux symbols: the public contracts for `dpll_mode`, `dpll_lock_status`,
`dpll_lock_status_error`, `dpll_type`, `dpll_pin_type`,
`dpll_pin_direction`, `dpll_pin_state`, `dpll_pin_operstate`,
`dpll_pin_capabilities`, and `dpll_feature_state`.

Local evidence: `vendor/linux/include/uapi/linux/dpll.h:2–5` records that this
is YNL-generated from `Documentation/netlink/specs/dpll.yaml` and gives its
regeneration command. Lines 13–230 specify per-enumerator netlink meanings,
including lock-status transitions and feature semantics. Candidate lines
10–133 reduce these to short summaries and omit the per-enumerator contracts,
and candidate lines 1–5 omit the generator provenance. For a generated UAPI
definition these are operative maintenance/protocol evidence; preserve the
source provenance and complete public contract comments unless a frozen
manifest explicitly permits their removal (none was found in the task-local
scope/ABI evidence inspected).

## Coverage notes

- Textual inventory coverage is otherwise complete: all 14 Linux enum tags,
  all named enumerators, and all eleven numeric macros appear in the candidate
  in source order, and the written numeric values match the pinned header.
- The header has no conditional compilation branches. No candidate `todo!`,
  `unimplemented!`, Rust test configuration, test function, panic shell, or
  unauthorized Lupos branding was observed.
- No functions, allocation, locking, RCU, refcount, error, cleanup, or runtime
  state-machine paths exist in this UAPI-only header. The findings above are
  therefore limited to source/UAPI representation, ABI, flag, licensing, and
  public-contract parity.

## Verdict

FAIL — source parity is not established. The P1 UAPI representation defects
must be resolved before this task can be accepted.

