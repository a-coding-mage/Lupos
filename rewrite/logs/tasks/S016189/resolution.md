# Resolution — S016189

## P1 / R1 — C integer expression type

Resolved. The 795 mapped object-like macros now use `i32`, which is the
signed C `int` constant-expression domain for the pinned x86_64 and AArch64
sources. The correction was a mechanical replacement of the Rust type
annotation only: every literal spelling, alias target, and `*_CNT` expression
remains sourced from `include/uapi/linux/input-event-codes.h` without a manual
value change. A source-to-Rust ordered comparison after the correction matched
all 795 `name -> normalized RHS` definitions.

The `__u16` fields in `include/uapi/linux/input.h` are separate ABI-boundary
conversions; this macro-only mapping does not narrow the macro namespace.

## P1 — C/DTS UAPI macro surface and include guard

Resolved for the mapped source boundary. The C/DTS public macro artifact is
the unchanged pinned `vendor/linux/include/uapi/linux/input-event-codes.h`,
whose `_UAPI_INPUT_EVENT_CODES_H` include guard and all 795 macro definitions
remain the source used by C/DTS consumers and later original-driver/UAPI ABI
integration. `src/include/uapi/linux/input-event-codes.rs` is the Rust mapping
of that artifact, not a replacement C preprocessor header; its immutable
provenance records both the C UAPI header and guard. The Rust mapping exposes
the same 795 identifiers as `i32` constants, including all aliases and
derived expressions. No Rust configuration gate is added because the pinned
header has no selected conditional branch beyond the C include guard.

No compiler, formatter, linker, test, runtime, or benchmark command was run.
