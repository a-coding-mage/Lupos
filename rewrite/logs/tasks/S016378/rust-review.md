# Rust review — S016378

Reviewed `src/include/uapi/linux/serial_reg.rs` against pinned
`vendor/linux/include/uapi/linux/serial_reg.h` as the independent Rust reviewer
(slot 2). No compiler, formatter, build, or test command was run.

## Finding R1 — `UART_FCR_R_TRIG_BITS` loses C macro argument typing

- **Severity:** must fix
- **Candidate:** `src/include/uapi/linux/serial_reg.rs:68`
- **Pinned source:** `vendor/linux/include/uapi/linux/serial_reg.h:101-102`

The source is a function-like macro:

```c
#define UART_FCR_R_TRIG_BITS(x) \
	(((x) & UART_FCR_TRIGGER_MASK) >> UART_FCR_R_TRIG_SHIFT)
```

It evaluates its argument once and applies C's integer promotions/usual
arithmetic conversions at each use.  The candidate fixes its public argument
and return type to `i32` with `pub const fn UART_FCR_R_TRIG_BITS(x: i32) ->
i32`.  This rejects the natural translation of the existing Linux call site:
`up->fcr` is `unsigned char` in
`vendor/linux/include/linux/serial_8250.h:132`, and the macro is used with it
in `vendor/linux/drivers/tty/serial/8250/8250_port.c:2978`.  C promotes that
argument to `int`; a Rust `u8` field cannot be passed to the candidate without
an extra caller-side conversion.  The function form also cannot preserve the
macro's behavior for other integral argument types (notably unsigned values),
whose C expression result is correspondingly unsigned.

The applier needs a macro- or otherwise call-site-type-preserving mapping that
keeps one evaluation and the C promotion/conversion semantics, rather than a
fixed `i32` callable API.

## Checks with no additional finding

- All 231 non-guard source macro identifiers are present once in the candidate;
  there are no extra Rust identifiers.
- The three OMAP base literals are correctly represented as `u32`: their
  unsuffixed hexadecimal C literals select `unsigned int` on both approved
  32-bit-`int` architectures.  Other literal and composed values fit the
  source `int` expressions represented as `i32`.
- The header has no configuration conditional beyond its include guard and no
  structs, exported functions, or FFI layout to preserve.  The candidate
  retains the source SPDX identifier, relevant copyright attribution, source
  path, pinned revision, common architecture scope, and task provenance.
