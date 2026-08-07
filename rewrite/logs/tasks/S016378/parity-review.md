# Parity review — S016378, attempt 2, slot 1

Result: **FINDINGS** (one blocking source-parity finding).

Reviewed direct evidence only:

- `vendor/linux/include/uapi/linux/serial_reg.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`;
- `src/include/uapi/linux/serial_reg.rs`;
- this attempt's implementation evidence, sealed semantic-closure proposal,
  frozen queue row, header-closure evidence, and frozen x86_64/AArch64
  configurations.

The source header is selected for both frozen architectures and the task's
`common` label correctly denotes that shared scope.  The C include guard is a
preprocessor-only multiple-inclusion mechanism; its omission is correct and no
guard materialization is requested.

I compared the complete macro identifier set (excluding that guard) against the
candidate.  Every object-like UART, RSA, OMAP, and Altera name is present; the
literal values, derived bitwise/arithmetic expressions, signed `UART_RSA_*`
base offsets, and unsigned OMAP base literals agree with the pinned header.
The function-like expression retains parenthesized masking before the right
shift and evaluates its supplied expression once.  There are no configuration
conditional branches beyond the guard.

## PARITY-001 — `UART_FCR_R_TRIG_BITS` has the wrong exported namespace

`src/include/uapi/linux/serial_reg.rs:63` applies `#[macro_export]` to
`UART_FCR_R_TRIG_BITS`.  Rust exports such a macro at the crate root, not at
the Linux-shaped `include::uapi::linux::serial_reg` module represented by this
task.  The candidate provides no module re-export.  Consequently a translated
consumer of this header cannot access the UAPI macro through the header path,
whereas both pinned C uses get it from the included header:
`drivers/tty/serial/8250/8250_port.c:2978` and `:2988`.

Resolve by making the macro available at the serial-reg module path (while
preserving its exact name, one evaluation, and
`(((x) & UART_FCR_TRIGGER_MASK) >> UART_FCR_R_TRIG_SHIFT)` precedence).  Do
not address this by materializing `_LINUX_SERIAL_REG_H`.

Affected semantic proposal records:

- `SC1-e6e380a83cc395cf363413bcc69c687f58893c2b0e44099e7c19918fcff6a264`
  (`aarch64`, `UART_FCR_R_TRIG_BITS`, `selection_expression`)
- `SC1-3e78fac723d8342b81e7273bcb31e3e182fcb078a46bcd3df405872a461e0758`
  (`aarch64`, `UART_FCR_R_TRIG_BITS`, `status`)
- `SC1-779550ee17d23cddd624bb3443da8084ee546888b8f64a0e6b7491c8bf607de0`
  (`x86_64`, `UART_FCR_R_TRIG_BITS`, `selection_expression`)
- `SC1-14635f26e2b3313c7c3aab74078b2e6fe71d872f5817f438d6fdaf39e2d98c67`
  (`x86_64`, `UART_FCR_R_TRIG_BITS`, `status`)

No compiler, formatter, linker, test, rust-analyzer diagnostic, or historical
translation source was used.
