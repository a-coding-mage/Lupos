# Rust review — S016189

**Result: REJECT — one must-fix finding.**

## R1 — every C integer macro was narrowed to `u16`

`src/include/uapi/linux/input-event-codes.rs:25-1012` declares each of the 795
translated definitions as `u16`.  This is not the type semantics of the
upstream macros.  The corresponding definitions in
`vendor/linux/include/uapi/linux/input-event-codes.h:23-1014` are unsuffixed
integer constants, aliases of those constants, or `+ 1` integer expressions;
on both selected Linux C targets their expression type is signed `int` before
the context applies any conversion.  In particular,
`KEY_MAX`/`KEY_CNT` are `0x2ff`/`(KEY_MAX + 1)` at upstream lines 837-838, not
16-bit objects.

The UAPI demonstrates that a 16-bit conversion is a boundary operation, not a
property of every code macro: `struct input_event` stores `type` and `code` as
`__u16` at `include/uapi/linux/input.h:43-44`.  The same macros are separately
used as integer constant expressions for counts and indexes, e.g.
`INPUT_PROP_CNT` through `SW_CNT` in `include/linux/input.h:143-153` and
`REP_CNT` in `include/linux/input.h:174`.  Reifying all of them as `u16`
changes the arithmetic/promotion domain, prevents their direct equivalent use
as Rust array lengths or general integer expressions, and makes the eventual
wire-field conversion implicit in the definition instead of explicit at the
`__u16` boundary.

Required resolution: model the macro definitions with their C integer
expression semantics (the unsuffixed literals and aliases are `i32` for the
frozen x86_64/aarch64 C ABI) and make any `u16` or `usize` conversion at the
specific translated use site, with a checked/otherwise Linux-equivalent
invariant where needed.  Do not retain a blanket `u16` type merely because the
current numeric values fit in it.

## Checked items with no finding

- The 795 non-guard macro names are all present once; the numeric RHS values,
  15 aliases, and 11 `*_CNT` expressions match the pinned header exactly.
- The source has no feature/configuration conditional around these definitions
  other than its include guard (`input-event-codes.h:16,1016`); no Rust `cfg`
  gate was required.
- Provenance identifies the correct source, revision, architectures, task, and
  preserves `GPL-2.0-only WITH Linux-syscall-note`.

No build, formatting, test, or execution command was run.
