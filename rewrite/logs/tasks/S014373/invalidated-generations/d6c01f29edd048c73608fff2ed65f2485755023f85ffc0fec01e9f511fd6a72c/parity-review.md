# Parity review — S014373

Scope reviewed: `vendor/linux/include/linux/migrate_mode.h` against
`src/include/linux/migrate_mode.rs`, with direct consumer declarations in
`include/linux/migrate.h`, `include/linux/fs.h`, `mm/internal.h`,
`mm/migrate.c`, `mm/debug.c`, and `include/trace/events/migrate.h`.

## Result: changes required

### P1 — Closed Rust enums do not preserve the C enum object contract

`src/include/linux/migrate_mode.rs:13-17` and `:25-37` use fieldless
`#[repr(C)]` Rust enums.  They preserve the enumerator spelling and the
implicit values for the declared variants, but they make the representable
Rust values a closed set.  The C declarations at
`include/linux/migrate_mode.h:11-15` and `:17-29` are integer-compatible C
enum object types; no source-level validity restriction limits an object to
the listed enumerators.

This is material in the selected consumers.  `enum migrate_mode` is stored in
`struct compact_control` (`mm/internal.h:1041`), passed through exported and
callback signatures (`include/linux/migrate.h:47,58,60` and
`include/linux/fs.h:429-430`), and recorded in trace event fields
(`include/trace/events/migrate.h:54,66,94,99`).  `enum migrate_reason` is a
field of `struct migration_target_control` (`mm/internal.h:1520`).  Moreover,
the public `migrate_pages` interface accepts `reason` as `int`
(`include/linux/migrate.h:59-61`; definition `mm/migrate.c:2090-2093`), while
internal calls pass that integer to `migrate_folio_move`'s `enum
migrate_reason` parameter (`mm/migrate.c:1354-1357,1739-1741`).  A Rust enum
value constructed/read from an unrecognised C integer is therefore not a
faithful representation of the C object/FFI contract.

Use a C-`int`-ABI transparent integer newtype (with the named constants at
the exact values) or an equally explicit representation that admits every
underlying C integer value.  Preserve field and function argument ABI for the
above consumers; do not rely on a closed Rust-enum validity invariant.

### P2 — The mode's operative blocking invariants were dropped from comments

The candidate's generic doc comment at `src/include/linux/migrate_mode.rs:8-10`
does not retain the upstream constraints in
`include/linux/migrate_mode.h:4-10`: `MIGRATE_ASYNC` never blocks;
`MIGRATE_SYNC_LIGHT` may block for most operations but not `->writepage` due
to potentially significant stall time; and `MIGRATE_SYNC` blocks while
migrating pages.  These are operative caller/callback constraints (for
example `include/linux/fs.h:426-430` states the async callback must not
block).  Retain these invariants verbatim or equivalently in the translated
source documentation.

### P3 — SPDX identifier differs from the pinned source

The pinned header begins `/* SPDX-License-Identifier: GPL-2.0 */`
(`include/linux/migrate_mode.h:1`), whereas the candidate begins
`// SPDX-License-Identifier: GPL-2.0-only` (`src/include/linux/migrate_mode.rs:1`).
The rewrite rules require retaining the upstream SPDX identifier; restore the
exact `GPL-2.0` identifier unless a specific branding/license allowlist
authorizes this change.

## Confirmed items

- Both enums are unconditional for the frozen common x86_64/aarch64 scope;
  no architecture or Kconfig branch is missing.
- Declaration order and implicit values are otherwise correct:
  `MIGRATE_ASYNC=0`, `MIGRATE_SYNC_LIGHT=1`, `MIGRATE_SYNC=2`; and
  `MR_COMPACTION=0` through `MR_DAMON=9`, with `MR_TYPES=10` as the terminal
  array-bound sentinel.  `MR_TYPES` is consumed as the
  `migrate_reason_names` array bound in `include/linux/migrate.h:52` and
  `mm/debug.c:30-32`.
- The immutable provenance source path, revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, common architecture tag, and
  task ID match the queue and `vendor/linux.SHA`.
