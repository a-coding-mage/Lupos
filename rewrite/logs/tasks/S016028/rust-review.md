# S016028 Rust review (slot 2)

## Verdict

**Finding required before acceptance.** The candidate has no ownership,
pointer, layout, unsafe, panic, test, or configuration defect, but it changes
the public integer type of most source macros.

## Finding RUST-001 — all macros were made `u32`, losing the C integer
category

`include/uapi/asm-generic/termbits-common.h:9-64` defines its constants as
unsuffixed C integer constant expressions.  On both frozen targets,
`0x001` through `0x40000000`, the baud values, `IBSHIFT`, and all `TCO*` and
`TC*FLUSH` selectors have C type `int`; `EXTA` and `EXTB` expand to the
`int`-typed baud constants.  Only `CRTSCTS` (`0x80000000`) selects `unsigned
int` under the C literal rules.  The candidate instead exposes every one as
`pub const ...: u32`.

This is observable at the Rust public interface: for example, C accepts
`TCOOFF` and `TCIFLUSH` as `int` action/queue-selector values, while Rust
callers expecting `i32` cannot use the candidate constants without an explicit
cast.  The same forced unsigned type also changes trait selection, signed
conversion, and expression typing for all of the positive unsuffixed literals.
The values and bit patterns alone do not establish type parity.  The ABI
record for this task remains `PENDING_REVIEW`, so it cannot justify the blanket
`u32` choice.

The applier must resolve the source-compatible Rust representation of these
macro categories from the pinned UAPI usage and record the decision.  In
particular, flag constants consumed through `tcflag_t` (whose definition in
`include/uapi/asm-generic/termbits.h:6` is `unsigned int`) must be considered
separately from the action, flush, and shift-count macros; a single `u32`
annotation for all 45 macros is not established by the source.

## Checks that passed

- `cc_t = u8` exactly represents `unsigned char`, and `speed_t = u32` exactly
  represents the frozen targets' `unsigned int`; neither alias needs a Rust
  layout attribute.
- All 45 value/alias macros from the pinned header are present under their
  original names. `EXTA` and `EXTB` retain the `B19200`/`B38400` aliases, and
  the `CRTSCTS` bit pattern is representable without runtime overflow as
  `u32`.
- The common task is correctly unconditional for both frozen architectures;
  the C include guard has no Rust source-level equivalent. No `cfg` branch is
  missing.
- Provenance matches `vendor/linux.SHA`
  (`425f94c2954b1fe80ebdbf9b29854e89750355df`) and the source SPDX expression.
- The file contains no FFI declarations, aggregate layout, unsafe code,
  allocation, panic path, placeholder, or Rust test configuration.

No build, test, formatter, compiler, linker, or runtime command was run.
