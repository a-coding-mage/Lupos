# Parity review — S013171 (slot 1)

Reviewed pinned `vendor/linux/include/dt-bindings/leds/common.h` at Linux
revision `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/dt-bindings/leds/common.rs` for the frozen common x86_64/AArch64
scope.

## Findings

### P1 — The numeric macro expressions were narrowed and made unsigned

Every numeric definition in the pinned header is an unsuffixed decimal
integer literal.  Thus `LEDS_TRIG_TYPE_*`, `LEDS_BOOST_*`, and all
`LED_COLOR_ID_*` definitions (upstream lines 16–41) are C `int` constant
expressions on both frozen targets.  The candidate declares all 21 of them as
`u32` (candidate lines 13–34).

This changes each public expression's signedness, integer-promotion behavior,
permitted operator domain, and overflow semantics.  The source uses these
macros in contexts requiring the ordinary C `int` expression behavior:
`drivers/leds/led-core.c:29–45` uses `LED_COLOR_ID_*` as array bounds and
designated indices, while `drivers/leds/led-core.c:502` compares an integer
property against `LED_COLOR_ID_MAX`; the same source header is also used for
`int` and `u8` array bounds in
`include/linux/platform_data/leds-lp55xx.h:33–34` and
`drivers/leds/rgb/leds-lp5812.h:146–150`.

Required resolution: retain the C `int` constant-expression semantics (for
example, use the C `int` ABI scalar for the ordinary unsuffixed literals) and
perform explicit conversion only at a consuming ABI boundary.  Do not assign
this macro namespace a blanket `u32` type.

### P1 — Replacing the DT-binding macros with Rust `&str` items loses their C/DTS interface

Each `LED_FUNCTION_*` definition in the source is an object-like C macro whose
replacement list is a string literal (upstream lines 46–112).  It is therefore
usable both as a C string literal—with its trailing NUL when materialized—and
in preprocessor string-literal concatenation.  Pinned consumers rely on that
contract, for example `drivers/hid/hid-apple.c:929` expands
`":white:" LED_FUNCTION_KBD_BACKLIGHT`, and
`drivers/leds/leds-upboard.c:33–42` expands name prefixes immediately followed
by `LED_FUNCTION_STATUS`.

The candidate makes each name a Rust `&str` (candidate lines 37–88).  A Rust
`&str` is a non-NUL-terminated fat reference, cannot take part in the required
compile-time literal concatenation, and supplies no C/DTS preprocessor macro
surface.  No separate Linux-compatible C/DTS binding or integration mechanism
is present in the candidate or task evidence.  This is an ABI/interface loss,
not merely a Rust spelling change.

Required resolution: preserve the complete DT-binding macro interface,
including literal-concatenation and NUL-terminated C-string behavior, through
an explicitly recorded compatible mechanism; a Rust `pub const &str` is not
equivalent.

## Verified portions

- Exhaustive normalized comparison found all 73 non-guard object-like macro
  names and replacement values represented, with no missing, extra, or
  value mismatch: 21 numeric definitions and 52 function-name string
  definitions.
- The source has no Kconfig, architecture, or feature conditionals.  Its sole
  conditional is the conventional `__DT_BINDINGS_LEDS_H` include guard
  (upstream lines 12–13 and 114).  A Rust module needs no runtime equivalent,
  but the required C/DTS include-guard and macro surface remain unpreserved as
  described above.
- SPDX identifier `(GPL-2.0 OR BSD-2-Clause)`, source path, pinned revision,
  `common` architecture scope, task ID, and all four upstream copyright/author
  notices are retained exactly.  There is no branding delta, Rust test
  configuration, placeholder, type/layout declaration, function, storage
  definition, or synchronization behavior in the source header.

No source, manifest, or queue file was edited.  No compilation, formatting,
testing, linking, runtime, or debugger command was run.
