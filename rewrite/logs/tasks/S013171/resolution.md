# Resolution — S013171

Reviewed and resolved both independent reports against the complete pinned
`vendor/linux/include/dt-bindings/leds/common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## P1 / RUST-2 — C integer categories

Resolved.  Upstream lines 16–41 define 21 unsuffixed decimal literals.  Each
value fits the C `int` category on both frozen targets, so the Rust mirror now
uses `core::ffi::c_int` for all `LEDS_TRIG_TYPE_*`, `LEDS_BOOST_*`, and
`LED_COLOR_ID_*` constants.  It no longer assigns this macro namespace the
different unsigned `u32` category.  A consumer that needs a narrower field or
an unsigned ABI field must make its conversion at that consuming boundary, as
in C after the ordinary integer conversions.

## P1 / RUST-1 — C/DTS string literals, terminators, and concatenation

Resolved with the language-boundary split required by the source contract.
Every one of the 52 Rust `LED_FUNCTION_*` mirror constants is now a
fixed-length byte-string reference whose literal spelling has the upstream
bytes followed by exactly one `\0`; its array extent includes that terminator.
This preserves the C-string byte value and permits an explicit `.as_ptr()`
when a Rust consumer crosses a C ABI boundary.  It is deliberately not a Rust
`&str`, which has neither the terminator nor the required C representation.

The C/DTS preprocessor interface is not and cannot be provided by Rust items:
adjacent literal concatenation happens before C parsing.  The pinned original
header remains the authoritative C/DTS include at
`vendor/linux/include/dt-bindings/leds/common.h`; it is retained unchanged for
the original Linux driver objects and DTS preprocessing.  Thus usages such as
`":white:" LED_FUNCTION_KBD_BACKLIGHT` in
`drivers/hid/hid-apple.c:929` and `"upboard:yellow:" LED_FUNCTION_STATUS` in
`drivers/leds/leds-upboard.c:33` continue to expand through the exact upstream
macro replacement lists, including C's adjacent-literal concatenation and
NUL-terminated materialized string semantics.  A Rust constant is only the
path-preserving Rust-core mirror and is not substituted into that C/DTS build
path.

## Final per-task mapping

- The 21 numeric macro records are final as `c_int` constants with their
  verified values 0 through 15 (including the trigger and boost values).
- The 52 string macro records are final as fixed-size, NUL-terminated byte
  literals with their verified upstream spellings.
- The conventional C include guard has no Rust-module runtime equivalent; it
  remains intact in the retained authoritative C/DTS header.
- There are no functions, layouts, ownership transitions, locking, RCU,
  refcounts, allocations, feature branches, branding changes, or unsafe code
  in this task.

This closes the S013171 `PENDING_REVIEW` semantic records for both frozen
architectures.  Source-only review was performed; no compiler, formatter,
linker, test, emulator, debugger, or runtime command was run.
