# S016378 parity review — slot 1

Reviewed `src/include/uapi/linux/serial_reg.rs` against pinned
`vendor/linux/include/uapi/linux/serial_reg.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding P1 — function-like macro contract narrowed

`UART_FCR_R_TRIG_BITS(x)` is a C function-like macro at upstream lines 101–102:
`(((x) & UART_FCR_TRIGGER_MASK) >> UART_FCR_R_TRIG_SHIFT)`.  The candidate at
Rust lines 68–70 changes that interface to `pub const fn
UART_FCR_R_TRIG_BITS(x: i32) -> i32`.

The macro accepts an integral expression and applies the C integral promotions
and usual arithmetic conversions at each call site; the Rust function accepts
only `i32`.  This is not theoretical in the pinned tree: the selected source
consumer `drivers/tty/serial/8250/8250_port.c:2978` passes `up->fcr`, whose
declaration is `unsigned char fcr` at `drivers/tty/serial/8250/8250.h:71`.
That C value promotes to `int` before the mask and shift, while a corresponding
Rust `u8` caller cannot invoke the candidate without adding a caller-side cast.
The candidate therefore does not preserve this public macro's input-domain and
conversion contract.  Restore an interface/expansion that preserves the
upstream macro's single evaluation and integral-expression behavior, then
re-review the call-site compatibility.

## Verified portions

- The upstream header has no configuration conditionals other than its include
  guard (lines 15–16 and 386); no selected conditional definition is omitted.
- The 231 upstream definitions other than the include guard are represented by
  230 Rust constants and the one function-like macro translation; all literal
  values, aliases, bitwise compositions, RSA additions, DA830 values, OMAP
  register values, and Altera values were compared against lines 21–384.
- `OMAP1_UART{1,2,3}_BASE` correctly use `u32`, matching their unsigned C
  hexadecimal-literal type on the frozen architectures.  The remaining
  object-like macro literal/expression types and values are `i32`-compatible.
- Required SPDX and immutable source/revision/architecture/task provenance are
  present at candidate lines 1–5.  No branding delta is present.

No compilation, formatting, or tests were run.
