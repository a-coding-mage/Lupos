# Parity review — S016189 (slot 1)

Reviewed `vendor/linux/include/uapi/linux/input-event-codes.h` at pinned
revision `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/input-event-codes.rs`.

## Findings

### P1 — Every translated event-code macro has the wrong expression type

The upstream file defines 795 non-guard object-like macros with unsuffixed
integer literals and aliases/expressions of those literals.  Examples are
`#define EV_SYN 0x00` (upstream line 39), `#define KEY_MAX 0x2ff` (line 724),
and `#define KEY_CNT (KEY_MAX+1)` (line 725).  On both approved targets these
are C `int` constant expressions (with normal C integer promotions); their
type is not a 16-bit unsigned type merely because each currently defined value
fits in 16 bits.

The candidate declares all 795 counterparts as `u16`, including every base
constant, alias, and computed `*_CNT` expression (for example candidate lines
26, 728, and 729).  This changes the public Rust expression type, permitted
operators and coercions, signedness, promotion behavior, and overflow behavior
for all consumers.  It is observable by the in-scope input code: upstream
`drivers/input/input.c:54-64` initializes an `unsigned int` array from these
macros, and `drivers/input/evdev.c:51-71` uses `EV_CNT` and the `*_CNT` macros
as array bounds and `size_t` values.

Required resolution: map the source macros with their C `int` constant-
expression semantics (and retain explicit conversions only at the consuming
ABI boundary); do not globally narrow this UAPI macro namespace to `u16`.

### P1 — The candidate does not preserve the UAPI preprocessor interface

The upstream file is a C/UAPI preprocessor header guarded by
`_UAPI_INPUT_EVENT_CODES_H` (lines 16-17 and 1016).  Its file comment
explicitly states that it is included from both C and devicetree source and
therefore must contain only comments and defines.  `include/uapi/linux/input.h`
includes it directly at line 20, exposing these names to the Linux input UAPI.
The candidate replaces every `#define` with a Rust item and supplies no
preprocessor definition or include-guard equivalent.  Thus a C or DTS consumer
cannot include this mapped artifact or observe any of the required macro
definitions.

Required resolution: preserve/provide the original UAPI C/DTS macro surface as
part of the later Linux-compatible ABI integration.  If the frozen source-tree
mapping intentionally cannot contain that surface, the applier must establish
and record the exact separate ABI-preservation mechanism rather than treating
the Rust `pub const` namespace as equivalent.

## Verified portions

- The SPDX identifier, upstream path, pinned revision, `common` architecture
  scope, and task ID provenance are exact.
- Exhaustive identifier/expression comparison found all 795 non-guard
  definitions present: no missing or extra Rust constants, and no numeric,
  alias, or normalized-expression mismatch.  This includes all `*_MAX`,
  `*_CNT`, reserved values, and aliases.
- The upstream has no selected conditional branch beyond its include guard.

No source was edited.  This report records parity findings only.
