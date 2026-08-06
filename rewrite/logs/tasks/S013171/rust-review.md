# Rust review — S013171

Reviewed `src/include/dt-bindings/leds/common.rs` against the complete pinned
`include/dt-bindings/leds/common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`. This is an independent Rust
semantic review only; no source file was modified and no build/test was run.

## Result: reject pending applier resolution

### RUST-1 — blocking: `&str` is not a C string-literal macro

All 52 `LED_FUNCTION_*` definitions are C string-literal replacement lists
(for example `LED_FUNCTION_STATUS` at source line 64), but the candidate turns
them into `pub const ...: &str` (candidate lines 35 onward). A C string literal
is an array expression, includes a trailing NUL, and participates in
preprocessor adjacent-literal concatenation before C parsing. A Rust `&str` is
a UTF-8 slice/fat pointer with no trailing NUL and cannot be used in a
compile-time concatenation expression.

This is operative, rather than documentary, macro behavior: the pinned users
include `"upboard:yellow:" LED_FUNCTION_STATUS` in
`drivers/leds/leds-upboard.c:33` and `":white:" LED_FUNCTION_KBD_BACKLIGHT`
in `drivers/hid/hid-apple.c:929`. The latter consumers remain original C driver
objects, so their C-preprocessor contract also cannot be supplied by these Rust
constants. The applier must establish and record the binding/driver boundary
and provide a representation/mechanism that preserves each required consumer's
literal, NUL, and concatenation semantics; changing all definitions to `&str`
is not parity.

### RUST-2 — blocking: numeric macro type semantics were assumed, not derived

The 21 numeric replacement lists are unsuffixed C integer literals, hence have
the C `int` category before the surrounding expression supplies the usual
conversions. The candidate fixes every one to `u32` (candidate lines 13–33).
That changes the expression's type and available operations in every Rust
consumer. The source uses these values as array bounds/subscripts and in
comparisons, e.g. `LED_COLOR_ID_MAX` bounds `led_colors` at
`drivers/leds/led-core.c:29` and `intensity_value` at
`drivers/leds/led-class-multicolor.c:76`; it also assigns them to fields of
different widths.

`u32` may be appropriate for an explicitly identified device-tree property,
but neither the candidate nor the task ABI/lifetime rows contains evidence that
it is the single exact replacement for the C macro in every selected context.
The applier must derive and document the Rust-facing macro/constant contract
(including constant-expression, array-index, comparison, and field-assignment
uses) rather than relying on the implementation note's unsupported `u32`
assumption.

## Checks with no finding

- The candidate has the exact required provenance: source path, pinned revision,
  `common` architectures, and task `S013171`; it retains the SPDX and upstream
  copyright notices.
- The complete header has 73 non-guard public macros (21 numeric and 52 string
  literal), and the candidate has exactly 73 same-named public constants with
  the same spelled literal values. No identifier omission or value drift found.
- The only source conditional is the include guard (`#ifndef` at line 12);
  there are no configuration-dependent branches. Rust module loading has no
  direct runtime equivalent, so its absence is not independently a finding.
- No unsafe code, FFI declaration, layout declaration, Rust tests, or forbidden
  placeholders occur in the candidate.
