# Rust semantics review — S016378 attempt 2, slot 2

Reviewed only the pinned `vendor/linux/include/uapi/linux/serial_reg.h`, the
current candidate, current implementation evidence, and the sealed current
semantic proposal. No compiler, formatter, test, or Rust-analyzer diagnostics
were used.

## Finding RUST-S016378-001 — function-like macro does not preserve the C type or namespace contract

**Proposal records:**

- `SC1-e6e380a83cc395cf363413bcc69c687f58893c2b0e44099e7c19918fcff6a264`
  (`aarch64`, `UART_FCR_R_TRIG_BITS`, `selection_expression`)
- `SC1-779550ee17d23cddd624bb3443da8084ee546888b8f64a0e6b7491c8bf607de0`
  (`x86_64`, `UART_FCR_R_TRIG_BITS`, `selection_expression`)

Pinned source at `include/uapi/linux/serial_reg.h:101-103` defines
`UART_FCR_R_TRIG_BITS(x)` as `(((x) & UART_FCR_TRIGGER_MASK) >>
UART_FCR_R_TRIG_SHIFT)`.  It expands `x` once, but C performs the integer
promotions before `&` and `>>`.  A pinned caller supplies an `unsigned char`:
`drivers/tty/serial/8250/8250.h:71` declares `fcr` as `unsigned char`, and
`drivers/tty/serial/8250/8250_port.c:2978` invokes the macro with `up->fcr`.
The C expression is therefore evaluated as `int` and yields `int`.

The candidate's `UART_FCR_R_TRIG_BITS!` at
`src/include/uapi/linux/serial_reg.rs:62-66` applies `& 0xc0` directly to the
Rust argument.  For a `u8` argument, the literal is inferred as `u8` and the
result is `u8`, rather than the C-promoted `int`/`i32`; for other accepted Rust
integer argument types the result similarly follows the argument type rather
than C's usual arithmetic conversions.  This is a public-UAPI expression type
and can alter downstream indexing/arithmetic requirements even though the
numeric values 0..3 coincide.

The same candidate also marks the macro `#[macro_export]`.  That exports a
crate-root Rust macro (and requires Rust `!` invocation) rather than preserving
the lexical, header-inclusion-scoped C preprocessor name.  The upstream header
has no exported linker or crate-root symbol corresponding to this macro.

Required resolution: replace the macro surface with a source-backed mapping
that preserves the promoted result type and intended module visibility for both
architectures, or block the task if the frozen Rust UAPI mapping cannot express
the C macro contract without an unapproved ABI/API decision.  Retain the
single evaluation of the argument.

## Checked items without findings

- The candidate has no `unsafe`, panic, allocation, FFI-layout, or `Drop`
  surface.
- The object-like definitions are evaluated as explicit constants; their shown
  signed `int` expressions and the three `0xfffb...` unsigned-int base values
  have source-consistent literal values in the candidate.
- Omitting the empty C include guard (`serial_reg.h:15-16,386`) is correct for
  a Rust module: Rust does not textually re-include the module, so recreating
  `_LINUX_SERIAL_REG_H` would add an observable, non-source UAPI item.
- The immutable provenance and SPDX identifier match the pinned source and
  task identity.

**Result: FINDINGS (1).**
