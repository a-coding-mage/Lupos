# Rust review — S016105

## Scope and evidence

- Reviewed only `src/include/uapi/linux/dpll.rs` against pinned
  `vendor/linux/include/uapi/linux/dpll.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The checked-out branch reference is `refs/heads/feat/bun-like-rewrite-test`.
- Queue row S016105 was `REVIEWING`, owned by P02, at review start.  The frozen
  scope selects the header for both x86_64 and aarch64 through
  `net/core/rtnetlink.o`; its internal consumer header
  `include/linux/dpll.h:10` also exposes these enum tags in operation callback
  signatures (for example `:20-57`).
- This was a manual source/semantics review.  No compiler, formatter,
  test, rust-analyzer diagnostic, Git command, or prior Lupos Rust source was
  used.

## Verdict: REJECT — source changes required

### RUST-S016105-01 — `*_MAX` aliases make twelve Rust enums invalid and do not preserve the C alias mechanism

Severity: blocking

The C header intentionally makes each public `DPLL_*_MAX` enumerator an alias
of the preceding public value, while retaining the following private sentinel:
`DPLL_MODE_AUTOMATIC == DPLL_MODE_MAX == 2` and
`__DPLL_MODE_MAX == 3` at `dpll.h:20-27`.  The same mechanism occurs for
`dpll_lock_status` (`:43-52`), `dpll_lock_status_error` (`:72-81`),
`dpll_clock_quality_level` (`:91-104`), `dpll_type` (`:114-121`),
`dpll_pin_type` (`:133-142`), `dpll_pin_direction` (`:151-157`),
`dpll_pin_state` (`:173-181`), `dpll_pin_operstate` (`:194-203`), `dpll_a`
(`:232-250`), `dpll_a_pin` (`:252-288`), and `dpll_cmd` (`:290-306`).

The candidate instead declares each alias as another fieldless Rust enum
variant, e.g. `DPLL_MODE_AUTOMATIC` and `DPLL_MODE_MAX = 2` at
`dpll.rs:15-17`; equivalent duplicate discriminants are at `:24-28`, `:35-39`,
`:46-54`, `:63-66`, `:73-78`, `:85-87`, `:99-102`, `:110-114`, `:138-152`,
`:158-190`, and `:196-208`.  Rust fieldless enum discriminants must be unique.
Thus this is not a representable Rust enum definition, independently of any
build result.  It also changes the C mechanism: the public `*_MAX` names are
integer aliases, not distinct enum states.

Required resolution: model C values/sentinels and public aliases without
duplicate Rust enum variants (for example, ABI-reviewed integer/newtype storage
plus distinct top-level constants), preserving every public name and exact
value.

### RUST-S016105-02 — C global integer constants became namespaced, restricted Rust enum variants

Severity: blocking

Every enumerator in the UAPI header is a global C integer constant: for
example `DPLL_A_ID` through `DPLL_A_MAX` at `dpll.h:232-250`, pin-attribute
constants at `:252-288`, capability flags at `:212-216`, and command constants
at `:290-306`.  They are directly usable as integer constant expressions,
including bit/flag and Netlink attribute operations; the selected internal
header consumes the enum tags in C callback parameters at
`include/linux/dpll.h:20-57`.

The candidate exposes them only as associated variants of `pub enum` types
(`dpll.rs:13-18`, `:22-29`, `:119-123`, `:136-153`, `:156-191`, `:194-209`).
This changes each public identifier from `DPLL_*` to a required
`dpll_*::DPLL_*` path, changes its type from an integer constant to an enum
value, prevents direct C-equivalent arithmetic/bitwise use without casts, and
does not provide the C global constant namespace.  The non-alias capability
flags are affected too, even though their discriminants are distinct.

Further, a Rust enum can only hold listed discriminants safely, whereas C
enum-typed callback variables and decoded Netlink values are integer storage
and may carry an unrecognised value.  No checked conversion/error path is
present.  The candidate therefore cannot preserve the header's integer/FFI
surface or its handling of future/invalid UAPI values.  Use ABI-reviewed
integer-backed representations and top-level constants, with any typed wrapper
accepting the complete C representation domain; do not make a Rust enum the
only representation.

### RUST-S016105-03 — UAPI string macros lost C-string representation and terminator

Severity: blocking

`DPLL_FAMILY_NAME` and `DPLL_MCGRP_MONITOR` are C string-literal macros at
`dpll.h:10` and `:308`.  Their expansion has a trailing NUL and can decay to a
pointer to its first byte in C APIs.  The candidate substitutes Rust fat
`&str` constants without a NUL at `dpll.rs:7` and `:211`.  A Rust `&str` has
pointer-plus-length representation and cannot be supplied as the C string
macro's pointer without a separate conversion/allocation; neither preserve the
macro's type nor its terminator.

Required resolution: expose a NUL-terminated byte representation appropriate
to the required UAPI/FFI use, while preserving the exact public macro values;
do not substitute `&str` as the only definition.

### RUST-S016105-04 — UAPI SPDX license identifier was changed

Severity: blocking

The pinned UAPI source begins with
`SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`
at `dpll.h:1`.  Candidate `dpll.rs:1` instead claims `GPL-2.0-only`.
This is neither the source identifier nor an allowlisted branding difference,
and it removes the syscall-note/BSD licensing terms attached to this UAPI
surface.  Restore the upstream SPDX identifier and relevant generated-UAPI
provenance notices.

## Checks with no additional finding

- All literal numeric macro translations present at `dpll.rs:8`, `:57`,
  `:90-93`, and `:125-126` match their respective small, positive C integer
  values at `dpll.h:11`, `:106`, `:160-163`, and `:218-219`; this does not cure
  the enum/API defects above.
- The candidate contains no `unsafe`, raw-pointer operation, FFI declaration,
  allocation, `Drop`, callback implementation, panic/placeholder,
  `#[test]`, or `#[cfg(test)]` construct.  Accordingly there is no separate
  borrow, provenance, pinning, Send/Sync, interior-mutability, or unsafe-block
  finding in this header-only candidate.
- No unauthorized Lupos branding was observed beyond the SPDX defect above.
