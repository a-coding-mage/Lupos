# S016378 Rust review — attempt 2, slot 2

## Result

FAIL — source correction and another independent Rust review are required.

## Review basis

- Task row: `S016378`, `REVIEWING`, attempt `2`, pipeline `P02`; scope maps
  `include/uapi/linux/serial_reg.h` to
  `src/include/uapi/linux/serial_reg.rs` for `common`.
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`; the candidate
  provenance names that same revision and source file.
- The pinned header has only its include guard (`serial_reg.h:15-16,386`), so
  every defined UART macro is selected for both approved architectures.

## Finding RUST-S016378-01 — function-like macro loses C integer promotion

Severity: high.

`UART_FCR_R_TRIG_BITS(x)` in the pinned header expands to
`(((x) & UART_FCR_TRIGGER_MASK) >> UART_FCR_R_TRIG_SHIFT)`
(`vendor/linux/include/uapi/linux/serial_reg.h:100-102`).  The unsuffixed
mask and shift-count literals are C `int`; therefore an `unsigned char` input
is integer-promoted before the bitwise operation.  This is not merely a value
constant: the macro accepts an expression and preserves that C promotion
mechanism while evaluating its argument once.

The candidate fixes both operands to `i32`
(`src/include/uapi/linux/serial_reg.rs:61,75,77-80`).  Its expansion requires
the supplied expression to implement bitwise-and directly with `i32`; Rust
does not perform C's implicit integer promotion.  The pinned consumer proves
the difference is operative: `serial8250_config.fcr` is `unsigned char`
(`vendor/linux/drivers/tty/serial/8250/8250.h:67-73`) and is passed directly
to this macro in `8250_port.c:2973-2980`.  A faithful Rust representation of
that field is byte-sized, for which the candidate macro cannot supply the
header's promotion behavior.  The same fixed `i32` mask also changes the
compound-mask use that C performs on byte register state (for example,
`up->fcr &= ~UART_FCR_TRIGGER_MASK` in
`vendor/linux/drivers/tty/serial/8250/8250_port.c:2661-2662`).

Required resolution: retain the pinned macro's byte-input / C-promotion
semantics at the header boundary, including single evaluation and the exact
masked-right-shift result, rather than requiring every consumer to adopt an
`i32` operand or to move conversions into unrelated call sites.  Recheck the
public macro's namespace after doing so.

## Checks without findings

- Manual macro-name inventory found every pinned non-guard macro represented
  once in the candidate; their spelled numeric values and derived expressions
  match the header where they are evaluated as the candidate's declared types.
- `OMAP1_UART{1,2,3}_BASE` are correctly represented as `u32` for the pinned
  hexadecimal values (`serial_reg.h:351-353`), which do not fit signed `i32`.
- Candidate provenance, SPDX identifier, and `common` architecture annotation
  match the task and pin.  All scalar definitions are public; no unallowlisted
  branding was found.
- No `unsafe`, raw-pointer/FFI layout, interior-mutability, `Drop`, callback,
  allocation, panic, placeholder, or Rust-test construct appears in this
  scalar/macro-only file.  Consequently there is no ownership, aliasing,
  pinning, Send/Sync, or ABI-layout boundary to approve here.

No compiler, formatter, test, linker, rust-analyzer diagnostic, or historical
Lupos Rust source was used for this review.
