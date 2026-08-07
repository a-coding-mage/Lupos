# Resolution — S016378 attempt 2

## Result

**BLOCKED.** No source change is applied.

The pinned UAPI source defines `UART_FCR_R_TRIG_BITS(x)` at
`vendor/linux/include/uapi/linux/serial_reg.h:101-102` as one textual
substitution of `x` into `(((x) & UART_FCR_TRIGGER_MASK) >>
UART_FCR_R_TRIG_SHIFT)`.  It therefore retains C integer promotions and the
usual arithmetic conversions selected by each caller's expression type.

The complete pinned-tree use set contains two uses, in
`vendor/linux/drivers/tty/serial/8250/8250_port.c:2978` and `:2988`.  The
first receives `up->fcr`, whose declared type is `unsigned char` in
`vendor/linux/drivers/tty/serial/8250/8250.h:71`; the C expression is promoted
to `int` before the `&` and produces `int`.  The second receives the
unsuffixed integer macro `UART_FCR_R_TRIG_00`.

A Rust `macro_rules!` expansion can evaluate `$x` once, but it cannot inspect
the expression type and apply C's type-dependent integer promotions/usual
arithmetic conversions for every expression accepted by the original
function-like UAPI macro.  Leaving the argument uncast gives a `u8` result for
the pinned `unsigned char` use; forcing an `i32` cast fixes that use but changes
valid C `unsigned int`, `long`, and wider-integer uses.  A generic function or
trait would replace the source macro's expression surface and/or add an
unjustified API.  The pinned source supplies no Rust module/export convention
that could justify a different public macro surface; `#[macro_export]` also
places the name at the Rust crate root rather than this Linux-shaped header
module.

Thus no faithful Rust module-path mapping of this function-like macro can be
established solely from the pinned source.  Completing the generated semantic
final/disposition records or committing their `COMPLETE` values would falsely
close the affected records, so no semantic-closure commit is made.

## Review dispositions

- `PARITY-001` (slot 1): **accepted**. The current `#[macro_export]` name is
  crate-root scoped, not a source-established header-module mapping. This is
  blocked together with the type/evaluation issue above; no source-backed
  module-path replacement is available.
- `RUST-S016378-001` (slot 2): **accepted**. The direct Rust bitwise expression
  retains the argument's Rust integer type rather than C promotion. The pinned
  `unsigned char` caller demonstrates the mismatch. No faithful generic macro
  replacement is established, so this cannot be resolved by a cast or a
  function.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or historical
translation source was used.
