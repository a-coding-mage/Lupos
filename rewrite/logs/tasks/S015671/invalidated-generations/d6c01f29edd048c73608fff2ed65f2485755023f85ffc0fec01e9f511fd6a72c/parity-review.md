# Parity Review — S015671 (slot 1)

Result: **PASS — no actionable parity findings.**

Reviewed `src/include/net/tls_prot.rs` against pinned
`vendor/linux/include/net/tls_prot.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, together with the S015671 scope,
symbol, ABI, lifetime, file-map, task-dependency, and selected-consumer
records.

## Exhaustive source mapping

- The anonymous C enum at upstream lines 16–24 is represented by the seven
  identically named public `i32` constants at candidate lines 21–27, with the
  exact values 20 through 26.
- The anonymous C enum at upstream lines 29–32 is represented by the two
  identically named public `i32` constants at candidate lines 32–33, with
  exact values 1 and 2.
- The anonymous C enum at upstream lines 37–66 is represented by the 28
  identically named public `i32` constants at candidate lines 38–65.  Each
  explicit sparse value, including 0, 10, 20, 22, 40, 42–52, 70, 71, 80, 86,
  90, 109, 110, 112, 113, 115, 116, and 120, matches exactly.
- Each C enumerator is an `int` constant expression; `i32` is the matching
  32-bit `int` value domain for both frozen x86_64 and AArch64 targets.  The
  source declares no named enum tag, enum object, storage, function, linkage,
  layout-bearing structure, or FFI surface to preserve beyond those values.
- The only source conditional/macro is the C include guard (upstream lines
  10–68).  It has no selected configuration branch or run-time semantics;
  Rust module inclusion supplies the corresponding single-definition boundary.
  No other macro or configuration condition is omitted.

## Context and non-source checks

The selected consumers use these constants as values (not a named enum ABI),
including `net/handshake/alert.c:36`,
`net/handshake/tlshd.c:454-455`,
`net/sunrpc/svcsock.c:251-260`, and
`net/sunrpc/xprtsock.c:369-378`; all candidate values preserve those uses.
All values fit the `u8` conversions present in the pinned consumers.

The candidate starts with the exact source path, pinned revision, common
architecture membership, and task provenance; preserves the upstream SPDX
expression and Oracle copyright notice; introduces no branding delta, tests,
placeholder, unsafe code, ABI/linkage declaration, or additional behavior.

The Phase 0 records retain `PENDING_REVIEW` for the three anonymous enums on
both architectures.  This report establishes their source facts: they are
anonymous, value-only `int` enumerator declarations with no object lifetime,
layout, linkage, or separate ABI.  The applier must record the required final
manifest closure before `DONE`.
